use crate::app::{App, PanelType, Screen};
use crate::message::Message;
use crossterm::event::{KeyCode, KeyModifiers};

/// Converts keyboard input to Message based on current screen/state
pub fn key_to_message(app: &App, key: KeyCode, modifiers: KeyModifiers) -> Option<Message> {
    // Handle error/success message dismissal
    if !app.error_message.is_empty() || !app.success_message.is_empty() {
        return Some(Message::Clear);
    }

    match app.screen {
        Screen::DualPanel => dual_panel_key_to_message(app, key),
        Screen::ConfigForm => config_form_key_to_message(app, key),
        Screen::ProfileConfigForm => profile_form_key_to_message(app, key),
        Screen::SortDialog => sort_dialog_key_to_message(key),
        Screen::DeleteConfirmation => delete_confirmation_key_to_message(key),
        Screen::FileContentPreview => file_content_preview_key_to_message(key),
        Screen::ImagePreview => image_preview_key_to_message(key),
        Screen::Input => input_key_to_message(key, modifiers),
        Screen::Help => Some(Message::GoBack),
    }
}

fn dual_panel_key_to_message(app: &App, key: KeyCode) -> Option<Message> {
    // Get active panel type without mutable borrow
    let active_panel_type = match app.active_panel {
        crate::app::ActivePanel::Left => &app.left_panel.panel_type,
        crate::app::ActivePanel::Right => &app.right_panel.panel_type,
    };

    match key {
        KeyCode::Char('x') | KeyCode::Char('X') => {
            // Cancel transfer if one is running
            if app.background_transfer_task.is_some() {
                Some(Message::CancelTransfer)
            } else {
                None
            }
        }
        KeyCode::Char('q') | KeyCode::F(10) => Some(Message::Quit),
        KeyCode::Char('?') | KeyCode::F(1) => Some(Message::ShowHelp),
        KeyCode::F(12) => Some(Message::ToggleLocalFilesystem),
        KeyCode::Up => Some(Message::NavigateUp),
        KeyCode::Down => Some(Message::NavigateDown),
        KeyCode::PageUp => Some(Message::NavigatePageUp),
        KeyCode::PageDown => Some(Message::NavigatePageDown),
        KeyCode::Home => Some(Message::NavigateHome),
        KeyCode::End => Some(Message::NavigateEnd),
        KeyCode::Tab => Some(Message::SwitchPanel),
        KeyCode::Enter => Some(Message::EnterSelected),
        KeyCode::F(2) => Some(Message::ShowSortDialog),
        KeyCode::F(4) => Some(Message::ShowFilterPrompt),
        KeyCode::F(7) => match active_panel_type {
            PanelType::BucketList { .. } => Some(Message::ShowConfigForm),
            PanelType::S3Browser { .. } | PanelType::LocalFilesystem { .. } => {
                Some(Message::ShowCreateFolderPrompt)
            }
            _ => None,
        },
        KeyCode::F(3) => match active_panel_type {
            PanelType::ProfileList => Some(Message::ShowProfileConfigForm),
            PanelType::BucketList { .. } => Some(Message::EditBucketConfig),
            PanelType::S3Browser { .. } | PanelType::LocalFilesystem { .. } => {
                Some(Message::ViewFile)
            }
        },
        KeyCode::F(5) => Some(Message::CopyToOtherPanel),
        KeyCode::F(6) => {
            if matches!(
                active_panel_type,
                PanelType::S3Browser { .. } | PanelType::LocalFilesystem { .. }
            ) {
                Some(Message::ShowRenamePrompt)
            } else {
                None
            }
        }
        KeyCode::Delete | KeyCode::F(8) => Some(Message::DeleteFile),
        KeyCode::F(9) => Some(Message::ToggleAdvancedMode),
        _ => None,
    }
}

fn sort_dialog_key_to_message(key: KeyCode) -> Option<Message> {
    match key {
        KeyCode::Up => Some(Message::SortDialogUp),
        KeyCode::Down => Some(Message::SortDialogDown),
        KeyCode::Enter => Some(Message::ApplySort),
        KeyCode::Esc => Some(Message::GoBack),
        _ => None,
    }
}

fn delete_confirmation_key_to_message(key: KeyCode) -> Option<Message> {
    match key {
        KeyCode::Left => Some(Message::DeleteConfirmationLeft),
        KeyCode::Right => Some(Message::DeleteConfirmationRight),
        KeyCode::Tab => Some(Message::DeleteConfirmationRight), // Cycle between buttons
        KeyCode::Enter => Some(Message::ConfirmDelete),
        KeyCode::Esc => Some(Message::GoBack),
        _ => None,
    }
}

fn profile_form_key_to_message(app: &App, key: KeyCode) -> Option<Message> {
    match key {
        KeyCode::Up => Some(Message::ProfileFormUp),
        KeyCode::Down => Some(Message::ProfileFormDown),
        KeyCode::Left => Some(Message::ProfileFormLeft),
        KeyCode::Right => Some(Message::ProfileFormRight),
        KeyCode::Home => Some(Message::ProfileFormHome),
        KeyCode::End => Some(Message::ProfileFormEnd),
        KeyCode::Delete => Some(Message::ProfileFormDelete),
        KeyCode::Char(c) => Some(Message::ProfileFormChar { c }),
        KeyCode::Backspace => Some(Message::ProfileFormBackspace),
        KeyCode::Enter => {
            if app.profile_form.field == 2 {
                Some(Message::SaveProfileConfig)
            } else if app.profile_form.field == 3 {
                Some(Message::GoBack)
            } else {
                Some(Message::ProfileFormDown)
            }
        }
        KeyCode::Esc => Some(Message::GoBack),
        _ => None,
    }
}

fn config_form_key_to_message(app: &App, key: KeyCode) -> Option<Message> {
    match key {
        KeyCode::Up => Some(Message::ConfigFormUp),
        KeyCode::Down => Some(Message::ConfigFormDown),
        KeyCode::Left => Some(Message::ConfigFormLeft),
        KeyCode::Right => Some(Message::ConfigFormRight),
        KeyCode::Home => Some(Message::ConfigFormHome),
        KeyCode::End => Some(Message::ConfigFormEnd),
        KeyCode::Delete => Some(Message::ConfigFormDelete),
        KeyCode::Char('+') => {
            let button_field = app.config_form.roles.len() + 4;
            if app.config_form.field >= button_field {
                Some(Message::ConfigFormAddRole)
            } else {
                Some(Message::ConfigFormChar { c: '+' })
            }
        }
        KeyCode::Char('-') => {
            let button_field = app.config_form.roles.len() + 4;
            if app.config_form.field >= button_field {
                Some(Message::ConfigFormRemoveRole)
            } else {
                Some(Message::ConfigFormChar { c: '-' })
            }
        }
        KeyCode::Char(c) => Some(Message::ConfigFormChar { c }),
        KeyCode::Backspace => Some(Message::ConfigFormBackspace),
        KeyCode::Enter => {
            let button_field = app.config_form.roles.len() + 4;
            if app.config_form.field == button_field {
                Some(Message::SaveConfigForm)
            } else if app.config_form.field == button_field + 1 {
                Some(Message::GoBack)
            } else {
                Some(Message::ConfigFormDown)
            }
        }
        KeyCode::Esc => Some(Message::GoBack),
        _ => None,
    }
}

fn file_content_preview_key_to_message(key: KeyCode) -> Option<Message> {
    match key {
        KeyCode::Up => Some(Message::FilePreviewUp),
        KeyCode::Down => Some(Message::FilePreviewDown),
        KeyCode::PageUp => Some(Message::FilePreviewPageUp),
        KeyCode::PageDown => Some(Message::FilePreviewPageDown),
        KeyCode::Home => Some(Message::FilePreviewHome),
        KeyCode::End => Some(Message::FilePreviewEnd),
        KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBack),
        _ => None,
    }
}

fn image_preview_key_to_message(key: KeyCode) -> Option<Message> {
    match key {
        KeyCode::Esc | KeyCode::Char('q') => Some(Message::GoBack),
        _ => None,
    }
}

fn input_key_to_message(key: KeyCode, modifiers: KeyModifiers) -> Option<Message> {
    match key {
        KeyCode::Enter => Some(Message::InputSubmit),
        KeyCode::Esc => Some(Message::InputCancel),
        KeyCode::Left => Some(Message::InputLeft),
        KeyCode::Right => Some(Message::InputRight),
        KeyCode::Home => Some(Message::InputHome),
        KeyCode::End => Some(Message::InputEnd),
        KeyCode::Delete => Some(Message::InputDelete),
        KeyCode::Backspace => Some(Message::InputBackspace),
        KeyCode::Char(c) => {
            let ctrl = modifiers.contains(KeyModifiers::CONTROL);
            if ctrl && (c == 'c' || c == 'C') {
                Some(Message::InputCancel)
            } else {
                Some(Message::InputChar { c, ctrl })
            }
        }
        _ => None,
    }
}
