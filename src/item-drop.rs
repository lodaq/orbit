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
            || (next.pinned.len() >= 8 && !next.pinned.iter().any(|pin| pin == id))
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
            .unwrap_or(crate::spatial::Rect::cell(0, next.columns));
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
    fn reorder_both_directions_and_end() {
        let mut c = config();
        assert!(apply(&mut c, "a", 0, 2, "", "f".into()));
        assert_eq!(ids(&c, 0), ["b", "a", "c"]);
        assert!(apply(&mut c, "a", 0, 0, "", "f".into()));
        assert_eq!(ids(&c, 0), ["a", "b", "c"]);
        assert!(!apply(&mut c, "a", 0, 0, "", "f".into()));
    }
    #[test]
    fn move_first_to_last_and_back() {
        let mut c = config();
        apply(&mut c, "a", 2, 1, "", "f".into());
        assert_eq!(ids(&c, 0), ["b", "c"]);
        assert_eq!(ids(&c, 2), ["d", "a"]);
        apply(&mut c, "a", 0, 1, "", "f".into());
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
        apply(&mut c, "a", 2, 0, "", "unused".into());
        apply(&mut c, "b", 2, 0, "", "unused".into());
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
        assert!(apply(&mut c, "f", 2, 1, "", "unused".into()));
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
            assert!(apply(&mut c, "b", 1, 1, "", "unused".into()));
            assert!(c.pages[0].items.contains(&shortcut));
            assert_eq!(c.positions[shortcut.id()], rect);
            assert!(c.valid());
        }
    }
}
