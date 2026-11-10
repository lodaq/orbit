use crate::config::{Config, Item};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};

pub fn cell_pitch(item_size: i32, grid_gap: i32) -> usize {
    item_size.max(1) as usize + grid_gap.clamp(0, 64) as usize
}
pub fn item_span(units: usize, item_size: i32, grid_gap: i32) -> usize {
    units * cell_pitch(item_size, grid_gap)
}
pub fn widget_span(units: usize, item_size: i32, grid_gap: i32) -> usize {
    units * item_size.max(1) as usize + grid_gap.clamp(0, 64) as usize
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Rect {
    pub row: i32,
    pub col: i32,
    pub width: usize,
    pub height: usize,
}
impl Rect {
    pub fn valid(self) -> bool {
        self.width > 0
            && self.height > 0
            && self.width <= i32::MAX as usize
            && self.height <= i32::MAX as usize
    }
    pub fn overlaps(self, other: Self) -> bool {
        i64::from(self.col) < i64::from(other.col) + other.width as i64
            && i64::from(other.col) < i64::from(self.col) + self.width as i64
            && i64::from(self.row) < i64::from(other.row) + other.height as i64
            && i64::from(other.row) < i64::from(self.row) + self.height as i64
    }
}
pub fn place(
    occupied: &BTreeMap<String, Rect>,
    id: &str,
    target: Rect,
) -> Option<BTreeMap<String, Rect>> {
    if !target.valid()
        || occupied
            .iter()
            .any(|(other, rect)| other != id && rect.overlaps(target))
    {
        return None;
    }
    let mut result = occupied.clone();
    result.insert(id.into(), target);
    Some(result)
}
pub fn occupied(config: &Config) -> BTreeMap<String, Rect> {
    config
        .items
        .iter()
        .filter(|item| !config.pinned.iter().any(|id| id == item.id()))
        .filter_map(|item| {
            config
                .positions
                .get(item.id())
                .map(|rect| (item.id().into(), *rect))
        })
        .collect()
}
// A free rectangle touches either the anchor or an occupied edge; inspect those candidates once.
pub fn nearby(occupied: &BTreeMap<String, Rect>, target: Rect) -> Result<Rect, String> {
    if !target.valid() {
        return Err("Invalid Canvas size".into());
    }
    let mut cols = BTreeSet::from([target.col]);
    let mut rows = BTreeSet::from([target.row]);
    for rect in occupied.values() {
        for col in [
            i64::from(rect.col) - target.width as i64,
            i64::from(rect.col) + rect.width as i64,
        ] {
            if let Ok(col) = i32::try_from(col) {
                cols.insert(col);
            }
        }
        for row in [
            i64::from(rect.row) - target.height as i64,
            i64::from(rect.row) + rect.height as i64,
        ] {
            if let Ok(row) = i32::try_from(row) {
                rows.insert(row);
            }
        }
    }
    rows.into_iter()
        .flat_map(|row| {
            cols.iter()
                .copied()
                .map(move |col| Rect { row, col, ..target })
        })
        .filter(|rect| !occupied.values().any(|other| rect.overlaps(*other)))
        .min_by_key(|rect| {
            (
                u64::from(rect.row.abs_diff(target.row)) + u64::from(rect.col.abs_diff(target.col)),
                rect.row,
                rect.col,
            )
        })
        .ok_or_else(|| "No valid Canvas position".into())
}
pub fn reconcile(config: &mut Config) -> Result<(), String> {
    let mut promoted = Vec::new();
    for item in &mut config.items {
        if let Item::Folder { apps, .. } = item {
            let mut i = 0;
            while i < apps.len() {
                if config.pinned.contains(&apps[i].id) {
                    promoted.push(apps.remove(i).into_item());
                } else {
                    i += 1;
                }
            }
        }
    }
    config.items.extend(promoted);
    let ids: HashSet<_> = config
        .items
        .iter()
        .map(|item| item.id().to_owned())
        .collect();
    config.frames.retain_mut(|frame| {
        let mut seen = HashSet::new();
        frame
            .items
            .retain(|id| ids.contains(id) && seen.insert(id.clone()));
        !frame.items.is_empty()
    });
    let widget_ids: HashSet<_> = config
        .items
        .iter()
        .filter(|item| matches!(item, Item::Widget { .. } | Item::Annotation { .. }))
        .map(|item| item.id().to_owned())
        .collect();
    let mut seen = HashSet::new();
    config
        .pinned
        .retain(|id| ids.contains(id) && !widget_ids.contains(id) && seen.insert(id.clone()));
    config
        .positions
        .retain(|id, _| ids.contains(id) && !config.pinned.contains(id));
    let mut occupancy = BTreeMap::new();
    let mut pending = Vec::new();
    for item in &config.items {
        if config.pinned.iter().any(|id| id == item.id()) {
            continue;
        }
        let pitch = cell_pitch(config.item_size, config.grid_gap);
        let saved = config.positions.get(item.id()).copied();
        let mut rect = saved.unwrap_or(Rect {
            width: pitch,
            height: pitch,
            ..Default::default()
        });
        if let Item::Widget { variant, .. } = item {
            if *variant >= WIDGET_SPANS.len() {
                return Err("Invalid widget type".into());
            }
            let (width, height) = if saved.is_some() {
                WIDGET_MIN_SPANS[*variant]
            } else {
                WIDGET_SPANS[*variant]
            };
            rect.width = rect
                .width
                .max(widget_span(width, config.item_size, config.grid_gap));
            rect.height = rect
                .height
                .max(widget_span(height, config.item_size, config.grid_gap));
        }
        if saved.is_some() && place(&occupancy, item.id(), rect).is_some() {
            occupancy.insert(item.id().to_owned(), rect);
        } else {
            pending.push((item.id().to_owned(), rect));
        }
    }
    for (id, rect) in pending {
        let rect = nearby(&occupancy, rect)?;
        occupancy.insert(id, rect);
    }
    config.positions = occupancy;
    Ok(())
}
// Use the visible tile bounds, excluding the spacing around each footprint.
fn folder_target(
    items: &slint::ModelRc<crate::AppTile>,
    dragged: &crate::AppTile,
    col: i32,
    row: i32,
    held: i32,
    gap: i32,
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
            let gap = gap.clamp(0, 64);
            let width =
                (col + dragged.span_x - gap).min(item.col + item.span_x - gap) - col.max(item.col);
            let height =
                (row + dragged.span_y - gap).min(item.row + item.span_y - gap) - row.max(item.row);
            (width > 0 && height > 0)
                .then_some((index as i32, i64::from(width) * i64::from(height)))
        })
        .max_by_key(|(index, area)| (*area, *index == held, -index))
        .map_or(-1, |(index, _)| index)
}

// Legacy indexes remain readable only so old saved widgets can be migrated.
pub const WIDGETS: [&str; 21] = [
    "Random Text",
    "Random Quote",
    "Random Fact",
    "Random Word",
    "Motivation",
    "Random Number",
    "Custom Text",
    "TickTick Tasks",
    "TickTick Focus Task",
    "Random Quote",
    "Random Fact",
    "Motivation",
    "Affirmation",
    "Vocabulary Builder",
    "Achievements",
    "Quran Player",
    "Azan Reminder",
    "Habit Tracker",
    "Remembrances",
    "Remembrances",
    "Internet Usage",
];
pub const WIDGET_SPANS: [(usize, usize); 21] = [
    (2, 2),
    (2, 2),
    (2, 2),
    (2, 1),
    (2, 1),
    (2, 1),
    (2, 2),
    (2, 2),
    (4, 2),
    (2, 2),
    (2, 2),
    (2, 2),
    (2, 2),
    (2, 2),
    (2, 2),
    (3, 2),
    (2, 2),
    (2, 2),
    (3, 5),
    (3, 5),
    (3, 2),
];
pub const WIDGET_MIN_SPANS: [(usize, usize); 21] = [
    (2, 1),
    (2, 1),
    (2, 1),
    (2, 1),
    (2, 1),
    (1, 1),
    (2, 1),
    (2, 2),
    (4, 1),
    (2, 2),
    (2, 2),
    (2, 2),
    (2, 2),
    (2, 2),
    (2, 2),
    (3, 2),
    (2, 2),
    (2, 2),
    (3, 3),
    (3, 3),
    (3, 2),
];
pub const WIDGET_MAX_SPANS: [(usize, usize); 21] = [
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
    (128, 128),
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

pub fn connect(launcher: &crate::Launcher) {
    use slint::Model;
    launcher.on_next_grid_item(|items, index, direction| {
        let Some(current) = items
            .row_data(index.max(0) as usize)
            .filter(|item| !item.pinned)
        else {
            return items
                .iter()
                .position(|item| !item.pinned)
                .map_or(-1, |index| index as i32);
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
    launcher.on_folder_target(|items, dragged, col, row, held, gap| {
        folder_target(&items, &dragged, col, row, held, gap)
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
    launcher.on_preview_valid(|items, id, col, row, width, height| {
        let occupied = tile_positions(&items);
        place(
            &occupied,
            &id,
            Rect {
                col,
                row,
                width: width.max(1) as usize,
                height: height.max(1) as usize,
            },
        )
        .is_some()
    });
    launcher.on_selection_has(|selection, id| selection.iter().any(|selected| selected == id));
    launcher.on_selection_toggle(|selection, id| {
        use slint::Model;
        let mut selection = selection
            .iter()
            .map(|selected| selected.to_string())
            .collect::<Vec<_>>();
        selection_toggle(&mut selection, &id);
        std::rc::Rc::new(slint::VecModel::from(
            selection
                .into_iter()
                .map(slint::SharedString::from)
                .collect::<Vec<_>>(),
        ))
        .into()
    });
    launcher.on_marquee_select(|items, x1, y1, x2, y2, selection, additive| {
        use slint::Model;
        let selection = selection
            .iter()
            .map(|selected| selected.to_string())
            .collect::<Vec<_>>();
        std::rc::Rc::new(slint::VecModel::from(
            marquee_select(&items, x1, y1, x2, y2, &selection, additive)
                .into_iter()
                .map(slint::SharedString::from)
                .collect::<Vec<_>>(),
        ))
        .into()
    });
    launcher.on_frame_index(|frames, id| {
        use slint::Model;
        frames
            .iter()
            .position(|frame| frame.id == id)
            .map_or(-1, |index| index as i32)
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
                    row: item.row,
                    col: item.col,
                    width: item.span_x.max(1) as usize,
                    height: item.span_y.max(1) as usize,
                },
            )
        })
        .collect()
}

pub fn selection_has(selection: &[String], id: &str) -> bool {
    selection.iter().any(|selected| selected == id)
}

pub fn selection_toggle(selection: &mut Vec<String>, id: &str) {
    if let Some(index) = selection.iter().position(|selected| selected == id) {
        selection.remove(index);
    } else {
        selection.push(id.into());
    }
}

// A marquee selects every visible item footprint it touches, starting from the current selection.
pub fn marquee_select(
    items: &slint::ModelRc<crate::AppTile>,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    selection: &[String],
    additive: bool,
) -> Vec<String> {
    use slint::Model;
    let (left, right) = (x1.min(x2), x1.max(x2));
    let (top, bottom) = (y1.min(y2), y1.max(y2));
    let mut result = if additive {
        selection.to_vec()
    } else {
        Vec::new()
    };
    for item in items.iter().filter(|item| !item.pinned) {
        let hit = (item.col as f32) < right
            && left < ((i64::from(item.col) + i64::from(item.span_x)) as f32)
            && (item.row as f32) < bottom
            && top < ((i64::from(item.row) + i64::from(item.span_y)) as f32);
        if hit && !selection_has(&result, &item.id) {
            result.push(item.id.to_string());
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_unbounded_placement_keeps_neighbors_and_finds_nearest_space() {
        let a = Rect {
            col: -100_000,
            row: -70,
            width: 100,
            height: 80,
        };
        let occupied = BTreeMap::from([("a".into(), a)]);
        assert!(place(&occupied, "b", a).is_none());
        let free = nearby(&occupied, a).unwrap();
        assert_eq!(free, Rect { row: -150, ..a });
        let next = place(&occupied, "b", free).unwrap();
        assert_eq!(next["a"], a);
        assert_eq!(nearby(&occupied, a).unwrap(), free);
        assert!(place(&occupied, "a", Rect { col: 100_000, ..a }).is_some());
    }
}
