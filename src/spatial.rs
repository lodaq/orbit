use crate::config::{Config, Item, Page};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Rect {
    pub row: usize,
    pub col: usize,
    pub width: usize,
    pub height: usize,
}
impl Rect {
    pub fn cell(slot: usize, columns: usize) -> Self {
        Self {
            row: slot / columns,
            col: slot % columns,
            width: 1,
            height: 1,
        }
    }
    pub fn fits(self, columns: usize, rows: usize) -> bool {
        self.width > 0
            && self.height > 0
            && self.col.saturating_add(self.width) <= columns
            && self.row.saturating_add(self.height) <= rows
    }
    pub fn overlaps(self, other: Self) -> bool {
        self.col < other.col + other.width
            && other.col < self.col + self.width
            && self.row < other.row + other.height
            && other.row < self.row + self.height
    }
}

// Both the preview and commit call this function against the unchanged source.
pub fn place(
    occupied: &BTreeMap<String, Rect>,
    id: &str,
    target: Rect,
    columns: usize,
    rows: usize,
) -> Option<BTreeMap<String, Rect>> {
    if !target.fits(columns, rows) {
        return None;
    }
    let mut result = occupied.clone();
    result.remove(id);
    if !result.values().any(|rect| rect.overlaps(target)) {
        result.insert(id.into(), target);
        return Some(result);
    }
    if target.width != 1 || target.height != 1 {
        return None;
    }
    let start = target.row * columns + target.col;
    for forward in [true, false] {
        let mut trial = result.clone();
        let mut chain = Vec::new();
        let mut slot = start;
        loop {
            let cell = Rect::cell(slot, columns);
            let occupant = trial.iter().find(|(_, rect)| rect.overlaps(cell));
            match occupant {
                None => {
                    for (id, from) in chain.into_iter().rev() {
                        trial.insert(
                            id,
                            Rect::cell(if forward { from + 1 } else { from - 1 }, columns),
                        );
                    }
                    trial.insert(id.into(), target);
                    return Some(trial);
                }
                Some((id, rect)) if rect.width == 1 && rect.height == 1 => {
                    chain.push((id.clone(), slot))
                }
                _ => break,
            }
            if forward {
                slot += 1;
                if slot >= columns * rows {
                    break;
                }
            } else if slot == 0 {
                break;
            } else {
                slot -= 1;
            }
        }
    }
    None
}

pub fn occupied(config: &Config, page: usize) -> BTreeMap<String, Rect> {
    config
        .pages
        .get(page)
        .into_iter()
        .flat_map(|page| &page.items)
        .filter(|item| !config.pinned.iter().any(|id| id == item.id()))
        .filter_map(|item| {
            config
                .positions
                .get(item.id())
                .map(|rect| (item.id().into(), *rect))
        })
        .collect()
}

fn free(
    occupied: &BTreeMap<String, Rect>,
    mut rect: Rect,
    columns: usize,
    rows: usize,
) -> Option<Rect> {
    let origin = (rect.row, rect.col);
    (0..columns * rows)
        .filter_map(|slot| {
            rect.row = slot / columns;
            rect.col = slot % columns;
            (rect.fits(columns, rows) && !occupied.values().any(|other| rect.overlaps(*other)))
                .then_some(rect)
        })
        .min_by_key(|rect| {
            (
                rect.row.abs_diff(origin.0) + rect.col.abs_diff(origin.1),
                rect.row,
                rect.col,
            )
        })
}

// Keep valid coordinates first; displaced items follow page, row, column, ID order.
// Page vectors remain the item registry; pinned IDs have exclusive dock ownership.
pub fn reconcile(config: &mut Config) -> Result<(), String> {
    // Legacy pins could refer to folder children. Promote those instances once.
    for page in &mut config.pages {
        let mut promoted = Vec::new();
        for item in &mut page.items {
            if let Item::Folder { apps, .. } = item {
                let mut index = 0;
                while index < apps.len() {
                    if config.pinned.contains(&apps[index].id) {
                        promoted.push(Item::App(apps.remove(index)));
                    } else {
                        index += 1;
                    }
                }
            }
        }
        page.items.extend(promoted);
    }
    let ids: HashSet<_> = config
        .pages
        .iter()
        .flat_map(|page| page.items.iter().map(|item| item.id().to_owned()))
        .collect();
    config
        .positions
        .retain(|id, _| ids.contains(id) && !config.pinned.contains(id));
    let mut seen = HashSet::new();
    config
        .pinned
        .retain(|id| ids.contains(id) && seen.insert(id.clone()));
    let mut pending = Vec::new();
    let mut occupancy = vec![BTreeMap::new(); config.pages.len()];
    for (page_index, page) in config.pages.iter_mut().enumerate() {
        let items = std::mem::take(&mut page.items);
        for (index, item) in items.into_iter().enumerate() {
            if config.pinned.contains(&item.id().to_owned()) {
                page.items.push(item);
                continue;
            }
            let saved = config.positions.get(item.id()).copied();
            if let Some(rect) = saved.filter(|rect| {
                rect.fits(config.columns, config.rows)
                    && !occupancy[page_index]
                        .values()
                        .any(|other| rect.overlaps(*other))
            }) {
                occupancy[page_index].insert(item.id().to_owned(), rect);
                page.items.push(item);
            } else {
                let rect = saved.unwrap_or(Rect {
                    width: if matches!(item, Item::Widget { .. }) {
                        2
                    } else {
                        1
                    },
                    height: 1,
                    ..Rect::cell(index, config.columns)
                });
                pending.push((page_index, rect, item));
            }
        }
    }
    pending
        .sort_by(|a, b| (a.0, a.1.row, a.1.col, a.2.id()).cmp(&(b.0, b.1.row, b.1.col, b.2.id())));
    for (origin, mut rect, item) in pending {
        rect.width = rect.width.max(1).min(config.columns);
        rect.height = rect.height.max(1).min(config.rows);
        let mut page = origin;
        loop {
            if page == config.pages.len() {
                if page >= 9 {
                    return Err("Layout is full. Choose a smaller Grid Cell Size.".into());
                }
                config.pages.push(Page {
                    id: crate::config::new_page_id(),
                    items: Vec::new(),
                });
                occupancy.push(BTreeMap::new());
            }
            if let Some(rect) = free(&occupancy[page], rect, config.columns, config.rows) {
                occupancy[page].insert(item.id().into(), rect);
                config.positions.insert(item.id().into(), rect);
                config.pages[page].items.push(item);
                break;
            }
            page += 1;
        }
    }
    config.active_page = config.active_page.min(config.pages.len() - 1);
    Ok(())
}

// Coordinates are relative to the icon, not its surrounding card or label.
pub fn center(x: f32, y: f32, size: f32, held: bool) -> bool {
    let half = size * if held { 0.31 } else { 0.275 };
    (x - size / 2.).abs() <= half && (y - size / 2.).abs() <= half
}

pub const WIDGETS: [&str; 5] = [
    "Random Quote",
    "Random Fact",
    "Random Word",
    "Motivation",
    "Random Number",
];
pub fn widget_text(variant: usize, sample: u128) -> String {
    let snippets = match variant {
        0 => &[
            "Make room for curiosity.",
            "Small steps carry you far.",
            "A fresh page invites a new idea.",
        ][..],
        1 => &[
            "A triangle has three sides.",
            "An octagon has eight sides.",
            "A prime number has exactly two positive divisors.",
        ][..],
        2 => &[
            "Orbit — a path around a center.",
            "Serene — calm and peaceful.",
            "Discover — find something new.",
        ][..],
        3 => &[
            "Take the next small step.",
            "Give your idea a little time.",
            "Progress begins with practice.",
        ][..],
        _ => return format!("Your random number: {}", sample % 1000),
    };
    snippets[sample as usize % snippets.len()].into()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn slots(values: &[(&str, usize)]) -> BTreeMap<String, Rect> {
        values
            .iter()
            .map(|(id, slot)| (id.to_string(), Rect::cell(*slot, 4)))
            .collect()
    }
    #[test]
    fn holes_and_forward_backward() {
        let initial = slots(&[("a", 0), ("b", 1), ("c", 3)]);
        let moved = place(&initial, "b", Rect::cell(2, 4), 4, 1).unwrap();
        assert_eq!(moved, slots(&[("a", 0), ("b", 2), ("c", 3)]));
        let moved = place(&initial, "c", Rect::cell(0, 4), 4, 1).unwrap();
        assert_eq!(moved, slots(&[("c", 0), ("a", 1), ("b", 2)]));
        let moved = place(&initial, "a", Rect::cell(3, 4), 4, 1).unwrap();
        assert_eq!(moved, slots(&[("a", 3), ("b", 1), ("c", 2)]));
    }
    #[test]
    fn geometry_zones_and_hysteresis() {
        for size in [40., 50., 80., 110.] {
            assert!(center(size / 2., size / 2., size, false));
            for x in [0., size * 0.1, size * 0.9, size] {
                assert!(!center(x, size / 2., size, false));
            }
            assert!(!center(size * 0.2, size / 2., size, false));
            assert!(center(size * 0.2, size / 2., size, true));
        }
    }
    #[test]
    fn randomized_moves_preserve_invariants_and_preview_purity() {
        let mut state = slots(&[("a", 0), ("b", 1), ("c", 3)]);
        for n in 0..10000 {
            let id = ["a", "b", "c"][n % 3];
            let target = Rect::cell((n * 17 + n / 7) % 12, 4);
            let preview = place(&state, id, target, 4, 3);
            let _ = place(&state, id, Rect::cell(5, 4), 4, 3);
            assert_eq!(preview, place(&state, id, target, 4, 3));
            if let Some(next) = preview {
                state = next;
            }
            assert_eq!(state.len(), 3);
            for (id, rect) in &state {
                assert!(rect.fits(4, 3));
                assert!(
                    state
                        .iter()
                        .all(|(other, r)| id == other || !rect.overlaps(*r))
                );
            }
        }
    }
    #[test]
    fn widget_footprints_reject_collisions() {
        let mut state = slots(&[("a", 0)]);
        state.insert(
            "w".into(),
            Rect {
                row: 1,
                col: 1,
                width: 2,
                height: 2,
            },
        );
        assert!(place(&state, "a", Rect::cell(5, 4), 4, 3).is_none());
        assert!(
            place(
                &state,
                "w",
                Rect {
                    row: 0,
                    col: 0,
                    width: 2,
                    height: 2
                },
                4,
                3
            )
            .is_none()
        );
    }
}

pub fn connect(launcher: &crate::Launcher) {
    use slint::{Model, ModelRc, VecModel};
    launcher.on_next_grid_item(|items, index, direction| {
        let Some(current) = items.row_data(index.max(0) as usize) else {
            return 0;
        };
        items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                !item.pinned
                    && match direction {
                        0 => item.col < current.col,
                        1 => item.col > current.col,
                        2 => item.row < current.row,
                        _ => item.row > current.row,
                    }
            })
            .min_by_key(|(_, item)| {
                if direction < 2 {
                    (
                        item.row.abs_diff(current.row),
                        item.col.abs_diff(current.col),
                    )
                } else {
                    (
                        item.col.abs_diff(current.col),
                        item.row.abs_diff(current.row),
                    )
                }
            })
            .map_or(index, |(index, _)| index as i32)
    });
    launcher.on_center_zone(center);
    launcher.on_hit_cell(|items, col, row| {
        items
            .iter()
            .position(|item| {
                !item.pinned
                    && col >= item.col
                    && col < item.col + item.span_x
                    && row >= item.row
                    && row < item.row + item.span_y
            })
            .map_or(-1, |index| index as i32)
    });
    launcher.on_free_cell(|items, columns, rows| {
        let occupied = tile_positions(&items);
        free(
            &occupied,
            Rect::cell(0, columns.max(1) as usize),
            columns.max(1) as usize,
            rows.max(1) as usize,
        )
        .map_or(-1, |rect| (rect.row * columns as usize + rect.col) as i32)
    });
    launcher.on_preview_valid(|items, id, slot, width, height, columns, rows| {
        tile_preview(&items, &id, slot, width, height, columns, rows).is_some()
    });
    launcher.on_preview_cells(|items, id, slot, width, height, columns, rows| {
        let Some(positions) = tile_preview(&items, &id, slot, width, height, columns, rows) else {
            return items;
        };
        ModelRc::new(VecModel::from(
            items
                .iter()
                .map(|mut item| {
                    if let Some(rect) = positions.get(item.id.as_str()) {
                        item.col = rect.col as i32;
                        item.row = rect.row as i32;
                    }
                    item
                })
                .collect::<Vec<_>>(),
        ))
    });
}
fn tile_positions(items: &slint::ModelRc<crate::AppTile>) -> BTreeMap<String, Rect> {
    use slint::Model;
    items
        .iter()
        .filter(|item| !item.pinned)
        .map(|item| {
            (
                item.id.to_string(),
                Rect {
                    row: item.row as usize,
                    col: item.col as usize,
                    width: item.span_x as usize,
                    height: item.span_y as usize,
                },
            )
        })
        .collect()
}
#[allow(clippy::too_many_arguments)]
fn tile_preview(
    items: &slint::ModelRc<crate::AppTile>,
    id: &str,
    slot: i32,
    width: i32,
    height: i32,
    columns: i32,
    rows: i32,
) -> Option<BTreeMap<String, Rect>> {
    if slot < 0 || columns <= 0 || rows <= 0 {
        return None;
    }
    let rect = Rect {
        width: width.max(1) as usize,
        height: height.max(1) as usize,
        ..Rect::cell(slot as usize, columns as usize)
    };
    place(
        &tile_positions(items),
        id,
        rect,
        columns as usize,
        rows as usize,
    )
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use crate::config::{App, Icon, Launch};
    fn app(id: &str) -> Item {
        Item::App(App {
            id: id.into(),
            name: id.into(),
            icon: Icon::Letter,
            launch: Launch::Command("true".into()),
        })
    }
    #[test]
    fn dock_ownership_reorder_capacity_and_widget_rejection() {
        let mut config = Config::default();
        config.pages[0].items = (0..9).map(|id| app(&id.to_string())).collect();
        reconcile(&mut config).unwrap();
        for id in 0..8 {
            assert!(crate::item_drop::apply(
                &mut config,
                &id.to_string(),
                0,
                id,
                "@dock",
                "f".into()
            ));
        }
        let saved = serde_json::to_string(&config).unwrap();
        assert!(!crate::item_drop::apply(
            &mut config,
            "8",
            0,
            8,
            "@dock",
            "f".into()
        ));
        assert_eq!(saved, serde_json::to_string(&config).unwrap());
        assert!(crate::item_drop::apply(
            &mut config,
            "0",
            0,
            7,
            "@dock",
            "f".into()
        ));
        assert_eq!(config.pinned.last().unwrap(), "0");
        assert!(!occupied(&config, 0).contains_key("0"));
        assert!(!config.positions.contains_key("0"));
        assert!(crate::item_drop::apply(
            &mut config,
            "0",
            0,
            4,
            "",
            "f".into()
        ));
        assert!(!config.pinned.contains(&"0".into()));
        assert_eq!(config.positions["0"].col, 4);
    }
    #[test]
    fn reflow_and_serialization_preserve_every_instance_and_widget_text() {
        let mut config = Config::default();
        config.pages[0].items = (0..12).map(|id| app(&id.to_string())).collect();
        config.pages[0].items.push(Item::Widget {
            id: "widget".into(),
            name: WIDGETS[0].into(),
            variant: 0,
            text: widget_text(0, 1),
        });
        reconcile(&mut config).unwrap();
        config.columns = 3;
        config.rows = 2;
        reconcile(&mut config).unwrap();
        assert_eq!(
            config
                .pages
                .iter()
                .map(|page| page.items.len())
                .sum::<usize>(),
            13
        );
        let saved = serde_json::to_vec(&config).unwrap();
        let mut restored: Config = serde_json::from_slice(&saved).unwrap();
        reconcile(&mut restored).unwrap();
        assert_eq!(saved, serde_json::to_vec(&restored).unwrap());
        for page in 0..restored.pages.len() {
            let occupied = occupied(&restored, page);
            for (id, rect) in &occupied {
                assert!(rect.fits(3, 2));
                assert!(
                    occupied
                        .iter()
                        .all(|(other, r)| id == other || !rect.overlaps(*r))
                );
            }
        }
    }
}
