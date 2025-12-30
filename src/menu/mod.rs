use crate::app::{App, Panel, PanelType};
use crate::message::Message;

/// A menu item that combines display and action
pub struct MenuItem {
    pub key: &'static str,
    pub label: MenuLabel,
    pub action: MenuAction,
}

/// How to get the label for a menu item
pub enum MenuLabel {
    Static(&'static str),
    Dynamic(fn(&App, &Panel) -> &'static str),
}

/// What happens when the key is pressed
pub enum MenuAction {
    None,
    Message(Message),
    Dynamic(fn(&App, &Panel) -> Option<Message>),
}

impl MenuItem {
    pub fn static_item(key: &'static str, label: &'static str, message: Message) -> Self {
        Self {
            key,
            label: MenuLabel::Static(label),
            action: MenuAction::Message(message),
        }
    }

    pub fn empty(key: &'static str) -> Self {
        Self {
            key,
            label: MenuLabel::Static(""),
            action: MenuAction::None,
        }
    }

    pub fn dynamic(
        key: &'static str,
        label_fn: fn(&App, &Panel) -> &'static str,
        action_fn: fn(&App, &Panel) -> Option<Message>,
    ) -> Self {
        Self {
            key,
            label: MenuLabel::Dynamic(label_fn),
            action: MenuAction::Dynamic(action_fn),
        }
    }

    pub fn get_label(&self, app: &App, panel: &Panel) -> &'static str {
        match &self.label {
            MenuLabel::Static(s) => s,
            MenuLabel::Dynamic(f) => f(app, panel),
        }
    }

    pub fn get_action(&self, app: &App, panel: &Panel) -> Option<Message> {
        match &self.action {
            MenuAction::None => None,
            MenuAction::Message(msg) => Some(msg.clone()),
            MenuAction::Dynamic(f) => f(app, panel),
        }
    }
}

/// Get menu items for a specific panel type and mode
pub fn get_menu_items(_app: &App, panel: &Panel) -> Vec<MenuItem> {
    use Message::*;

    match &panel.panel_type {
        PanelType::ModeSelection => vec![
            MenuItem::static_item("01", "Help", ShowHelp),
            MenuItem::static_item("02", "Sort", ShowSortDialog),
            MenuItem::empty("03"),
            MenuItem::static_item("04", "Filter", ShowFilterPrompt),
            MenuItem::empty("05"),
            MenuItem::empty("06"),
            MenuItem::empty("07"),
            MenuItem::empty("08"),
            MenuItem::empty("09"),
            MenuItem::static_item("10", "Quit", Quit),
        ],
        PanelType::DriveSelection => vec![
            MenuItem::static_item("01", "Help", ShowHelp),
            MenuItem::static_item("02", "Sort", ShowSortDialog),
            MenuItem::empty("03"),
            MenuItem::static_item("04", "Filter", ShowFilterPrompt),
            MenuItem::empty("05"),
            MenuItem::empty("06"),
            MenuItem::empty("07"),
            MenuItem::empty("08"),
            MenuItem::empty("09"),
            MenuItem::static_item("10", "Quit", Quit),
        ],
        PanelType::ProfileList => vec![
            MenuItem::static_item("01", "Help", ShowHelp),
            MenuItem::static_item("02", "Sort", ShowSortDialog),
            MenuItem::static_item("03", "Edit", ShowProfileConfigForm),
            MenuItem::static_item("04", "Filter", ShowFilterPrompt),
            MenuItem::empty("05"),
            MenuItem::empty("06"),
            MenuItem::empty("07"),
            MenuItem::empty("08"),
            MenuItem::static_item("09", "Advanced", ToggleAdvancedMode),
            MenuItem::static_item("10", "Quit", Quit),
        ],
        PanelType::BucketList { .. } => vec![
            MenuItem::static_item("01", "Help", ShowHelp),
            MenuItem::static_item("02", "Sort", ShowSortDialog),
            MenuItem::static_item("03", "Edit Config", EditBucketConfig),
            MenuItem::static_item("04", "Filter", ShowFilterPrompt),
            MenuItem::empty("05"),
            MenuItem::empty("06"),
            MenuItem::static_item("07", "Add Bucket Conf", ShowConfigForm),
            MenuItem::dynamic("08", |_, _| "Del Bucket Conf", get_delete_action),
            MenuItem::static_item("09", "Advanced", ToggleAdvancedMode),
            MenuItem::static_item("10", "Quit", Quit),
        ],
        PanelType::S3Browser { .. } | PanelType::LocalFilesystem { .. } => vec![
            MenuItem::static_item("01", "Help", ShowHelp),
            MenuItem::static_item("02", "Sort", ShowSortDialog),
            MenuItem::static_item("03", "View", ViewFile),
            MenuItem::static_item("04", "Filter", ShowFilterPrompt),
            MenuItem::dynamic("05", get_f5_label, get_f5_action),
            MenuItem::static_item("06", "Rename", ShowRenamePrompt),
            MenuItem::static_item("07", "Mkdir", ShowCreateFolderPrompt),
            MenuItem::dynamic("08", |_, _| "Delete", get_delete_action),
            MenuItem::static_item("09", "Advanced", ToggleAdvancedMode),
            MenuItem::static_item("10", "Quit", Quit),
        ],
    }
}

/// Get F5 label based on selected item
fn get_f5_label(_app: &App, panel: &Panel) -> &'static str {
    use crate::models::list::ItemType;

    if panel.list_model.is_empty() {
        return "";
    }

    let Some(item) = panel.list_model.get_item(panel.selected_index) else {
        return "";
    };

    match item.item_type {
        ItemType::ParentDir => "",
        ItemType::Directory => "",
        ItemType::File => "Copy File",
    }
}

/// Get F5 action based on selected item
fn get_f5_action(_app: &App, panel: &Panel) -> Option<Message> {
    use crate::models::list::ItemType;

    if panel.list_model.is_empty() {
        return None;
    }

    let selected = panel.list_model.get_item(panel.selected_index);
    let item = selected?;

    match item.item_type {
        ItemType::ParentDir => None,
        ItemType::Directory => None,
        ItemType::File => Some(Message::CopyToOtherPanel),
    }
}

/// Get F8 delete action based on selected item
fn get_delete_action(_app: &App, panel: &Panel) -> Option<Message> {
    use crate::models::list::ItemType;

    if panel.list_model.is_empty() {
        return None;
    }

    let selected = panel.list_model.get_item(panel.selected_index);
    let item = selected?;

    // Can't delete parent directory
    if matches!(item.item_type, ItemType::ParentDir) {
        return None;
    }

    let path = match &panel.panel_type {
        PanelType::S3Browser {
            profile: _,
            bucket,
            prefix,
        } => {
            format!("s3://{}/{}{}", bucket, prefix, item.name)
        }
        PanelType::LocalFilesystem { path } => {
            format!("{}/{}", path.display(), item.name)
        }
        _ => return None,
    };

    Some(Message::ShowDeleteConfirmation {
        path,
        name: item.name.clone(),
        is_dir: matches!(item.item_type, ItemType::Directory),
    })
}

/// Get default advanced mode menu
pub fn get_advanced_menu() -> Vec<MenuItem> {
    use Message::*;

    vec![
        MenuItem::static_item("01", "Help", ShowHelp),
        MenuItem::empty("02"),
        MenuItem::empty("03"),
        MenuItem::empty("04"),
        MenuItem::empty("05"),
        MenuItem::empty("06"),
        MenuItem::empty("07"),
        MenuItem::empty("08"),
        MenuItem::static_item("09", "Back", ToggleAdvancedMode),
        MenuItem::static_item("10", "Quit", Quit),
    ]
}
