use crate::models::config::BucketConfig;
use crate::operations::s3::S3Object;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PanelItem {
    pub name: String,
    pub item_type: ItemType,
    pub size: Option<u64>,
    pub modified: Option<DateTime<Utc>>,
    pub data: ItemData,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemType {
    Directory,
    File,
    ParentDir,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ItemData {
    Mode(String),
    Drive(PathBuf),
    Profile(String),
    Bucket(BucketConfig),
    S3Object(S3Object),
    LocalFile {
        path: PathBuf,
        #[allow(dead_code)]
        is_dir: bool,
    },
}

#[derive(Debug, Clone)]
pub struct FilterCriteria {
    pub name_pattern: Option<String>,
    pub show_files: bool,
    pub show_dirs: bool,
}

impl Default for FilterCriteria {
    fn default() -> Self {
        Self {
            name_pattern: None,
            show_files: true,
            show_dirs: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortCriteria {
    NameAsc,
    NameDesc,
    SizeAsc,
    SizeDesc,
    ModifiedAsc,
    ModifiedDesc,
}

pub struct PanelListModel {
    items: Vec<PanelItem>,
    filtered_sorted_indices: Vec<usize>,
    filter: FilterCriteria,
    sort: SortCriteria,
}

impl PanelListModel {
    pub fn new(items: Vec<PanelItem>) -> Self {
        let mut model = Self {
            items,
            filtered_sorted_indices: Vec::new(),
            filter: FilterCriteria::default(),
            sort: SortCriteria::NameAsc,
        };
        model.rebuild_view();
        model
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn set_items(&mut self, items: Vec<PanelItem>) {
        self.items = items;
        self.rebuild_view();
    }

    fn rebuild_view(&mut self) {
        let mut indices: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| self.matches_filter(item))
            .map(|(i, _)| i)
            .collect();

        self.sort_indices(&mut indices);
        self.filtered_sorted_indices = indices;
    }

    fn matches_wildcard(text: &str, pattern: &str) -> bool {
        let text = text.to_lowercase();
        let pattern = pattern.to_lowercase();

        // Simple wildcard matching: * matches any sequence of characters
        let parts: Vec<&str> = pattern.split('*').collect();

        if parts.is_empty() {
            return true;
        }

        let mut current_pos = 0;

        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            if i == 0 && !pattern.starts_with('*') {
                // First part must match at the beginning
                if !text.starts_with(part) {
                    return false;
                }
                current_pos = part.len();
            } else if i == parts.len() - 1 && !pattern.ends_with('*') {
                // Last part must match at the end
                if !text[current_pos..].ends_with(part) {
                    return false;
                }
            } else {
                // Middle parts can match anywhere after current position
                if let Some(pos) = text[current_pos..].find(part) {
                    current_pos += pos + part.len();
                } else {
                    return false;
                }
            }
        }

        true
    }

    fn matches_filter(&self, item: &PanelItem) -> bool {
        // Parent dir always shown
        if matches!(item.item_type, ItemType::ParentDir) {
            return true;
        }

        // Type filter
        match item.item_type {
            ItemType::File if !self.filter.show_files => return false,
            ItemType::Directory if !self.filter.show_dirs => return false,
            _ => {}
        }

        // Name pattern filter with wildcard support
        if let Some(pattern) = &self.filter.name_pattern {
            if !Self::matches_wildcard(&item.name, pattern) {
                return false;
            }
        }

        true
    }

    fn sort_indices(&self, indices: &mut [usize]) {
        indices.sort_by(|&a, &b| {
            let item_a = &self.items[a];
            let item_b = &self.items[b];

            // Parent dir always first
            match (&item_a.item_type, &item_b.item_type) {
                (ItemType::ParentDir, _) => return std::cmp::Ordering::Less,
                (_, ItemType::ParentDir) => return std::cmp::Ordering::Greater,
                _ => {}
            }

            match self.sort {
                SortCriteria::NameAsc => {
                    item_a.name.to_lowercase().cmp(&item_b.name.to_lowercase())
                }
                SortCriteria::NameDesc => {
                    item_b.name.to_lowercase().cmp(&item_a.name.to_lowercase())
                }
                SortCriteria::SizeAsc => item_a.size.cmp(&item_b.size),
                SortCriteria::SizeDesc => item_b.size.cmp(&item_a.size),
                SortCriteria::ModifiedAsc => item_a.modified.cmp(&item_b.modified),
                SortCriteria::ModifiedDesc => item_b.modified.cmp(&item_a.modified),
            }
        });
    }

    pub fn get_item(&self, view_index: usize) -> Option<&PanelItem> {
        let data_index = *self.filtered_sorted_indices.get(view_index)?;
        self.items.get(data_index)
    }

    pub fn len(&self) -> usize {
        self.filtered_sorted_indices.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.filtered_sorted_indices.is_empty()
    }

    pub fn set_filter(&mut self, filter: FilterCriteria) {
        self.filter = filter;
        self.rebuild_view();
    }

    pub fn set_sort(&mut self, sort: SortCriteria) {
        self.sort = sort;
        self.rebuild_view();
    }

    #[allow(dead_code)]
    pub fn cycle_sort(&mut self) {
        self.sort = match self.sort {
            SortCriteria::NameAsc => SortCriteria::NameDesc,
            SortCriteria::NameDesc => SortCriteria::SizeDesc,
            SortCriteria::SizeDesc => SortCriteria::SizeAsc,
            SortCriteria::SizeAsc => SortCriteria::ModifiedDesc,
            SortCriteria::ModifiedDesc => SortCriteria::ModifiedAsc,
            SortCriteria::ModifiedAsc => SortCriteria::NameAsc,
        };
        self.rebuild_view();
    }

    pub fn get_current_sort(&self) -> SortCriteria {
        self.sort
    }

    #[allow(dead_code)]
    pub fn get_sort_display(&self) -> String {
        match self.sort {
            SortCriteria::NameAsc => "Name A→Z".to_string(),
            SortCriteria::NameDesc => "Name Z→A".to_string(),
            SortCriteria::SizeAsc => "Size ↑".to_string(),
            SortCriteria::SizeDesc => "Size ↓".to_string(),
            SortCriteria::ModifiedAsc => "Date ↑".to_string(),
            SortCriteria::ModifiedDesc => "Date ↓".to_string(),
        }
    }

    pub fn get_filter_display(&self) -> Option<String> {
        self.filter.name_pattern.as_ref().map(|p| format!("🔍 {p}"))
    }

    pub fn iter(&self) -> impl Iterator<Item = &PanelItem> {
        self.filtered_sorted_indices
            .iter()
            .filter_map(|&i| self.items.get(i))
    }
}
