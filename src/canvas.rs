use crate::config::Config;
use serde::{Deserialize, Serialize};

pub const WIDTH: f32 = 280.0;
pub const HEIGHT: f32 = 180.0;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct View {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl Default for View {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

impl View {
    pub fn world(self, x: f32, y: f32) -> (f32, f32) {
        ((x - self.x) / self.zoom, (y - self.y) / self.zoom)
    }

    pub fn zoom_at(self, x: f32, y: f32, factor: f32) -> Self {
        let (wx, wy) = self.world(x, y);
        let zoom = (self.zoom * factor).clamp(0.25, 2.0);
        Self {
            x: x - wx * zoom,
            y: y - wy * zoom,
            zoom,
        }
    }

    fn valid(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && (0.25..=2.0).contains(&self.zoom)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CanvasBox {
    pub id: String,
    pub name: String,
    pub description: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Canvas {
    pub enabled: bool,
    pub boxes: Vec<CanvasBox>,
    pub view: View,
}

impl Canvas {
    pub fn valid(&self) -> bool {
        let mut ids = std::collections::HashSet::new();
        self.view.valid()
            && self.boxes.iter().all(|b| {
                !b.id.is_empty()
                    && ids.insert(&b.id)
                    && !b.name.trim().is_empty()
                    && b.x.is_finite()
                    && b.y.is_finite()
            })
    }

    pub fn add(
        &mut self,
        id: String,
        name: String,
        description: String,
        x: f32,
        y: f32,
    ) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Enter a name".into());
        }
        if id.is_empty()
            || self.boxes.iter().any(|b| b.id == id)
            || !x.is_finite()
            || !y.is_finite()
        {
            return Err("Invalid box position".into());
        }
        self.boxes.push(CanvasBox {
            id,
            name: name.trim().into(),
            description,
            x,
            y,
        });
        Ok(())
    }
}

// Preflight on a copy: a full horizontal layout must never lose the old top page.
pub fn prepare(config: &mut Config) -> Result<bool, String> {
    if config.canvas.enabled {
        return Ok(false);
    }
    let mut next = config.clone();
    if let Some(top) = next.pages.iter().position(|page| page.vertical == -1) {
        let items = std::mem::take(&mut next.pages[top].items);
        next.pages[0].items.extend(items);
        crate::spatial::reconcile(&mut next)
            .map_err(|_| "Canvas needs space for the top page's items. Free space on regular pages and try again.".to_string())?;
    }
    next.canvas.enabled = true;
    *config = next;
    Ok(true)
}

pub fn is_page(config: &Config, page: usize) -> bool {
    config.canvas.enabled
        && config
            .pages
            .get(page)
            .is_some_and(|page| page.vertical == -1)
}

pub enum Mutation {
    Add(String, String, f32, f32),
    Move(String, f32, f32),
    View(View),
}

pub fn apply(config: &mut Config, change: Mutation) -> Result<(), String> {
    if !config.canvas.enabled {
        return Err("Canvas is unavailable".into());
    }
    let mut next = config.clone();
    match change {
        Mutation::Add(name, description, x, y) => {
            if config.layout_locked {
                return Err("Unlock layout to make changes".into());
            }
            next.canvas
                .add(crate::apps::new_id()?, name, description, x, y)?;
        }
        Mutation::Move(id, x, y) => {
            if config.layout_locked {
                return Err("Unlock layout to make changes".into());
            }
            let item = next
                .canvas
                .boxes
                .iter_mut()
                .find(|b| b.id == id)
                .ok_or("Box is missing")?;
            item.x = x;
            item.y = y;
        }
        Mutation::View(view) => next.canvas.view = view,
    }
    if !next.canvas.valid() {
        return Err("Invalid canvas position".into());
    }
    if next.canvas != config.canvas {
        next.save()
            .map_err(|error| format!("Couldn't save canvas: {error}"))?;
        *config = next;
    }
    Ok(())
}

pub fn sync(launcher: &crate::Launcher, canvas: &Canvas, restore: bool) {
    use slint::ComponentHandle;
    let state = launcher.global::<crate::CanvasState>();
    state.set_enabled(canvas.enabled);
    state.set_boxes(
        std::rc::Rc::new(slint::VecModel::from(
            canvas
                .boxes
                .iter()
                .map(|b| crate::CanvasBox {
                    id: b.id.as_str().into(),
                    name: b.name.as_str().into(),
                    description: b.description.as_str().into(),
                    x: b.x,
                    y: b.y,
                })
                .collect::<Vec<_>>(),
        ))
        .into(),
    );
    if restore || !state.get_initialized() {
        state.set_pan_x(canvas.view.x);
        state.set_pan_y(canvas.view.y);
        state.set_zoom(canvas.view.zoom);
        state.set_initialized(true);
    }
}

pub fn connect(launcher: &crate::Launcher, send: impl Fn(Mutation) + Clone + 'static) {
    use slint::ComponentHandle;
    let state = launcher.global::<crate::CanvasState>();
    state.on_valid_name(|name| !name.trim().is_empty());
    let sender = send.clone();
    let weak = launcher.as_weak();
    state.on_add(move |name, description, x, y| {
        if let Some(ui) = weak.upgrade() {
            let state = ui.global::<crate::CanvasState>();
            let view = View {
                x: state.get_pan_x(),
                y: state.get_pan_y(),
                zoom: state.get_zoom(),
            };
            let (x, y) = view.world(x, y);
            sender(Mutation::Add(
                name.into(),
                description.into(),
                x - WIDTH / 2.0,
                y - HEIGHT / 2.0,
            ));
        }
    });
    let sender = send.clone();
    state.on_move(move |id, x, y| sender(Mutation::Move(id.into(), x, y)));
    state.on_save_view(move |x, y, zoom| send(Mutation::View(View { x, y, zoom })));
    let weak = launcher.as_weak();
    state.on_zoom_at(move |x, y, factor| {
        if let Some(ui) = weak.upgrade() {
            let state = ui.global::<crate::CanvasState>();
            let view = View {
                x: state.get_pan_x(),
                y: state.get_pan_y(),
                zoom: state.get_zoom(),
            }
            .zoom_at(x, y, factor);
            if view.valid() {
                state.set_pan_x(view.x);
                state.set_pan_y(view.y);
                state.set_zoom(view.zoom);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_keeps_the_pointer_world_position_when_zooming() {
        let view = View {
            x: -120.0,
            y: 70.0,
            zoom: 0.5,
        };
        let before = view.world(310.0, 220.0);
        let after = view.zoom_at(310.0, 220.0, 1.5);
        assert_eq!(before, after.world(310.0, 220.0));
        assert_eq!(view.zoom_at(0.0, 0.0, 20.0).zoom, 2.0);
        assert_eq!(view.zoom_at(0.0, 0.0, 0.01).zoom, 0.25);
    }

    #[test]
    fn boxes_and_view_round_trip_and_old_configs_start_empty() {
        let mut config = crate::config::Config::default();
        config.canvas.view = View {
            x: -45.0,
            y: 80.0,
            zoom: 1.5,
        };
        config
            .canvas
            .add(
                "id".into(),
                "  فكرة  ".into(),
                "Description\nوصف".into(),
                -20.0,
                30.0,
            )
            .unwrap();
        let value = serde_json::to_value(&config).unwrap();
        let copy: crate::config::Config = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(copy.canvas, config.canvas);
        let mut old = value;
        old.as_object_mut().unwrap().remove("canvas");
        let old: crate::config::Config = serde_json::from_value(old).unwrap();
        assert!(old.canvas.boxes.is_empty());
        assert_eq!(old.canvas.view.zoom, 1.0);
        assert!(
            config
                .canvas
                .add("empty".into(), "  ".into(), "".into(), 0.0, 0.0)
                .is_err()
        );
        assert_eq!(config.canvas.boxes[0].name, "فكرة");
    }

    #[test]
    fn top_page_migration_preserves_items_and_pins() {
        let mut config = crate::config::Config::default();
        config.ensure_vertical_pages();
        let item = crate::config::Item::Widget {
            id: "widget".into(),
            name: "Facts".into(),
            text: "saved".into(),
            variant: 9,
            ticktick_project_id: "".into(),
            min_number: 0,
            max_number: 10,
        };
        config.pages[1].items.push(item.clone());
        config.pages[1].items.push(crate::config::Item::Website {
            id: "pin".into(),
            name: "Pinned".into(),
            url: "https://example.com".into(),
            icon: crate::config::Icon::Letter,
        });
        config.pinned.push("pin".into());
        prepare(&mut config).unwrap();
        assert!(config.canvas.enabled);
        assert!(config.pages[config.horizontal_pages()].items.is_empty());
        assert!(
            config.pages[..config.horizontal_pages()]
                .iter()
                .flat_map(|p| &p.items)
                .any(|i| i == &item)
        );
        assert_eq!(config.pinned, ["pin"]);
    }

    #[test]
    fn locked_layout_rejects_box_changes_and_invalid_views_without_writing() {
        let mut config = crate::config::Config::default();
        config.canvas.enabled = true;
        config.layout_locked = true;
        let before = serde_json::to_value(&config).unwrap();
        assert!(
            apply(
                &mut config,
                Mutation::Add("Box".into(), "".into(), 0.0, 0.0)
            )
            .is_err()
        );
        assert!(apply(&mut config, Mutation::Move("missing".into(), 1.0, 2.0)).is_err());
        assert!(
            apply(
                &mut config,
                Mutation::View(View {
                    x: f32::NAN,
                    y: 0.0,
                    zoom: 1.0
                })
            )
            .is_err()
        );
        assert!(apply(&mut config, Mutation::View(View::default())).is_ok());
        assert_eq!(serde_json::to_value(&config).unwrap(), before);
    }

    #[test]
    fn launcher_items_cannot_be_dropped_on_the_canvas() {
        let mut config = crate::config::Config::default();
        config.ensure_vertical_pages();
        prepare(&mut config).unwrap();
        let item = crate::config::Item::Website {
            id: "website".into(),
            name: "Site".into(),
            url: "https://example.com".into(),
            icon: crate::config::Icon::Letter,
        };
        config.pages[0].items.push(item.clone());
        let before = serde_json::to_value(&config).unwrap();
        assert!(crate::item_drop::place_search_result(&mut config, item, 1, -1, false).is_err());
        assert!(!crate::item_drop::apply(
            &mut config,
            "website",
            1,
            0,
            "",
            "unused".into()
        ));
        assert_eq!(serde_json::to_value(&config).unwrap(), before);
    }

    #[test]
    fn full_pages_leave_the_config_unchanged() {
        let mut config = crate::config::Config::default();
        config.columns = 1;
        config.rows = 1;
        for _ in 1..9 {
            config.insert_page();
        }
        config.ensure_vertical_pages();
        for index in 0..10 {
            config.pages[index]
                .items
                .push(crate::config::Item::Website {
                    id: format!("item-{index}"),
                    name: "Item".into(),
                    url: "https://example.com".into(),
                    icon: crate::config::Icon::Letter,
                });
        }
        let before = serde_json::to_value(&config).unwrap();
        assert!(prepare(&mut config).is_err());
        assert_eq!(serde_json::to_value(&config).unwrap(), before);
    }
}
