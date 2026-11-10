use crate::config::{CanvasFrame, Config, InkStroke, Item};
use crate::spatial::Rect;
use std::collections::BTreeMap;

#[derive(Clone, PartialEq)]
pub struct State {
    items: Vec<Item>,
    positions: BTreeMap<String, Rect>,
    pinned: Vec<String>,
    frames: Vec<CanvasFrame>,
    ink: Vec<InkStroke>,
    item_size: i32,
    grid_gap: i32,
}

impl State {
    pub fn capture(config: &Config) -> Self {
        Self {
            items: config.items.clone(),
            positions: config.positions.clone(),
            pinned: config.pinned.clone(),
            frames: config.frames.clone(),
            ink: config.ink.clone(),
            item_size: config.item_size,
            grid_gap: config.grid_gap,
        }
    }

    pub fn restore(&self, config: &mut Config) {
        config.items.clone_from(&self.items);
        config.positions.clone_from(&self.positions);
        config.pinned.clone_from(&self.pinned);
        config.frames.clone_from(&self.frames);
        config.ink.clone_from(&self.ink);
        config.item_size = self.item_size;
        config.grid_gap = self.grid_gap;
    }
}

#[derive(Default)]
pub struct History {
    past: Vec<State>,
    future: Vec<State>,
    pub clipboard: Option<Clipboard>,
}

impl History {
    pub fn record(&mut self, before: State, config: &Config) {
        if before.items != config.items
            || before.positions != config.positions
            || before.pinned != config.pinned
            || before.frames != config.frames
            || before.ink != config.ink
            || before.item_size != config.item_size
            || before.grid_gap != config.grid_gap
        {
            if self.past.len() == 20 {
                self.past.remove(0);
            }
            self.past.push(before);
            self.future.clear();
        }
    }

    pub fn target(&self, redo: bool) -> Option<&State> {
        if redo { &self.future } else { &self.past }.last()
    }

    // Advance only after the restored configuration has been saved successfully.
    pub fn finish(&mut self, current: State, redo: bool) {
        let (source, destination) = if redo {
            (&mut self.future, &mut self.past)
        } else {
            (&mut self.past, &mut self.future)
        };
        if source.pop().is_some() {
            destination.push(current);
        }
    }
}

pub struct Clipboard {
    items: Vec<(Item, Rect)>,
    cut: bool,
}

impl Clipboard {
    pub fn capture(config: &Config, selection: &[String], cut: bool) -> Option<Self> {
        let items: Vec<_> = config
            .items
            .iter()
            .filter_map(|item| {
                if !selection.iter().any(|id| id == item.id()) {
                    return None;
                }
                config
                    .positions
                    .get(item.id())
                    .map(|rect| (item.clone(), *rect))
            })
            .collect();
        (!items.is_empty()).then_some(Self { items, cut })
    }

    pub fn cut(&self, config: &mut Config) {
        config
            .items
            .retain(|item| !self.items.iter().any(|(cut, _)| cut.id() == item.id()));
    }

    pub fn paste(&self, config: &mut Config, x: i32, y: i32) -> Result<Vec<String>, String> {
        let left = self.items.iter().map(|(_, r)| r.col).min().unwrap_or(0);
        let top = self.items.iter().map(|(_, r)| r.row).min().unwrap_or(0);
        let right = self
            .items
            .iter()
            .map(|(_, r)| i64::from(r.col) + r.width as i64)
            .max()
            .unwrap_or(0);
        let bottom = self
            .items
            .iter()
            .map(|(_, r)| i64::from(r.row) + r.height as i64)
            .max()
            .unwrap_or(0);
        let anchor = crate::spatial::nearby(
            &crate::spatial::occupied(config),
            Rect {
                col: x,
                row: y,
                width: (right - i64::from(left)) as usize,
                height: (bottom - i64::from(top)) as usize,
            },
        )?;
        let preserve = self.cut
            && self.items.iter().all(|(item, _)| {
                !config
                    .items
                    .iter()
                    .any(|existing| existing.id() == item.id())
            });
        let mut selected = Vec::new();
        for (item, rect) in &self.items {
            let mut item = item.clone();
            if !preserve {
                replace_id(&mut item)?;
            }
            let id = item.id().to_owned();
            let col = i32::try_from(i64::from(anchor.col) + i64::from(rect.col) - i64::from(left))
                .map_err(|_| "Canvas position is too far away")?;
            let row = i32::try_from(i64::from(anchor.row) + i64::from(rect.row) - i64::from(top))
                .map_err(|_| "Canvas position is too far away")?;
            config
                .positions
                .insert(id.clone(), Rect { col, row, ..*rect });
            config.items.push(item);
            selected.push(id);
        }
        Ok(selected)
    }

    pub fn pasted(&mut self) {
        self.cut = false;
    }
}

fn replace_id(item: &mut Item) -> Result<(), String> {
    let id = match item {
        Item::App(app) => &mut app.id,
        Item::Annotation { id, .. }
        | Item::Robot { id, .. }
        | Item::Widget { id, .. }
        | Item::Website { id, .. }
        | Item::File { id, .. }
        | Item::Directory { id, .. } => id,
        Item::Folder { id, apps, .. } => {
            for app in apps {
                app.id = crate::apps::new_id()?;
            }
            id
        }
    };
    *id = crate::apps::new_id()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        let mut config = Config::default();
        config.items.push(Item::File {
            id: "a".into(),
            name: "A".into(),
            path: "/tmp/a".into(),
            icon: Default::default(),
        });
        config.positions.insert(
            "a".into(),
            Rect {
                col: -20,
                row: 30,
                width: 144,
                height: 144,
            },
        );
        config
    }

    #[test]
    fn history_ignores_views_preserves_settings_and_clears_redo_after_an_edit() {
        let mut config = config();
        let original = State::capture(&config);
        let mut history = History::default();
        config.canvas.view.x = 100.0;
        history.record(original.clone(), &config);
        assert!(history.target(false).is_none());
        config.positions.get_mut("a").unwrap().col = 500;
        history.record(original, &config);
        config.canvas_speed = 9;
        let current = State::capture(&config);
        history.target(false).unwrap().restore(&mut config);
        history.finish(current, false);
        assert_eq!(config.positions["a"].col, -20);
        assert_eq!(config.canvas_speed, 9);
        assert_eq!(config.canvas.view.x, 100.0);
        assert!(history.target(true).is_some());
        let before = State::capture(&config);
        config.positions.get_mut("a").unwrap().row = 400;
        history.record(before, &config);
        assert!(history.target(true).is_none());
    }

    #[test]
    fn copy_has_fresh_ids_and_cut_preserves_ids_once_without_overlapping() {
        let mut config = config();
        let selection = vec!["a".into()];
        let copy = Clipboard::capture(&config, &selection, false).unwrap();
        let pasted = copy.paste(&mut config, -20, 30).unwrap();
        assert_ne!(pasted[0], "a");
        assert!(!config.positions["a"].overlaps(config.positions[&pasted[0]]));
        let mut cut = Clipboard::capture(&config, &selection, true).unwrap();
        cut.cut(&mut config);
        crate::spatial::reconcile(&mut config).unwrap();
        assert_eq!(cut.paste(&mut config, 800, 300).unwrap(), selection);
        cut.pasted();
        assert_ne!(cut.paste(&mut config, 800, 300).unwrap()[0], "a");
        assert!(config.valid());
    }
}
