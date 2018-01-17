use std::path::{Path, PathBuf};
use std::collections::HashSet;

use library::MediaLibrary;
use helpers::normalize;
use metadata::{MetaValue, MetaTarget};
use error::*;

trait LabelExtractor {
    fn extract_label<S: AsRef<str>>(&self, item_file_name: S) -> String;
}

pub enum LookupDirection {
    Origin,
    Parents,
    Children,
}

pub struct LookupOptions {
    field_name: String,
    labels: Option<HashSet<String>>,
}

impl LookupOptions {
    pub fn new<S: Into<String>>(field_name: S) -> Self {
        let field_name = field_name.into();

        LookupOptions {
            field_name,
            labels: None,
        }
    }

    pub fn add_label<S: Into<String>>(&mut self, label: S) -> &mut Self {
        let label = label.into();

        match self.labels {
            None => { self.labels = Some(hashset![label]); },
            Some(ref mut hs) => { hs.insert(label); },
        }

        self
    }

    pub fn add_labels<SS, S>(&mut self, labels: SS) -> &mut Self
    where SS: IntoIterator<Item = S>,
          S: Into<String>
    {
        let labels = labels.into_iter().map(Into::into);

        match self.labels {
            None => {
                self.labels = Some(labels.collect());
            },
            Some(ref mut hs) => {
                for label in labels {
                    hs.insert(label);
                }
            },
        }

        self
    }
}

pub struct LookupOutcomeNew {
    meta_value: MetaValue,
    meta_target: MetaTarget,
    meta_file_path: PathBuf,
    item_file_path: PathBuf,
}

pub type LookupOutcome = Option<(MetaValue, PathBuf)>;

pub fn lookup_origin<P: AsRef<Path>>(
    media_library: &MediaLibrary,
    abs_item_path: P,
    options: &LookupOptions,
    ) -> Result<LookupOutcome>
{
    let abs_item_path = normalize(abs_item_path.as_ref());

    // Get meta file paths from item path.
    let meta_file_paths = media_library.meta_fps_from_item_fp(&abs_item_path)?;

    'meta: for meta_file_path in meta_file_paths {
        // Open this meta file path and see if it contains the field we are looking for.
        let records = media_library.item_fps_from_meta_fp(&meta_file_path)?;

        // Search found item paths for a match to target item path.
        'item: for (found_item_path, found_meta_block) in records {
            if abs_item_path == found_item_path {
                // Found a match for this path, check if the desired field is contained in meta block.
                match found_meta_block.get(&options.field_name) {
                    Some(val) => {
                        return Ok(Some((val.clone(), found_item_path)))
                    },
                    None => {
                        continue 'item;
                    }
                }
            }
        }
    }

    // No error, but value was not found.
    Ok(None)
}

pub fn lookup_parents<P: AsRef<Path>>(
    media_library: &MediaLibrary,
    abs_item_path: P,
    options: &LookupOptions,
    ) -> Result<LookupOutcome>
{
    let mut curr_item_path = normalize(abs_item_path.as_ref());

    while let Some(curr_item_parent) = curr_item_path.parent().map(Path::to_path_buf) {
        if !media_library.is_proper_sub_path(&curr_item_parent) {
            break;
        }

        match lookup_origin(media_library, &curr_item_parent, options)? {
            Some(results) => { return Ok(Some(results)); },
            None => {},
        }

        curr_item_path = curr_item_parent;
    }

    // No error, but value was not found.
    Ok(None)
}

pub fn lookup_children<P: AsRef<Path>>(
    media_library: &MediaLibrary,
    abs_item_path: P,
    options: &LookupOptions,
    ) -> Result<LookupOutcome>
{
    let curr_item_path = normalize(abs_item_path.as_ref());

    // A non-directory has no children; this is a leaf (and a base case).
    if !curr_item_path.is_dir() {
        return Ok(None);
    }

    // Look at the metadata for each child contained in this directory, in the expected order.
    // for child_abs_item_path in

    // No error, but value was not found.
    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::collections::HashMap;
    use std::thread::sleep;
    use std::time::Duration;

    use tempdir::TempDir;

    use super::{lookup_origin, lookup_parents, LookupOptions};
    use library::{MediaLibrary, LibraryBuilder};
    use library::selection::Selection;
    use library::sort_order::SortOrder;
    use metadata::{MetaTarget, MetaValue};
    use test_helpers::create_temp_media_test_dir;

    fn setup(name: &str) -> (TempDir, MediaLibrary) {
        let temp_media_root = create_temp_media_test_dir(name);
        sleep(Duration::from_millis(1));

        let meta_target_specs = vec![
            (String::from("self.yml"), MetaTarget::Contains),
            (String::from("item.yml"), MetaTarget::Siblings),
        ];

        let selection = Selection::Or(
            Box::new(Selection::Ext(String::from("flac"))),
            Box::new(Selection::IsDir),
        );
        let media_lib = LibraryBuilder::new(temp_media_root.path(), meta_target_specs).selection(selection).create().expect("Unable to create media library");

        (temp_media_root, media_lib)
    }

    #[test]
    fn test_lookup_origin() {
        let (temp_media_root, media_lib) = setup("test_lookup_origin");
        let tp = temp_media_root.path();

        let inputs_and_expected = vec![
            ((tp.join("ALBUM_01").join("DISC_01"), "const_key"), Some((MetaValue::Str("const_val".to_string()), tp.join("ALBUM_01").join("DISC_01")))),
            ((tp.join("ALBUM_01").join("DISC_01"), "DISC_01_self_key"), Some((MetaValue::Str("DISC_01_self_val".to_string()), tp.join("ALBUM_01").join("DISC_01")))),
            ((tp.join("ALBUM_01").join("DISC_01"), "DISC_01_item_key"), Some((MetaValue::Str("DISC_01_item_val".to_string()), tp.join("ALBUM_01").join("DISC_01")))),
            ((tp.join("ALBUM_01").join("DISC_01"), "ALBUM_01_item_key"), None),
            ((tp.join("ALBUM_01").join("DISC_01"), "ALBUM_01_self_key"), None),
            ((tp.join("ALBUM_01").join("DISC_01"), "NON_EXISTENT_KEY"), None),
            ((tp.to_path_buf(), "ROOT_item_key"), None),
            ((tp.to_path_buf(), "ROOT_self_key"), Some((MetaValue::Str("ROOT_self_val".to_string()), tp.to_path_buf()))),
        ];

        for ((target_item_path, field_name), expected) in inputs_and_expected {
            let produced = lookup_origin(&media_lib, target_item_path, &LookupOptions::new(field_name)).unwrap();

            assert_eq!(expected, produced);
        }
    }

    #[test]
    fn test_lookup_parents() {
        let (temp_media_root, media_lib) = setup("test_lookup_origin");
        let tp = temp_media_root.path();

        let inputs_and_expected = vec![
            // TODO: Need a test to demo self meta overriding item meta.
            ((tp.join("ALBUM_01").join("DISC_01"), "const_key"), Some((MetaValue::Str("const_val".to_string()), tp.join("ALBUM_01")))),
            ((tp.join("ALBUM_01").join("DISC_01"), "DISC_01_self_key"), None),
            ((tp.join("ALBUM_01").join("DISC_01"), "DISC_01_item_key"), None),
            ((tp.join("ALBUM_01").join("DISC_01"), "ALBUM_01_item_key"), Some((MetaValue::Str("ALBUM_01_item_val".to_string()), tp.join("ALBUM_01")))),
            ((tp.join("ALBUM_01").join("DISC_01"), "ALBUM_01_self_key"), Some((MetaValue::Str("ALBUM_01_self_val".to_string()), tp.join("ALBUM_01")))),
            ((tp.join("ALBUM_01").join("DISC_01"), "NON_EXISTENT_KEY"), None),
            ((tp.join("ALBUM_01").join("DISC_01"), "ROOT_item_key"), None),
            ((tp.join("ALBUM_01").join("DISC_01"), "ROOT_self_key"), Some((MetaValue::Str("ROOT_self_val".to_string()), tp.to_path_buf()))),
        ];

        for ((target_item_path, field_name), expected) in inputs_and_expected {
            let produced = lookup_parents(&media_lib, target_item_path, &LookupOptions::new(field_name)).unwrap();

            assert_eq!(expected, produced);
        }
    }
}
