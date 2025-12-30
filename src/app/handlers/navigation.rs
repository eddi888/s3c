use crate::app::{App, Panel};

pub fn navigate_up(app: &mut App) {
    let panel = app.get_active_panel();
    if panel.selected_index > 0 {
        panel.selected_index -= 1;
        update_scroll_offset(panel);
    }
}

pub fn navigate_down(app: &mut App) {
    let panel = app.get_active_panel();
    let item_count = panel.list_model.len();
    if panel.selected_index < item_count.saturating_sub(1) {
        panel.selected_index += 1;
        update_scroll_offset(panel);
    }
}

pub fn navigate_page_up(app: &mut App) {
    let panel = app.get_active_panel();
    let page_size = panel.visible_height.saturating_sub(1).max(1);
    panel.selected_index = panel.selected_index.saturating_sub(page_size);
    update_scroll_offset(panel);
}

pub fn navigate_page_down(app: &mut App) {
    let panel = app.get_active_panel();
    let item_count = panel.list_model.len();
    let page_size = panel.visible_height.saturating_sub(1).max(1);
    panel.selected_index = (panel.selected_index + page_size).min(item_count.saturating_sub(1));
    update_scroll_offset(panel);
}

pub fn navigate_home(app: &mut App) {
    let panel = app.get_active_panel();
    panel.selected_index = 0;
    update_scroll_offset(panel);
}

pub fn navigate_end(app: &mut App) {
    let panel = app.get_active_panel();
    let item_count = panel.list_model.len();
    panel.selected_index = item_count.saturating_sub(1);
    update_scroll_offset(panel);
}

fn update_scroll_offset(panel: &mut Panel) {
    if panel.selected_index < panel.scroll_offset {
        panel.scroll_offset = panel.selected_index;
    } else if panel.selected_index >= panel.scroll_offset + panel.visible_height {
        panel.scroll_offset = panel
            .selected_index
            .saturating_sub(panel.visible_height - 1);
    }
}
