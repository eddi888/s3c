use crate::app::{App, PanelType, Screen};
use crate::message::Message;
use crate::models::list::{ItemData, ItemType, PanelItem};
use anyhow::Result;

pub fn show_config_form(app: &mut App) {
    if let PanelType::BucketList { profile } = &app.get_active_panel().panel_type {
        app.config_form.profile = profile.clone();
        app.config_form.bucket = String::new();
        app.config_form.description = String::new();
        app.config_form.region = "eu-west-1".to_string();
        app.config_form.roles = vec![String::new()];
        app.config_form.field = 0;
        app.config_form.cursor = 0;
        app.prev_screen = Some(app.screen.clone());
        app.screen = Screen::ConfigForm;
    }
}

pub fn show_profile_config_form(app: &mut App) {
    if let PanelType::ProfileList = app.get_active_panel().panel_type {
        let selected_index = app.get_active_panel().selected_index;
        let item = app.get_active_panel().list_model.get_item(selected_index);

        if let Some(PanelItem {
            data: ItemData::Profile(profile),
            ..
        }) = item
        {
            let profile = profile.clone();
            app.profile_form.name = profile.clone();

            if let Some(profile_config) = app.config_manager.get_profile_config(&profile) {
                app.profile_form.description =
                    profile_config.description.clone().unwrap_or_default();
                app.profile_form.setup_script =
                    profile_config.setup_script.clone().unwrap_or_default();
            } else {
                app.profile_form.description = String::new();
                app.profile_form.setup_script = String::new();
            }

            app.profile_form.field = 0;
            app.profile_form.cursor = 0;
            app.prev_screen = Some(app.screen.clone());
            app.screen = Screen::ProfileConfigForm;
        }
    }
}

pub fn handle_config_form_message(app: &mut App, msg: Message) -> Result<()> {
    use Message::*;

    match msg {
        ConfigFormUp => {
            if app.config_form.field > 0 {
                app.config_form.field -= 1;
                app.config_form.cursor = get_config_form_field_len(app, app.config_form.field);
            }
        }
        ConfigFormDown => {
            let max_field = app.config_form.roles.len() + 7; // Changed from 5 to 7 (added 2 fields)
            if app.config_form.field < max_field {
                app.config_form.field += 1;
                app.config_form.cursor = get_config_form_field_len(app, app.config_form.field);
            }
        }
        ConfigFormLeft => {
            if app.config_form.cursor > 0 {
                app.config_form.cursor -= 1;
            }
        }
        ConfigFormRight => {
            let max_cursor = get_config_form_field_len(app, app.config_form.field);
            if app.config_form.cursor < max_cursor {
                app.config_form.cursor += 1;
            }
        }
        ConfigFormHome => {
            app.config_form.cursor = 0;
        }
        ConfigFormEnd => {
            app.config_form.cursor = get_config_form_field_len(app, app.config_form.field);
        }
        ConfigFormDelete => {
            if app.config_form.field == 0 && app.config_form.cursor < app.config_form.bucket.len() {
                app.config_form.bucket.remove(app.config_form.cursor);
            } else if app.config_form.field == 1
                && app.config_form.cursor < app.config_form.base_prefix.len()
            {
                app.config_form.base_prefix.remove(app.config_form.cursor);
            } else if app.config_form.field == 2
                && app.config_form.cursor < app.config_form.description.len()
            {
                app.config_form.description.remove(app.config_form.cursor);
            } else if app.config_form.field == 3
                && app.config_form.cursor < app.config_form.region.len()
            {
                app.config_form.region.remove(app.config_form.cursor);
            } else if app.config_form.field == 4
                && app.config_form.cursor < app.config_form.endpoint_url.len()
            {
                app.config_form.endpoint_url.remove(app.config_form.cursor);
            } else if app.config_form.field > 5
                && app.config_form.field <= app.config_form.roles.len() + 5
            {
                let role_idx = app.config_form.field - 6;
                if let Some(role) = app.config_form.roles.get_mut(role_idx) {
                    if app.config_form.cursor < role.len() {
                        role.remove(app.config_form.cursor);
                    }
                }
            }
        }
        ConfigFormChar { c } => {
            if app.config_form.field == 0 {
                app.config_form.bucket.insert(app.config_form.cursor, c);
                app.config_form.cursor += 1;
            } else if app.config_form.field == 1 {
                app.config_form
                    .base_prefix
                    .insert(app.config_form.cursor, c);
                app.config_form.cursor += 1;
            } else if app.config_form.field == 2 {
                app.config_form
                    .description
                    .insert(app.config_form.cursor, c);
                app.config_form.cursor += 1;
            } else if app.config_form.field == 3 {
                app.config_form.region.insert(app.config_form.cursor, c);
                app.config_form.cursor += 1;
            } else if app.config_form.field == 4 {
                app.config_form
                    .endpoint_url
                    .insert(app.config_form.cursor, c);
                app.config_form.cursor += 1;
            } else if app.config_form.field == 5 && c == ' ' {
                // Toggle path_style checkbox with space
                app.config_form.path_style = !app.config_form.path_style;
            } else if app.config_form.field > 5
                && app.config_form.field <= app.config_form.roles.len() + 5
            {
                let role_idx = app.config_form.field - 6;
                if let Some(role) = app.config_form.roles.get_mut(role_idx) {
                    role.insert(app.config_form.cursor, c);
                    app.config_form.cursor += 1;
                }
            }
        }
        ConfigFormBackspace => {
            if app.config_form.cursor > 0 {
                if app.config_form.field == 0 {
                    app.config_form.cursor -= 1;
                    app.config_form.bucket.remove(app.config_form.cursor);
                } else if app.config_form.field == 1 {
                    app.config_form.cursor -= 1;
                    app.config_form.base_prefix.remove(app.config_form.cursor);
                } else if app.config_form.field == 2 {
                    app.config_form.cursor -= 1;
                    app.config_form.description.remove(app.config_form.cursor);
                } else if app.config_form.field == 3 {
                    app.config_form.cursor -= 1;
                    app.config_form.region.remove(app.config_form.cursor);
                } else if app.config_form.field == 4 {
                    app.config_form.cursor -= 1;
                    app.config_form.endpoint_url.remove(app.config_form.cursor);
                } else if app.config_form.field > 5
                    && app.config_form.field <= app.config_form.roles.len() + 5
                {
                    let role_idx = app.config_form.field - 6;
                    if let Some(role) = app.config_form.roles.get_mut(role_idx) {
                        app.config_form.cursor -= 1;
                        role.remove(app.config_form.cursor);
                    }
                }
            }
        }
        ConfigFormAddRole => {
            app.config_form.roles.push(String::new());
        }
        ConfigFormRemoveRole => {
            if app.config_form.roles.len() > 1 {
                app.config_form.roles.pop();
                if app.config_form.field >= 5 + app.config_form.roles.len() {
                    app.config_form.field = 5 + app.config_form.roles.len() - 1;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn get_config_form_field_len(app: &App, field: usize) -> usize {
    match field {
        0 => app.config_form.bucket.len(),
        1 => app.config_form.base_prefix.len(),
        2 => app.config_form.description.len(),
        3 => app.config_form.region.len(),
        4 => app.config_form.endpoint_url.len(),
        5 => 0, // path_style checkbox has no cursor
        _ if field <= app.config_form.roles.len() + 5 => {
            let role_idx = field - 6;
            app.config_form
                .roles
                .get(role_idx)
                .map(|r| r.len())
                .unwrap_or(0)
        }
        _ => 0,
    }
}

pub fn save_config_form(app: &mut App) -> Result<()> {
    if !app.config_form.bucket.trim().is_empty() {
        let roles: Vec<String> = app
            .config_form
            .roles
            .iter()
            .filter(|r| !r.trim().is_empty())
            .cloned()
            .collect();

        let description = if app.config_form.description.trim().is_empty() {
            None
        } else {
            Some(app.config_form.description.clone())
        };

        let base_prefix = if app.config_form.base_prefix.trim().is_empty() {
            None
        } else {
            Some(app.config_form.base_prefix.clone())
        };

        let endpoint_url = if app.config_form.endpoint_url.trim().is_empty() {
            None
        } else {
            Some(app.config_form.endpoint_url.clone())
        };

        let path_style = if app.config_form.path_style {
            Some(true)
        } else {
            None
        };

        app.config_manager.add_bucket_to_profile(
            &app.config_form.profile,
            app.config_form.bucket.clone(),
            roles,
            app.config_form.region.clone(),
            description,
            base_prefix,
            endpoint_url,
            path_style,
        )?;

        // Refresh bucket list if we're on BucketList screen
        let profile = app.config_form.profile.clone();
        let buckets = app.config_manager.get_buckets_for_profile(&profile);
        if let crate::app::PanelType::BucketList { .. } = &app.get_active_panel().panel_type {
            let panel = app.get_active_panel();
            panel
                .list_model
                .set_items(crate::app::converters::buckets_to_items(buckets));
        }

        app.show_success("Bucket configuration saved!");
    }
    Ok(())
}

pub fn edit_bucket_config(app: &mut App) {
    let panel_type = app.get_active_panel().panel_type.clone();
    let selected_index = app.get_active_panel().selected_index;

    if let PanelType::BucketList { profile } = panel_type {
        let item = app.get_active_panel().list_model.get_item(selected_index);

        if let Some(PanelItem {
            item_type: ItemType::ParentDir,
            ..
        }) = item
        {
            return;
        }

        if let Some(PanelItem {
            data: ItemData::Bucket(bucket_config),
            ..
        }) = item
        {
            let bucket_config = bucket_config.clone();

            app.config_form.profile = profile;
            app.config_form.bucket = bucket_config.name.clone();
            app.config_form.base_prefix = bucket_config.base_prefix.clone().unwrap_or_default();
            app.config_form.description = bucket_config.description.clone().unwrap_or_default();
            app.config_form.region = bucket_config.region.clone();
            app.config_form.endpoint_url = bucket_config.endpoint_url.clone().unwrap_or_default();
            app.config_form.path_style = bucket_config.path_style.unwrap_or(false);
            app.config_form.roles = if bucket_config.role_chain.is_empty() {
                vec![String::new()]
            } else {
                bucket_config.role_chain.clone()
            };
            app.config_form.field = 0;
            app.config_form.cursor = 0;
            app.prev_screen = Some(app.screen.clone());
            app.screen = Screen::ConfigForm;
        }
    }
}

pub fn delete_bucket_config(app: &mut App) -> Result<()> {
    let panel_type = app.get_active_panel().panel_type.clone();
    let selected_index = app.get_active_panel().selected_index;

    if let PanelType::BucketList { profile } = panel_type {
        let item = app.get_active_panel().list_model.get_item(selected_index);

        if let Some(PanelItem {
            item_type: ItemType::ParentDir,
            ..
        }) = item
        {
            return Ok(());
        }

        if let Some(PanelItem {
            data: ItemData::Bucket(bucket_config),
            ..
        }) = item
        {
            let bucket_name = bucket_config.name.clone();

            app.config_manager
                .remove_bucket_from_profile(&profile, &bucket_name)?;

            let buckets = app.config_manager.get_buckets_for_profile(&profile);
            let panel = app.get_active_panel();
            panel
                .list_model
                .set_items(crate::app::converters::buckets_to_items(buckets));

            if panel.selected_index > 0 {
                panel.selected_index -= 1;
            }

            app.show_success(&format!("Bucket '{bucket_name}' deleted!"));
        }
    }
    Ok(())
}

pub fn handle_profile_form_message(app: &mut App, msg: Message) -> Result<()> {
    use Message::*;

    match msg {
        ProfileFormUp => {
            if app.profile_form.field > 0 {
                app.profile_form.field -= 1;
                app.profile_form.cursor = match app.profile_form.field {
                    0 => app.profile_form.description.len(),
                    1 => app.profile_form.setup_script.len(),
                    _ => 0,
                };
            }
        }
        ProfileFormDown => {
            if app.profile_form.field < 3 {
                app.profile_form.field += 1;
                app.profile_form.cursor = match app.profile_form.field {
                    0 => app.profile_form.description.len(),
                    1 => app.profile_form.setup_script.len(),
                    _ => 0,
                };
            }
        }
        ProfileFormLeft => {
            if app.profile_form.cursor > 0 {
                app.profile_form.cursor -= 1;
            }
        }
        ProfileFormRight => {
            let max_cursor = match app.profile_form.field {
                0 => app.profile_form.description.len(),
                1 => app.profile_form.setup_script.len(),
                _ => 0,
            };
            if app.profile_form.cursor < max_cursor {
                app.profile_form.cursor += 1;
            }
        }
        ProfileFormHome => {
            app.profile_form.cursor = 0;
        }
        ProfileFormEnd => {
            app.profile_form.cursor = match app.profile_form.field {
                0 => app.profile_form.description.len(),
                1 => app.profile_form.setup_script.len(),
                _ => 0,
            };
        }
        ProfileFormDelete => {
            if app.profile_form.field == 0
                && app.profile_form.cursor < app.profile_form.description.len()
            {
                app.profile_form.description.remove(app.profile_form.cursor);
            } else if app.profile_form.field == 1
                && app.profile_form.cursor < app.profile_form.setup_script.len()
            {
                app.profile_form
                    .setup_script
                    .remove(app.profile_form.cursor);
            }
        }
        ProfileFormChar { c } => {
            if app.profile_form.field == 0 {
                app.profile_form
                    .description
                    .insert(app.profile_form.cursor, c);
                app.profile_form.cursor += 1;
            } else if app.profile_form.field == 1 {
                app.profile_form
                    .setup_script
                    .insert(app.profile_form.cursor, c);
                app.profile_form.cursor += 1;
            }
        }
        ProfileFormBackspace => {
            if app.profile_form.cursor > 0 {
                if app.profile_form.field == 0 {
                    app.profile_form.cursor -= 1;
                    app.profile_form.description.remove(app.profile_form.cursor);
                } else if app.profile_form.field == 1 {
                    app.profile_form.cursor -= 1;
                    app.profile_form
                        .setup_script
                        .remove(app.profile_form.cursor);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn save_profile_config(app: &mut App) -> Result<()> {
    let description = if app.profile_form.description.trim().is_empty() {
        None
    } else {
        Some(app.profile_form.description.clone())
    };

    let setup_script = if app.profile_form.setup_script.trim().is_empty() {
        None
    } else {
        Some(app.profile_form.setup_script.clone())
    };

    if let Some(profile_config) = app
        .config_manager
        .app_config
        .profiles
        .iter_mut()
        .find(|p| p.name == app.profile_form.name)
    {
        profile_config.description = description;
        profile_config.setup_script = setup_script;
    } else {
        app.config_manager
            .app_config
            .profiles
            .push(crate::models::config::ProfileConfig {
                name: app.profile_form.name.clone(),
                buckets: Vec::new(),
                setup_script,
                description,
            });
    }

    app.config_manager.save()?;
    app.show_success("Profile configuration saved!");
    Ok(())
}
