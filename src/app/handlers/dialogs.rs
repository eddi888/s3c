use crate::app::{App, InputMode, PanelType, Screen};
use crate::models::list::{FilterCriteria, ItemData, ItemType, PanelItem, SortCriteria};

pub fn show_sort_dialog(app: &mut App) {
    let current_sort = app.get_active_panel().list_model.get_current_sort();

    app.sort_dialog.selected = match current_sort {
        SortCriteria::NameAsc => 0,
        SortCriteria::NameDesc => 1,
        SortCriteria::SizeAsc => 2,
        SortCriteria::SizeDesc => 3,
        SortCriteria::ModifiedAsc => 4,
        SortCriteria::ModifiedDesc => 5,
    };

    app.prev_screen = Some(app.screen.clone());
    app.screen = Screen::SortDialog;
}

pub fn apply_sort_selection(app: &mut App) {
    let sort = match app.sort_dialog.selected {
        0 => SortCriteria::NameAsc,
        1 => SortCriteria::NameDesc,
        2 => SortCriteria::SizeAsc,
        3 => SortCriteria::SizeDesc,
        4 => SortCriteria::ModifiedAsc,
        5 => SortCriteria::ModifiedDesc,
        _ => SortCriteria::NameAsc,
    };

    let panel = app.get_active_panel();
    panel.list_model.set_sort(sort);
    panel.selected_index = 0;
}

pub fn show_filter_prompt(app: &mut App) {
    app.input.mode = InputMode::Filter;
    app.input.buffer.clear();
    app.input.prompt =
        "Filter (use * as wildcard and empty for reset) press ENTER to apply:".to_string();
    app.prev_screen = Some(app.screen.clone());
    app.screen = Screen::Input;
}

pub fn apply_filter(app: &mut App, pattern: String) {
    let panel = app.get_active_panel();
    if pattern.trim().is_empty() {
        panel.list_model.set_filter(FilterCriteria::default());
    } else {
        let filter = FilterCriteria {
            name_pattern: Some(pattern),
            show_files: true,
            show_dirs: true,
        };
        panel.list_model.set_filter(filter);
    }
    panel.selected_index = 0;
}

pub fn show_create_folder_prompt(app: &mut App) {
    let panel_type = &app.get_active_panel().panel_type;
    if matches!(
        panel_type,
        PanelType::S3Browser { .. } | PanelType::LocalFilesystem { .. }
    ) {
        app.input.mode = InputMode::CreateFolder;
        app.input.buffer.clear();
        app.input.cursor_position = 0;
        app.input.prompt = "Folder name:".to_string();
        app.prev_screen = Some(app.screen.clone());
        app.screen = Screen::Input;
    }
}

pub fn show_rename_prompt(app: &mut App) {
    let panel = app.get_active_panel();
    let panel_type = panel.panel_type.clone();
    let selected_index = panel.selected_index;

    match panel_type {
        PanelType::S3Browser { .. } => {
            let item = app.get_active_panel().list_model.get_item(selected_index);
            if let Some(PanelItem {
                data: ItemData::S3Object(s3_obj),
                ..
            }) = item
            {
                let key = s3_obj.key.clone();
                app.input.rename_original_path = key.clone();
                app.input.buffer = key;
                app.input.cursor_position = app.input.buffer.chars().count();
                app.input.mode = InputMode::Rename;
                app.input.prompt = "Rename to:".to_string();
                app.prev_screen = Some(app.screen.clone());
                app.screen = Screen::Input;
            }
        }
        PanelType::LocalFilesystem { .. } => {
            let item = app.get_active_panel().list_model.get_item(selected_index);
            if let Some(PanelItem {
                data: ItemData::LocalFile { path, .. },
                ..
            }) = item
            {
                let path_str = path.display().to_string();
                app.input.rename_original_path = path_str.clone();
                app.input.buffer = path_str;
                app.input.cursor_position = app.input.buffer.chars().count();
                app.input.mode = InputMode::Rename;
                app.input.prompt = "Rename to:".to_string();
                app.prev_screen = Some(app.screen.clone());
                app.screen = Screen::Input;
            }
        }
        _ => {}
    }
}

pub fn show_delete_confirmation_dialog(app: &mut App) {
    let selected_index = app.get_active_panel().selected_index;
    let panel_type = app.get_active_panel().panel_type.clone();

    // Check if we're on BucketList and if the item is a bucket
    if let crate::app::PanelType::BucketList { .. } = panel_type {
        let item = app.get_active_panel().list_model.get_item(selected_index);
        let is_bucket = matches!(
            item,
            Some(PanelItem {
                data: ItemData::Bucket(_),
                ..
            })
        );

        if is_bucket {
            // For bucket deletion, use the dedicated handler
            let _ = crate::app::handlers::forms::delete_bucket_config(app);
            return;
        }
    }

    // For non-bucket items, get item data
    let item = app.get_active_panel().list_model.get_item(selected_index);

    // Extract data from item before modifying app
    let (path, name, is_dir) = match item {
        Some(PanelItem {
            item_type: ItemType::ParentDir,
            ..
        }) => return,
        Some(PanelItem {
            item_type,
            name,
            data,
            ..
        }) => {
            let path = match data {
                ItemData::S3Object(obj) => obj.key.clone(),
                ItemData::LocalFile { path, .. } => path.display().to_string(),
                _ => return,
            };
            let is_dir = matches!(item_type, ItemType::Directory);
            (path, name.clone(), is_dir)
        }
        None => return,
    };

    // Now modify app without active borrows
    app.delete_confirmation.path = path;
    app.delete_confirmation.name = name;
    app.delete_confirmation.is_dir = is_dir;
    app.delete_confirmation.button = 0;
    app.prev_screen = Some(app.screen.clone());
    app.screen = Screen::DeleteConfirmation;
}
