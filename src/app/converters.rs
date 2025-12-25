use super::LocalFile;
use crate::models::list::{ItemData, ItemType, PanelItem};
use crate::operations::s3::S3Object;

pub fn profiles_to_items(profiles: &[String]) -> Vec<PanelItem> {
    profiles
        .iter()
        .map(|profile| PanelItem {
            name: profile.clone(),
            item_type: ItemType::Directory,
            size: None,
            modified: None,
            data: ItemData::Profile(profile.clone()),
        })
        .collect()
}

pub fn buckets_to_items(buckets: Vec<crate::models::config::BucketConfig>) -> Vec<PanelItem> {
    let mut items = vec![PanelItem {
        name: "..".to_string(),
        item_type: ItemType::ParentDir,
        size: None,
        modified: None,
        data: ItemData::Profile("..".to_string()),
    }];

    items.extend(buckets.into_iter().map(|bucket| PanelItem {
        name: bucket.name.clone(),
        item_type: ItemType::Directory,
        size: None,
        modified: None,
        data: ItemData::Bucket(bucket),
    }));

    items
}

pub fn s3_objects_to_items(objects: Vec<S3Object>) -> Vec<PanelItem> {
    let mut items = vec![PanelItem {
        name: "..".to_string(),
        item_type: ItemType::ParentDir,
        size: None,
        modified: None,
        data: ItemData::Profile("..".to_string()),
    }];

    items.extend(objects.into_iter().map(|obj| PanelItem {
        name: if obj.is_prefix {
            obj.key
                .trim_end_matches('/')
                .rsplit_once('/')
                .map(|(_, name)| name.to_string())
                .unwrap_or_else(|| obj.key.trim_end_matches('/').to_string())
        } else {
            obj.key
                .rsplit_once('/')
                .map(|(_, name)| name.to_string())
                .unwrap_or_else(|| obj.key.clone())
        },
        item_type: if obj.is_prefix {
            ItemType::Directory
        } else {
            ItemType::File
        },
        size: if obj.is_prefix {
            None
        } else {
            Some(obj.size as u64)
        },
        modified: obj.last_modified,
        data: ItemData::S3Object(obj),
    }));

    items
}

pub fn local_files_to_items(files: Vec<LocalFile>, has_parent: bool) -> Vec<PanelItem> {
    let mut items = Vec::new();

    if has_parent {
        items.push(PanelItem {
            name: "..".to_string(),
            item_type: ItemType::ParentDir,
            size: None,
            modified: None,
            data: ItemData::Profile("..".to_string()),
        });
    }

    items.extend(files.into_iter().map(|file| {
        let modified = file.modified.and_then(|st| {
            st.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
        });

        PanelItem {
            name: file.name.clone(),
            item_type: if file.is_dir {
                ItemType::Directory
            } else {
                ItemType::File
            },
            size: if file.is_dir { None } else { Some(file.size) },
            modified,
            data: ItemData::LocalFile {
                path: file.path,
                is_dir: file.is_dir,
            },
        }
    }));

    items
}
