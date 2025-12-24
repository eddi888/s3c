use crate::app::{ActivePanel, App};

impl App {
    pub fn navigate_up(&mut self) {
        let panel = self.get_active_panel();
        if panel.selected_index > 0 {
            panel.selected_index -= 1;
            if panel.selected_index < panel.scroll_offset {
                panel.scroll_offset = panel.selected_index;
            }
        }
    }

    pub fn navigate_down(&mut self) {
        let max = self.get_panel_item_count();
        let panel = self.get_active_panel();
        if panel.selected_index < max.saturating_sub(1) {
            panel.selected_index += 1;
        }
    }

    pub fn navigate_page_up(&mut self) {
        let panel = self.get_active_panel();
        let page_size = panel.visible_height;

        if panel.selected_index >= page_size {
            panel.selected_index -= page_size;
            panel.scroll_offset = panel.scroll_offset.saturating_sub(page_size);
        } else {
            panel.selected_index = 0;
            panel.scroll_offset = 0;
        }
    }

    pub fn navigate_page_down(&mut self) {
        let max = self.get_panel_item_count();
        let panel = self.get_active_panel();
        let page_size = panel.visible_height;

        if panel.selected_index + page_size < max {
            panel.selected_index += page_size;
            panel.scroll_offset =
                (panel.scroll_offset + page_size).min(max.saturating_sub(panel.visible_height));
        } else if max > 0 {
            panel.selected_index = max - 1;
            panel.scroll_offset = max.saturating_sub(panel.visible_height);
        }
    }

    pub fn navigate_home(&mut self) {
        let panel = self.get_active_panel();
        panel.selected_index = 0;
        panel.scroll_offset = 0;
    }

    pub fn navigate_end(&mut self) {
        let max = self.get_panel_item_count();
        let panel = self.get_active_panel();

        if max > 0 {
            panel.selected_index = max - 1;
            panel.scroll_offset = max.saturating_sub(panel.visible_height);
        }
    }

    pub(crate) fn get_panel_item_count(&self) -> usize {
        let panel = match self.active_panel {
            ActivePanel::Left => &self.left_panel,
            ActivePanel::Right => &self.right_panel,
        };
        panel.list_model.len()
    }

    pub fn switch_panel(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::Left => ActivePanel::Right,
            ActivePanel::Right => ActivePanel::Left,
        };
    }
}
