// SPDX-License-Identifier: GPL-3.0-only
// ui/cosmic/src/menu.rs
//
// Context menus: the item lists for a right click on a source file or on a
// page of an annotation tab. Pure data; the views only wrap them in widgets.

use std::collections::HashMap;

use cosmic::widget::menu;
use cosmic::widget::segmented_button::Entity;

use crate::fl;
use crate::message::{MenuAction, Message};
use crate::model::{AppModel, TabContent};

/// Build the right-click context menu for a source file: open it for editing
/// or add it to an open annotation tab.
pub(crate) fn file_context_menu(app: &AppModel) -> Option<Vec<menu::Tree<Message>>> {
    let mut items: Vec<menu::Item<MenuAction, String>> = vec![menu::Item::Button(
        fl!("open-for-editing"),
        None,
        MenuAction::OpenForEditing,
    )];

    let annotation_tabs: Vec<menu::Item<MenuAction, String>> = app
        .tabs
        .values()
        .filter_map(|content| match content {
            TabContent::Annotation { name, document, .. } => Some(menu::Item::Button(
                name.clone(),
                None,
                MenuAction::AddToAnnotation(*document),
            )),
            _ => None,
        })
        .collect();
    if !annotation_tabs.is_empty() {
        items.push(menu::Item::Folder(
            fl!("add-to-annotation"),
            annotation_tabs,
        ));
    }

    let empty_binds: HashMap<menu::KeyBind, MenuAction> = HashMap::new();
    Some(menu::items(&empty_binds, items))
}

/// Build the right-click context menu for a page of an annotation tab:
/// rotate, move and delete the page.
pub(crate) fn page_context_menu(tab: Entity, page: u32) -> Vec<menu::Tree<Message>> {
    let items: Vec<menu::Item<MenuAction, String>> = vec![
        menu::Item::Button(
            fl!("rotate-page-clockwise"),
            None,
            MenuAction::RotatePageClockwise { tab, page },
        ),
        menu::Item::Button(
            fl!("rotate-page-counter-clockwise"),
            None,
            MenuAction::RotatePageCounterClockwise { tab, page },
        ),
        menu::Item::Divider,
        menu::Item::Button(
            fl!("move-page-up"),
            None,
            MenuAction::MovePageUp { tab, page },
        ),
        menu::Item::Button(
            fl!("move-page-down"),
            None,
            MenuAction::MovePageDown { tab, page },
        ),
        menu::Item::Divider,
        menu::Item::Button(
            fl!("delete-page"),
            None,
            MenuAction::DeletePage { tab, page },
        ),
    ];
    let empty_binds: HashMap<menu::KeyBind, MenuAction> = HashMap::new();
    menu::items(&empty_binds, items)
}
