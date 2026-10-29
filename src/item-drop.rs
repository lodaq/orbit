use crate::config::{Config, Item, Page};

pub fn apply(
    config: &mut Config,
    id: &str,
    page: usize,
    target: usize,
    into: &str,
    folder_id: String,
) -> bool {
    let mut next = config.clone();
    if crate::spatial::reconcile(&mut next).is_err() {
        return false;
    }
    let Some(source) = find(&next, id) else {
        return false;
    };
    if into == "@dock" {
        if matches!(next.pages[source.0].items[source.1], Item::Widget { .. })
            || (next.pinned.len() >= crate::config::PINNED_LIMIT
                && !next.pinned.iter().any(|pin| pin == id))
        {
            return false;
        }
        if source.2.is_some() {
            let item = take(&mut next, source);
            let page = page.min(next.pages.len() - 1);
            next.pages[page].items.push(item);
        }
        next.pinned.retain(|pin| pin != id);
        next.pinned.insert(target.min(next.pinned.len()), id.into());
    } else if into.is_empty() {
        if page > next.pages.len() || page >= 9 {
            return false;
        }
        let mut rect = next
            .positions
            .get(id)
            .copied()
            .unwrap_or(crate::spatial::Rect {
                width: crate::spatial::item_span(1, next.item_size, next.grid_gap),
                height: crate::spatial::item_span(1, next.item_size, next.grid_gap),
                ..crate::spatial::Rect::cell(0, next.columns)
            });
        rect.row = target / next.columns;
        rect.col = target % next.columns;
        let Some(mut positions) = crate::spatial::place(
            &crate::spatial::occupied(&next, page),
            id,
            rect,
            next.columns,
            next.rows,
        ) else {
            return false;
        };
        if source.0 == page
            && source.2.is_none()
            && !next.pinned.iter().any(|pin| pin == id)
            && next.positions.get(id) == Some(&rect)
        {
            return false;
        }
        let source_folder = next.pages[source.0].items[source.1].id().to_owned();
        let item = take(&mut next, source);
        if source.2.is_some()
            && source.0 == page
            && let Some(remaining) = next.pages[source.0].items.get(source.1)
            && !matches!(remaining, Item::Folder { .. } | Item::Widget { .. })
            && let Some(rect) = positions.remove(&source_folder)
        {
            positions.insert(remaining.id().into(), rect);
        }
        if page == next.pages.len() {
            next.pages.push(Page {
                id: crate::config::new_page_id(),
                items: Vec::new(),
            });
        }
        next.pages[page].items.push(item);
        next.positions.extend(positions);
        next.pinned.retain(|pin| pin != id);
    } else {
        let rect = next.positions.get(into).copied();
        let folder = folder_id.clone();
        if !apply_ordered(&mut next, id, page, target, into, folder_id) {
            return false;
        }
        next.pinned.retain(|pin| pin != id);
        if let Some(rect) = rect {
            next.positions.insert(folder, rect);
        }
    }
    if crate::spatial::reconcile(&mut next).is_err() {
        return false;
    }
    *config = next;
    true
}

pub fn place_search_result(
    config: &mut Config,
    item: Item,
    page: usize,
    target: i32,
    pin: bool,
) -> Result<bool, String> {
    if config.layout_locked {
        return Err("Unlock layout to make changes".into());
    }
    if page > config.pages.len() || page >= 9 {
        return Err("Page is missing".into());
    }
    if pin
        && config.pinned.len() >= crate::config::PINNED_LIMIT
        && !config.pinned.iter().any(|id| id == item.id())
    {
        return Err("Dock is full".into());
    }
    if pin && config.pinned.iter().any(|id| id == item.id()) {
        return Ok(false);
    }
    let mut next = config.clone();
    let source = find(&next, item.id());
    if !pin
        && target < 0
        && source.is_some_and(|source| source.0 == page && source.2.is_none())
        && !next.pinned.iter().any(|id| id == item.id())
    {
        return Ok(false);
    }
    let rect = if pin {
        None
    } else {
        Some(search_result_rect(&next, item.id(), page, target)?)
    };
    let id = item.id().to_owned();
    let item = source.map(|source| take(&mut next, source)).unwrap_or(item);
    if page == next.pages.len() {
        next.pages.push(Page {
            id: crate::config::new_page_id(),
            items: Vec::new(),
        });
    }
    next.pages[page].items.push(item);
    next.pinned.retain(|pinned| pinned != &id);
    if let Some(rect) = rect {
        next.positions.insert(id, rect);
        next.active_page = page;
    } else {
        next.pinned.push(id);
    }
    crate::spatial::reconcile(&mut next)?;
    *config = next;
    Ok(true)
}

fn search_result_rect(
    config: &Config,
    id: &str,
    page: usize,
    target: i32,
) -> Result<crate::spatial::Rect, String> {
    let span = crate::spatial::item_span(1, config.item_size, config.grid_gap);
    let mut occupied = crate::spatial::occupied(config, page);
    occupied.remove(id);
    let rect = if target >= 0 {
        crate::spatial::Rect {
            row: target as usize / config.columns,
            col: target as usize % config.columns,
            width: span,
            height: span,
        }
    } else {
        crate::spatial::find_first_fit(&occupied, span, span, config.columns, config.rows)
            .ok_or("Page is full")?
    };
    if !crate::spatial::can_place(rect, &occupied, config.columns, config.rows) {
        return Err("Not enough free space".into());
    }
    Ok(rect)
}

// Indices describe the final order. IDs keep the source and folder target stable
// when removing an item shifts indices or removes an empty source folder.
fn apply_ordered(
    config: &mut Config,
    id: &str,
    page: usize,
    target: usize,
    into: &str,
    folder_id: String,
) -> bool {
    if page > config.pages.len() || page >= 9 || id == into {
        return false;
    }
    let Some(source) = find(config, id) else {
        return false;
    };
    if !into.is_empty() && !can_insert(config, source, page, into) {
        return false;
    }
    if into.is_empty()
        && source.0 == page
        && source.2.is_none()
        && source.1 == target.min(config.pages[page].items.len() - 1)
    {
        return false;
    }
    if source.0 == page
        && config.pages[page].items[source.1].id() == into
        && let Some(index) = source.2
        && let Item::Folder { apps, .. } = &mut config.pages[page].items[source.1]
    {
        let target = target.min(apps.len() - 1);
        if index == target {
            return false;
        }
        let app = apps.remove(index);
        apps.insert(target, app);
        return true;
    }
    if page == config.pages.len() {
        config.pages.push(Page {
            id: crate::config::new_page_id(),
            items: Vec::new(),
        });
    }
    let item = take(config, source);
    let items = &mut config.pages[page].items;
    if into.is_empty() {
        items.insert(target.min(items.len()), item);
    } else {
        let index = items.iter().position(|item| item.id() == into);
        if let (Some(app), Some(index)) = (item.folder_child(), index) {
            match &mut items[index] {
                Item::Folder { apps, .. } => apps.insert(target.min(apps.len()), app),
                other => {
                    let other = other.folder_child().expect("validated folder target");
                    items[index] = Item::Folder {
                        id: folder_id,
                        name: "Folder".into(),
                        apps: vec![other, app],
                    };
                }
            }
        }
    }
    true
}

fn find(config: &Config, id: &str) -> Option<(usize, usize, Option<usize>)> {
    for (page, contents) in config.pages.iter().enumerate() {
        for (index, item) in contents.items.iter().enumerate() {
            if item.id() == id {
                return Some((page, index, None));
            }
            if let Item::Folder { apps, .. } = item
                && let Some(app) = apps.iter().position(|app| app.id == id)
            {
                return Some((page, index, Some(app)));
            }
        }
    }
    None
}

fn can_insert(
    config: &Config,
    source: (usize, usize, Option<usize>),
    page: usize,
    into: &str,
) -> bool {
    let Some(destination) = config.pages.get(page) else {
        return false;
    };
    let item = &config.pages[source.0].items[source.1];
    (source.2.is_some() || !matches!(item, Item::Folder { .. } | Item::Widget { .. }))
        && destination
            .items
            .iter()
            .any(|item| item.id() == into && !matches!(item, Item::Widget { .. }))
}

fn take(config: &mut Config, source: (usize, usize, Option<usize>)) -> Item {
    let (page, index, app) = source;
    let items = &mut config.pages[page].items;
    let Some(app) = app else {
        return items.remove(index);
    };
    let Item::Folder { apps, .. } = &mut items[index] else {
        unreachable!()
    };
    let item = apps.remove(app).into_item();
    if apps.len() == 1 {
        let remaining = apps.remove(0).into_item();
        let folder_id = items[index].id().to_owned();
        if let Some(rect) = config.positions.remove(&folder_id) {
            config.positions.insert(remaining.id().into(), rect);
        }
        for pin in &mut config.pinned {
            if *pin == folder_id {
                *pin = remaining.id().into();
            }
        }
        items[index] = remaining;
    } else if apps.is_empty() {
        let folder = items.remove(index);
        config.pinned.retain(|id| id != folder.id());
    }
    item
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{App, Icon, Launch};

    #[test]
    fn search_file_can_be_placed_pinned_and_returned_to_page() {
        let mut config = config();
        let file = Item::File {
            id: "search-file".into(),
            name: "better-websites.canvas".into(),
            path: "/notes/better-websites.canvas".into(),
            icon: Icon::Letter,
        };
        place_search_result(&mut config, file.clone(), 1, 40, false).unwrap();
        assert_eq!(config.positions[file.id()].col, 40);
        assert!(config.pages[1].items == vec![file.clone()]);
        place_search_result(&mut config, file.clone(), 1, -1, true).unwrap();
        assert!(config.pinned.iter().any(|id| id == file.id()));
        assert!(!crate::spatial::occupied(&config, 1).contains_key(file.id()));
        place_search_result(&mut config, file.clone(), 2, -1, false).unwrap();
        assert!(!config.pinned.iter().any(|id| id == file.id()));
        assert!(
            !config.pages[1]
                .items
                .iter()
                .any(|item| item.id() == file.id())
        );
        assert_eq!(
            config.pages[2]
                .items
                .iter()
                .filter(|item| item.id() == file.id())
                .count(),
            1
        );
        assert!(config.pages[2].items.contains(&file));
    }

    #[test]
    fn search_placement_rejects_occupied_space_and_locked_layout_atomically() {
        let mut config = config();
        let before = serde_json::to_value(&config).unwrap();
        assert!(place_search_result(&mut config, app("new"), 0, 0, false).is_err());
        assert_eq!(serde_json::to_value(&config).unwrap(), before);
        config.layout_locked = true;
        assert!(place_search_result(&mut config, app("new"), 1, -1, true).is_err());
        assert!(
            !config
                .pages
                .iter()
                .any(|page| page.items.iter().any(|item| item.id() == "new"))
        );
    }

    fn app(id: &str) -> Item {
        Item::App(App {
            id: id.into(),
            name: id.into(),
            icon: Icon::Letter,
            launch: Launch::Command("true".into()),
        })
    }
    fn config() -> Config {
        let mut config = Config::default();
        config.pages = vec![
            Page {
                id: crate::config::new_page_id(),
                items: vec![app("a"), app("b"), app("c")],
            },
            Page {
                id: crate::config::new_page_id(),
                items: vec![],
            },
            Page {
                id: crate::config::new_page_id(),
                items: vec![app("d")],
            },
        ];
        for page in &config.pages {
            for (index, item) in page.items.iter().enumerate() {
                let pitch = crate::spatial::cell_pitch(config.item_size, config.grid_gap);
                config.positions.insert(
                    item.id().into(),
                    crate::spatial::Rect {
                        col: index * pitch,
                        row: 0,
                        width: pitch,
                        height: pitch,
                    },
                );
            }
        }
        config
    }
    fn ids(config: &Config, page: usize) -> Vec<&str> {
        let mut items: Vec<_> = config.pages[page].items.iter().collect();
        items.sort_by_key(|item| {
            config
                .positions
                .get(item.id())
                .map(|rect| (rect.row, rect.col))
        });
        items.into_iter().map(Item::id).collect()
    }
    #[test]
    fn occupied_drop_is_rejected_and_empty_move_preserves_neighbors() {
        let mut c = config();
        let initial = serde_json::to_string(&c).unwrap();
        assert!(!apply(&mut c, "a", 0, 288, "", "f".into()));
        assert_eq!(serde_json::to_string(&c).unwrap(), initial);
        assert!(apply(&mut c, "a", 0, 432, "", "f".into()));
        assert_eq!(c.positions["b"].col, 144);
        assert_eq!(c.positions["c"].col, 288);
        assert!(apply(&mut c, "a", 0, 0, "", "f".into()));
        assert_eq!(ids(&c, 0), ["a", "b", "c"]);
        assert!(!apply(&mut c, "a", 0, 0, "", "f".into()));
    }
    #[test]
    fn move_first_to_last_and_back() {
        let mut c = config();
        apply(&mut c, "a", 2, 144, "", "f".into());
        assert_eq!(ids(&c, 0), ["b", "c"]);
        assert_eq!(ids(&c, 2), ["d", "a"]);
        apply(&mut c, "a", 0, 0, "", "f".into());
        assert_eq!(ids(&c, 0), ["a", "b", "c"]);
    }
    #[test]
    fn merge_across_pages_then_insert_and_reorder_folder() {
        let mut c = config();
        apply(&mut c, "a", 2, 0, "d", "f".into());
        apply(&mut c, "b", 2, usize::MAX, "f", "unused".into());
        apply(&mut c, "b", 2, 0, "f", "unused".into());
        let Item::Folder { apps, .. } = &c.pages[2].items[0] else {
            panic!()
        };
        assert_eq!(
            apps.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
            ["b", "d", "a"]
        );
        apply(&mut c, "b", 0, 0, "", "unused".into());
        assert_eq!(ids(&c, 0), ["b", "c"]);
    }
    #[test]
    fn empty_folder_removed_and_unpinned() {
        let mut c = config();
        apply(&mut c, "a", 0, 0, "b", "f".into());
        c.pinned.push("f".into());
        assert!(apply(&mut c, "a", 2, 144, "", "unused".into()));
        assert!(apply(&mut c, "b", 2, 288, "", "unused".into()));
        assert_eq!(ids(&c, 0), ["c"]);
        assert!(c.pinned.is_empty());
    }
    #[test]
    fn invalid_target_does_not_remove_source() {
        let mut c = config();
        assert!(!apply(&mut c, "a", 0, 0, "missing", "f".into()));
        assert!(!apply(&mut c, "a", 9, 0, "", "f".into()));
        assert!(!apply(&mut c, "a", 0, 0, "a", "f".into()));
        assert_eq!(ids(&c, 0), ["a", "b", "c"]);
    }
    #[test]
    fn last_app_can_stay_in_its_folder() {
        let mut c = config();
        apply(&mut c, "a", 0, 0, "b", "f".into());
        let rect = c.positions["f"];
        apply(&mut c, "a", 1, 0, "", "unused".into());
        assert!(c.pages[0].items.iter().any(|item| item.id() == "b"));
        assert!(!c.pages[0].items.iter().any(|item| item.id() == "f"));
        assert_eq!(c.positions["b"], rect);
    }
    #[test]
    fn folders_reorder_without_nesting() {
        let mut c = config();
        apply(&mut c, "a", 0, 0, "b", "f".into());
        assert!(!apply(&mut c, "f", 2, 0, "d", "unused".into()));
        assert!(apply(&mut c, "f", 2, 144, "", "unused".into()));
        assert_eq!(ids(&c, 2), ["d", "f"]);
    }
    #[test]
    fn new_page_and_page_limit() {
        let mut c = config();
        assert!(apply(&mut c, "a", 3, 0, "", "f".into()));
        assert_eq!(ids(&c, 3), ["a"]);
        while c.pages.len() < 9 {
            c.pages.push(Page {
                id: crate::config::new_page_id(),
                items: vec![],
            });
        }
        assert!(!apply(&mut c, "a", 9, 0, "", "f".into()));
        assert_eq!(ids(&c, 3), ["a"]);
    }

    #[test]
    fn shortcuts_merge_append_extract_and_dissolve_without_changing_type() {
        let website = Item::Website {
            id: "web".into(),
            name: "Website".into(),
            icon: Icon::Text("W".into()),
            url: "https://example.com/path?q=1".into(),
        };
        for shortcut in [
            website,
            Item::File {
                id: "file".into(),
                name: "File".into(),
                icon: Icon::Letter,
                path: "/tmp/example.txt".into(),
            },
            Item::Directory {
                id: "dir".into(),
                name: "Directory".into(),
                icon: Icon::Letter,
                path: "/tmp/example".into(),
            },
        ] {
            let mut c = config();
            c.pages[0].items.push(shortcut.clone());
            crate::spatial::reconcile(&mut c).unwrap();
            let rect = c.positions[shortcut.id()];
            assert!(apply(&mut c, "a", 0, usize::MAX, shortcut.id(), "f".into()));
            assert_eq!(c.positions["f"], rect);
            assert!(apply(&mut c, "b", 0, usize::MAX, "f", "unused".into()));
            let Item::Folder { apps, .. } = c.pages[0]
                .items
                .iter()
                .find(|item| item.id() == "f")
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(
                apps.iter().map(|app| app.id.as_str()).collect::<Vec<_>>(),
                [shortcut.id(), "a", "b"]
            );
            assert!(apply(&mut c, shortcut.id(), 1, 0, "", "unused".into()));
            assert!(c.pages[1].items.contains(&shortcut));
            assert!(apply(
                &mut c,
                shortcut.id(),
                0,
                usize::MAX,
                "f",
                "unused".into()
            ));
            assert!(apply(&mut c, "a", 1, 0, "", "unused".into()));
            assert!(apply(&mut c, "b", 1, 144, "", "unused".into()));
            assert!(c.pages[0].items.contains(&shortcut));
            assert_eq!(c.positions[shortcut.id()], rect);
            assert!(c.valid());
        }
    }
}
