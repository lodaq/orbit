use crate::config::{Config, Item};
use crate::spatial::{self, Rect};

pub fn apply(
    config: &mut Config,
    id: &str,
    col: i32,
    row: i32,
    into: &str,
    folder_id: String,
) -> bool {
    if config.layout_locked || id == into {
        return false;
    }
    let mut next = config.clone();
    let Some(source) = find(&next, id) else {
        return false;
    };
    if into == "@dock" {
        if matches!(next.items[source.0], Item::Widget { .. })
            || (next.pinned.len() >= crate::config::PINNED_LIMIT
                && !next.pinned.iter().any(|pin| pin == id))
        {
            return false;
        }
        if source.1.is_some() {
            let item = take(&mut next, source);
            next.items.push(item);
        }
        let old = next.pinned.clone();
        next.pinned.retain(|pin| pin != id);
        next.pinned
            .insert((col.max(0) as usize).min(next.pinned.len()), id.into());
        if old == next.pinned && source.1.is_none() {
            return false;
        }
        next.positions.remove(id);
    } else if into.is_empty() {
        let span = spatial::cell_pitch(next.item_size, next.grid_gap);
        let mut rect = Rect {
            col,
            row,
            ..next.positions.get(id).copied().unwrap_or(Rect {
                width: span,
                height: span,
                ..Default::default()
            })
        };
        if !next.pinned.iter().any(|pin| pin == id)
            && source.1.is_none()
            && next.positions.get(id) == Some(&rect)
        {
            return false;
        }
        let occupancy = spatial::occupied(&next);
        if source.1.is_some() || next.pinned.iter().any(|pin| pin == id) {
            let Ok(free) = spatial::nearby(&occupancy, rect) else {
                return false;
            };
            rect = free;
        } else if spatial::place(&occupancy, id, rect).is_none() {
            return false;
        }
        let item = take(&mut next, source);
        next.items.push(item);
        next.pinned.retain(|pin| pin != id);
        next.positions.insert(id.into(), rect);
    } else {
        let Some(target) = next.items.iter().find(|item| item.id() == into) else {
            return false;
        };
        if !matches!(target, Item::Folder { .. }) && target.folder_child().is_none() {
            return false;
        }
        if source.1.is_none() && next.items[source.0].folder_child().is_none() {
            return false;
        }
        if let Some(index) = source.1
            && next.items[source.0].id() == into
        {
            let Item::Folder { apps, .. } = &mut next.items[source.0] else {
                return false;
            };
            let position = (col.max(0) as usize).min(apps.len() - 1);
            if index == position {
                return false;
            }
            let app = apps.remove(index);
            apps.insert(position, app);
        } else {
            let new_folder = !matches!(target, Item::Folder { .. });
            if new_folder && (folder_id.is_empty() || find(&next, &folder_id).is_some()) {
                return false;
            }
            let target_rect = next.positions.get(into).copied();
            let item = take(&mut next, source);
            let app = item.folder_child().expect("validated folder child");
            let Some(target) = next.items.iter_mut().find(|item| item.id() == into) else {
                return false;
            };
            match target {
                Item::Folder { apps, .. } => {
                    apps.insert((col.max(0) as usize).min(apps.len()), app)
                }
                other => {
                    let original = other.folder_child().expect("validated folder target");
                    *other = Item::Folder {
                        id: folder_id.clone(),
                        name: "Folder".into(),
                        apps: vec![original, app],
                    };
                    next.positions.remove(into);
                    if let Some(rect) = target_rect {
                        next.positions.insert(folder_id.clone(), rect);
                    }
                    for pin in &mut next.pinned {
                        if pin == into {
                            *pin = folder_id.clone();
                        }
                    }
                }
            }
            next.pinned.retain(|pin| pin != id);
            next.positions.remove(id);
        }
    }
    if spatial::reconcile(&mut next).is_err() {
        return false;
    }
    *config = next;
    true
}

pub fn place_search_result(config: &mut Config, item: Item, rect: Option<Rect>, pin: bool) -> bool {
    if config.layout_locked || (pin && matches!(item, Item::Widget { .. })) {
        return false;
    }
    let id = item.id().to_owned();
    if pin
        && (config.pinned.iter().any(|pin| pin == &id)
            || config.pinned.len() >= crate::config::PINNED_LIMIT)
    {
        return false;
    }
    let source = find(config, &id);
    if !pin
        && rect.is_none()
        && source.is_some_and(|source| source.1.is_none())
        && !config.pinned.contains(&id)
    {
        return false;
    }
    let mut next = config.clone();
    let item = source.map(|source| take(&mut next, source)).unwrap_or(item);
    next.pinned.retain(|pin| pin != &id);
    next.positions.remove(&id);
    if pin {
        next.pinned.push(id.clone());
    } else {
        let span = spatial::cell_pitch(next.item_size, next.grid_gap);
        let target = rect.unwrap_or(Rect {
            width: span,
            height: span,
            ..Default::default()
        });
        let Ok(target) = spatial::nearby(&spatial::occupied(&next), target) else {
            return false;
        };
        next.positions.insert(id.clone(), target);
    }
    next.items.push(item);
    if spatial::reconcile(&mut next).is_err() {
        return false;
    }
    *config = next;
    true
}

fn find(config: &Config, id: &str) -> Option<(usize, Option<usize>)> {
    for (index, item) in config.items.iter().enumerate() {
        if item.id() == id {
            return Some((index, None));
        }
        if let Item::Folder { apps, .. } = item
            && let Some(child) = apps.iter().position(|app| app.id == id)
        {
            return Some((index, Some(child)));
        }
    }
    None
}
fn take(config: &mut Config, source: (usize, Option<usize>)) -> Item {
    let (index, child) = source;
    let Some(child) = child else {
        return config.items.remove(index);
    };
    let Item::Folder { apps, .. } = &mut config.items[index] else {
        unreachable!()
    };
    let item = apps.remove(child).into_item();
    if apps.len() == 1 {
        let remaining = apps.remove(0).into_item();
        let folder = config.items[index].id().to_owned();
        if let Some(rect) = config.positions.remove(&folder) {
            config.positions.insert(remaining.id().into(), rect);
        }
        for pin in &mut config.pinned {
            if *pin == folder {
                *pin = remaining.id().into();
            }
        }
        config.items[index] = remaining;
    } else if apps.is_empty() {
        let folder = config.items.remove(index);
        config.positions.remove(folder.id());
        config.pinned.retain(|id| id != folder.id());
    }
    item
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{App, Icon, Launch};
    fn file(id: &str) -> Item {
        Item::File {
            id: id.into(),
            name: id.into(),
            path: format!("/tmp/{id}"),
            icon: Icon::Letter,
        }
    }
    fn rect(col: i32, row: i32) -> Rect {
        Rect {
            col,
            row,
            width: 144,
            height: 144,
        }
    }
    #[test]
    fn nearby_add_preserves_neighbors_and_moving_collision_is_atomic() {
        let mut config = Config::default();
        assert!(place_search_result(
            &mut config,
            file("a"),
            Some(rect(-100_000, 50_000)),
            false
        ));
        assert!(place_search_result(
            &mut config,
            file("b"),
            Some(rect(-100_000, 50_000)),
            false
        ));
        let before = serde_json::to_value(&config).unwrap();
        assert!(!apply(
            &mut config,
            "b",
            -100_000,
            50_000,
            "",
            String::new()
        ));
        assert_eq!(serde_json::to_value(&config).unwrap(), before);
        assert_eq!(config.positions["a"], rect(-100_000, 50_000));
    }
    #[test]
    fn folder_child_extraction_retains_type_and_dissolves_folder() {
        let mut config = Config::default();
        config.items.push(Item::Folder {
            id: "folder".into(),
            name: "Folder".into(),
            apps: vec![
                file("a").folder_child().unwrap(),
                App {
                    id: "b".into(),
                    name: "App".into(),
                    icon: Icon::Letter,
                    launch: Launch::Command("true".into()),
                },
            ],
        });
        config.positions.insert("folder".into(), rect(0, 0));
        assert!(apply(&mut config, "a", -1000, -1000, "", String::new()));
        assert!(
            config
                .items
                .iter()
                .any(|item| matches!(item,Item::File {id,..} if id=="a"))
        );
        assert_eq!(config.positions["b"], rect(0, 0));
        assert!(
            !config
                .items
                .iter()
                .any(|item| matches!(item, Item::Folder { .. }))
        );
        assert!(apply(&mut config, "a", 0, 0, "@dock", String::new()));
        assert!(!config.positions.contains_key("a"));
        assert!(place_search_result(
            &mut config,
            file("a"),
            Some(rect(0, 0)),
            false
        ));
        assert!(!config.pinned.contains(&"a".into()));
        assert!(!config.positions["a"].overlaps(config.positions["b"]));
    }
    #[test]
    fn locked_layout_blocks_all_drop_and_search_mutations() {
        let mut config = Config::default();
        config.layout_locked = true;
        assert!(!place_search_result(
            &mut config,
            file("a"),
            Some(rect(0, 0)),
            false
        ));
        assert!(!apply(&mut config, "a", 0, 0, "@dock", String::new()));
        assert!(config.items.is_empty());
    }
}
