use crate::config::{Config, Item, Page};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

const ITEM_GAP: usize = 16;

pub fn cell_pitch(item_size: i32) -> usize {
    item_size.max(1) as usize + ITEM_GAP
}

pub fn item_span(units: usize, item_size: i32) -> usize {
    units * cell_pitch(item_size)
}

pub fn widget_span(units: usize, item_size: i32) -> usize {
    units * item_size.max(1) as usize + ITEM_GAP
}

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
    if result.values().any(|rect| rect.overlaps(target)) {
        return None;
    }
    result.insert(id.into(), target);
    Some(result)
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
    if rect.width > columns || rect.height > rows {
        return None;
    }
    let origin = (rect.row, rect.col);
    let mut best = None;
    let mut best_distance = usize::MAX;
    let max_col = columns.saturating_sub(rect.width);
    for row in 0..=rows - rect.height {
        let vertical_distance = row.abs_diff(origin.0);
        if vertical_distance > best_distance {
            continue;
        }
        for forward in [false, true] {
            let mut col = origin.1.min(max_col);
            loop {
                rect.row = row;
                rect.col = col;
                if let Some(other) = occupied.values().find(|other| rect.overlaps(**other)) {
                    let next = if forward {
                        col.max(other.col.saturating_add(other.width))
                    } else {
                        other.col.saturating_sub(rect.width)
                    };
                    if (forward && next <= col) || (!forward && next >= col) {
                        break;
                    }
                    col = next;
                    if col > max_col {
                        break;
                    }
                    continue;
                }
                let distance = vertical_distance + col.abs_diff(origin.1);
                if distance < best_distance
                    || (distance == best_distance
                        && best.is_none_or(|current: Rect| (row, col) < (current.row, current.col)))
                {
                    best_distance = distance;
                    best = Some(rect);
                }
                break;
            }
        }
    }
    best
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
                let (width, height) = if let Item::Widget { variant, .. } = &item {
                    WIDGET_SPANS[*variant]
                } else {
                    (1, 1)
                };
                let rect = saved.unwrap_or(Rect {
                    width: widget_span(width, config.item_size),
                    height: widget_span(height, config.item_size),
                    ..Rect::cell(index, config.columns)
                });
                pending.push((page_index, rect, item));
            }
        }
    }
    pending
        .sort_by(|a, b| (a.0, a.1.row, a.1.col, a.2.id()).cmp(&(b.0, b.1.row, b.1.col, b.2.id())));
    for (origin, mut rect, item) in pending {
        if let Item::Widget { variant, .. } = &item {
            let (min_width, min_height) = WIDGET_MIN_SPANS[*variant];
            rect.width = rect.width.max(widget_span(min_width, config.item_size));
            rect.height = rect.height.max(widget_span(min_height, config.item_size));
            if rect.width > config.columns || rect.height > config.rows {
                return Err("A widget does not fit at this Grid Cell Size.".into());
            }
        } else {
            rect.width = rect.width.max(1).min(config.columns);
            rect.height = rect.height.max(1).min(config.rows);
        }
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

// Use the visible tile bounds, excluding the spacing around each footprint.
fn folder_target(
    items: &slint::ModelRc<crate::AppTile>,
    dragged: &crate::AppTile,
    col: i32,
    row: i32,
    held: i32,
) -> i32 {
    use slint::Model;
    if dragged.folder || dragged.widget {
        return -1;
    }
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.pinned && !item.widget && item.id != dragged.id)
        .filter_map(|(index, item)| {
            let width = (col + dragged.span_x - ITEM_GAP as i32)
                .min(item.col + item.span_x - ITEM_GAP as i32)
                - col.max(item.col);
            let height = (row + dragged.span_y - ITEM_GAP as i32)
                .min(item.row + item.span_y - ITEM_GAP as i32)
                - row.max(item.row);
            (width > 0 && height > 0)
                .then_some((index as i32, i64::from(width) * i64::from(height)))
        })
        .max_by_key(|(index, area)| (*area, *index == held, -index))
        .map_or(-1, |(index, _)| index)
}

// Legacy indexes remain readable only so old saved widgets can be migrated.
pub const WIDGETS: [&str; 9] = [
    "Random Text",
    "Random Quote",
    "Random Fact",
    "Random Word",
    "Motivation",
    "Random Number",
    "Custom Text",
    "TickTick Tasks",
    "TickTick Focus Task",
];
pub const WIDGET_SPANS: [(usize, usize); 9] = [
    (2, 2),
    (2, 2),
    (2, 2),
    (2, 1),
    (2, 1),
    (2, 1),
    (2, 2),
    (2, 2),
    (4, 1),
];
pub const WIDGET_MIN_SPANS: [(usize, usize); 9] = [
    (2, 1),
    (2, 1),
    (2, 1),
    (2, 1),
    (2, 1),
    (1, 1),
    (2, 1),
    (2, 2),
    (2, 1),
];
pub const WIDGET_MAX_SPANS: [(usize, usize); 9] = [
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
];

pub fn can_place(
    candidate: Rect,
    others: &BTreeMap<String, Rect>,
    columns: usize,
    rows: usize,
) -> bool {
    candidate.fits(columns, rows) && !others.values().any(|other| candidate.overlaps(*other))
}

pub fn find_first_fit(
    occupied: &BTreeMap<String, Rect>,
    width: usize,
    height: usize,
    columns: usize,
    rows: usize,
) -> Option<Rect> {
    for row in 0..rows {
        let mut col: usize = 0;
        while col.saturating_add(width) <= columns {
            let rect = Rect {
                row,
                col,
                width,
                height,
            };
            if let Some(other) = occupied.values().find(|other| rect.overlaps(**other)) {
                col = col.max(other.col.saturating_add(other.width));
            } else if rect.fits(columns, rows) {
                return Some(rect);
            } else {
                break;
            }
        }
    }
    None
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
    fn empty_moves_keep_neighbors_fixed_and_occupied_moves_are_rejected() {
        let initial = slots(&[("a", 0), ("b", 1), ("c", 3)]);
        let moved = place(&initial, "b", Rect::cell(2, 4), 4, 1).unwrap();
        assert_eq!(moved, slots(&[("a", 0), ("b", 2), ("c", 3)]));
        assert!(place(&initial, "c", Rect::cell(0, 4), 4, 1).is_none());
        assert!(place(&initial, "a", Rect::cell(3, 4), 4, 1).is_none());
        assert_eq!(place(&initial, "a", Rect::cell(0, 4), 4, 1), Some(initial));
    }

    #[test]
    fn staggered_pixel_items_block_moves_without_displacement() {
        let rect = |col, row| Rect {
            col,
            row,
            width: 144,
            height: 144,
        };
        let initial = BTreeMap::from([
            ("a".into(), rect(0, 0)),
            ("b".into(), rect(300, 120)),
            ("c".into(), rect(470, 200)),
        ]);
        for target in [rect(280, 170), rect(320, 170)] {
            assert!(place(&initial, "a", target, 1200, 800).is_none());
            assert!(place(&initial, "external", target, 1200, 800).is_none());
        }
        let moved = place(&initial, "a", rect(700, 300), 1200, 800).unwrap();
        assert_eq!(moved["a"], rect(700, 300));
        assert_eq!(moved["b"], initial["b"]);
        assert_eq!(moved["c"], initial["c"]);
        assert!(place(&initial, "a", rect(1100, 300), 1200, 800).is_none());
    }

    #[test]
    fn folder_intent_uses_tile_overlap_and_largest_target() {
        let tile = |id: &str, col| crate::AppTile {
            id: id.into(),
            col,
            row: 0,
            span_x: 144,
            span_y: 144,
            ..Default::default()
        };
        let dragged = tile("a", 0);
        let items = slint::ModelRc::new(slint::VecModel::from(vec![
            dragged.clone(),
            tile("b", 144),
            tile("c", 288),
        ]));
        assert_eq!(folder_target(&items, &dragged, 0, 0, -1), -1);
        assert_eq!(folder_target(&items, &dragged, 16, 0, -1), -1);
        assert_eq!(folder_target(&items, &dragged, 17, 0, -1), 1);
        assert_eq!(folder_target(&items, &dragged, 100, 100, -1), 1);
        assert_eq!(folder_target(&items, &dragged, 100, 128, -1), -1);
        assert_eq!(folder_target(&items, &dragged, 216, 0, -1), 1);
        assert_eq!(folder_target(&items, &dragged, 216, 0, 2), 2);
        assert_eq!(folder_target(&items, &dragged, 217, 0, 1), 2);
        let mut folder = dragged.clone();
        folder.folder = true;
        assert_eq!(folder_target(&items, &folder, 144, 0, -1), -1);
        let mut widget = dragged.clone();
        widget.widget = true;
        assert_eq!(folder_target(&items, &widget, 144, 0, -1), -1);
        let mut target = tile("folder", 144);
        target.folder = true;
        let targets = |item| slint::ModelRc::new(slint::VecModel::from(vec![item]));
        assert_eq!(
            folder_target(&targets(target.clone()), &dragged, 100, 0, -1),
            0
        );
        target.widget = true;
        assert_eq!(
            folder_target(&targets(target.clone()), &dragged, 100, 0, -1),
            -1
        );
        target.widget = false;
        target.pinned = true;
        assert_eq!(folder_target(&targets(target), &dragged, 100, 0, -1), -1);
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

    #[test]
    fn first_fit_is_row_major_and_whole_rectangle_only() {
        let occupied = slots(&[("a", 0), ("b", 1), ("c", 2)]);
        assert_eq!(
            find_first_fit(&occupied, 2, 2, 4, 3),
            Some(Rect {
                row: 1,
                col: 0,
                width: 2,
                height: 2
            })
        );
        let blocked = BTreeMap::from([("app".into(), Rect::cell(1, 4))]);
        assert!(!can_place(
            Rect {
                row: 0,
                col: 0,
                width: 2,
                height: 1
            },
            &blocked,
            4,
            3
        ));
        assert!(can_place(
            Rect {
                row: 0,
                col: 2,
                width: 2,
                height: 1
            },
            &blocked,
            4,
            3
        ));
    }

    #[test]
    fn widget_resize_never_uses_app_reordering() {
        let occupied = BTreeMap::from([
            (
                "widget".into(),
                Rect {
                    row: 0,
                    col: 0,
                    width: 2,
                    height: 2,
                },
            ),
            ("app".into(), Rect::cell(2, 4)),
        ]);
        assert!(place(&occupied, "widget", Rect::cell(2, 4), 4, 3).is_none());
    }
}

pub fn connect(launcher: &crate::Launcher) {
    use slint::Model;
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
    launcher.on_folder_target(|items, dragged, col, row, held| {
        folder_target(&items, &dragged, col, row, held)
    });
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
    launcher.on_preview_valid(|items, id, slot, width, height, columns, rows| {
        tile_preview(&items, &id, slot, width, height, columns, rows).is_some()
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
            name: WIDGETS[7].into(),
            variant: 7,
            text: "TickTick Tasks".into(),
            ticktick_project_id: String::new(),
            min_number: 1,
            max_number: 100,
        });
        reconcile(&mut config).unwrap();
        config.columns = 3 * cell_pitch(config.item_size);
        config.rows = 2 * cell_pitch(config.item_size);
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
                assert!(rect.fits(restored.columns, restored.rows));
                assert!(
                    occupied
                        .iter()
                        .all(|(other, r)| id == other || !rect.overlaps(*r))
                );
            }
        }
    }
}

#[cfg(test)]
mod live_config_repro {
    use super::*;

    #[test]
    #[ignore = "reproduces the startup failure against the real config"]
    fn reconcile_real_config() {
        let (config, banner) = crate::config::load();
        println!("banner: {banner}");
        println!(
            "pages {} cols {} rows {} item_size {} active {}",
            config.pages.len(),
            config.columns,
            config.rows,
            config.item_size,
            config.active_page
        );
        let mut copy = config.clone();
        match reconcile(&mut copy) {
            Ok(()) => println!("reconcile OK"),
            Err(error) => {
                println!("reconcile FAILED: {error}");
                for (index, page) in copy.pages.iter().enumerate() {
                    for item in &page.items {
                        if let Item::Widget { variant, .. } = item {
                            let rect = copy.positions.get(item.id());
                            println!(
                                "  page {index} widget variant {variant} rect {rect:?} min {:?} spans {:?}",
                                WIDGET_MIN_SPANS[*variant], WIDGET_SPANS[*variant]
                            );
                        }
                    }
                }
            }
        }
    }
}
