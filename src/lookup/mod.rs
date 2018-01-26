pub mod cacher;

use std::path::{Path, PathBuf};
use std::collections::{HashSet, HashMap};

use library::Library;
use helpers::normalize;
use metadata::{MetaValue, MetaBlock};
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

// pub struct LookupContext<'a> {
//     media_lib: &'a Library,
//     cache: MetaFileCache,
// }

// impl<'a> LookupContext<'a> {
//     pub fn new(media_lib: &'a Library) -> LookupContext<'a> {
//         LookupContext {
//             media_lib,
//             cache: hashmap![],
//         }
//     }

//     fn cache_meta_files<I, P>(&mut self, meta_fps: I, force: bool) -> Result<()>
//     where I: IntoIterator<Item = P>,
//           P: AsRef<Path>,
//     {
//         for meta_fp in meta_fps.into_iter() {
//             let meta_fp = meta_fp.as_ref();

//             // Check if the entry is already cached, and skip if cache request is not forced.
//             if !force && self.cache.contains_key(meta_fp) {
//                 continue;
//             }

//             // Remove the old entry from the cache.
//             // TODO: Create .remove_cached_meta_file().
//             let _ = self.cache.remove(meta_fp);

//             // Temporary metadata cache, filled in below.
//             let mut temp: MetadataCache = hashmap![];

//             for (item_fp, meta_block) in self.media_lib.item_fps_from_meta_fp(meta_fp)? {
//                 temp.insert(item_fp, meta_block);
//             }

//             self.cache.insert(meta_fp.to_path_buf(), temp);
//         }

//         Ok(())
//     }

//     fn cache_meta_file<P: AsRef<Path>>(&mut self, meta_fp: P, force: bool) -> Result<()> {
//         self.cache_meta_files(&[meta_fp], force)
//     }

//     fn cache_item_files<I, P>(&mut self, item_fps: I, force: bool) -> Result<()>
//     where I: IntoIterator<Item = P>,
//           P: AsRef<Path>,
//     {
//         for item_fp in item_fps.into_iter() {
//             let item_fp = item_fp.as_ref();

//             // Get the meta files that could provide info for this item.
//             // TODO: Remove duplicates.
//             let mut meta_fps = self.media_lib.meta_fps_from_item_fp(&item_fp)?;

//             self.cache_meta_files(&meta_fps, force)?;
//         }

//         Ok(())
//     }

//     fn cache_item_file<P: AsRef<Path>>(&mut self, item_fp: P, force: bool) -> Result<()> {
//         self.cache_item_files(&[item_fp], force)
//     }

//     fn clear_cache(&mut self) {
//         self.cache.clear();
//     }
// }

pub type LookupResult = Result<Option<MetaValue>>;

pub fn lookup_origin<P: AsRef<Path>>(
    media_library: &Library,
    abs_item_path: P,
    options: &LookupOptions,
    ) -> LookupResult
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
                        return Ok(Some(val.clone()))
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
    media_library: &Library,
    abs_item_path: P,
    options: &LookupOptions,
    ) -> LookupResult
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
    media_library: &Library,
    abs_item_path: P,
    options: &LookupOptions,
    ) -> LookupResult
{
    let curr_item_path = normalize(abs_item_path.as_ref());

    // A non-directory has no children; this is a leaf (and a base case).
    if !curr_item_path.is_dir() {
        return Ok(None);
    }

    let mut agg_results: Vec<MetaValue> = vec![];

    // println!("Calling lookup_children for: {:?}", curr_item_path);

    // Look at the metadata for each child contained in this directory, in the expected order.
    for child_abs_item_path in media_library.children_paths(&curr_item_path)? {
        // println!("Checking child: {:?}", child_abs_item_path);
        // TODO: Do we want to short circuit on error here?
        let child_results = lookup_origin(media_library, &child_abs_item_path, options)?;

        match child_results {
            Some(ref child_values) => {
                // println!("Found result: {:?}", child_results.clone());
                // Found the value, add it to the results and do not recurse further on this path.
                agg_results.push(child_values.clone());
            },
            None => {
                // println!("Not found here, trying subchildren");
                // Recurse down this path.
                // Note that this will produce a list.
                let sub_result = lookup_children(media_library, &child_abs_item_path, options)?;

                match sub_result {
                    Some(sub_values) => { agg_results.push(sub_values); },
                    None => {
                        // println!("Not found at all");
                        // TODO: Do nothing, or return null here?
                        // Do nothing, this is a hole in the aggregation.
                    },
                }
            }
        }
    }

    // TODO: If an enpty list would be returned, would it be better to return None?
    Ok(Some(MetaValue::Seq(agg_results)))
}

#[cfg(test)]
mod tests {
    use std::thread::sleep;
    use std::time::Duration;

    use tempdir::TempDir;

    use super::{lookup_origin, lookup_parents, lookup_children, LookupOptions};
    use library::{Library, LibraryBuilder};
    use library::selection::Selection;
    use metadata::MetaValue;
    use metadata::target::MetaTarget;
    use test_helpers::{create_temp_media_test_dir, default_setup};

    #[test]
    fn test_lookup_origin() {
        let (temp_media_root, media_lib) = default_setup("test_lookup_origin");
        let tp = temp_media_root.path();

        let inputs_and_expected = vec![
            ((tp.join("ALBUM_01").join("DISC_01"), "const_key"), Some(MetaValue::Str("const_val".to_string()))),
            ((tp.join("ALBUM_01").join("DISC_01"), "DISC_01_self_key"), Some(MetaValue::Str("DISC_01_self_val".to_string()))),
            ((tp.join("ALBUM_01").join("DISC_01"), "DISC_01_item_key"), Some(MetaValue::Str("DISC_01_item_val".to_string()))),
            ((tp.join("ALBUM_01").join("DISC_01"), "ALBUM_01_item_key"), None),
            ((tp.join("ALBUM_01").join("DISC_01"), "ALBUM_01_self_key"), None),
            ((tp.join("ALBUM_01").join("DISC_01"), "NON_EXISTENT_KEY"), None),
            ((tp.to_path_buf(), "ROOT_item_key"), None),
            ((tp.to_path_buf(), "ROOT_self_key"), Some(MetaValue::Str("ROOT_self_val".to_string()))),
        ];

        for ((target_item_path, field_name), expected) in inputs_and_expected {
            let produced = lookup_origin(&media_lib, target_item_path, &LookupOptions::new(field_name)).unwrap();

            assert_eq!(expected, produced);
        }
    }

    #[test]
    fn test_lookup_parents() {
        let (temp_media_root, media_lib) = default_setup("test_lookup_origin");
        let tp = temp_media_root.path();

        let inputs_and_expected = vec![
            // TODO: Need a test to demo self meta overriding item meta.
            ((tp.join("ALBUM_01").join("DISC_01"), "const_key"), Some(MetaValue::Str("const_val".to_string()))),
            ((tp.join("ALBUM_01").join("DISC_01"), "DISC_01_self_key"), None),
            ((tp.join("ALBUM_01").join("DISC_01"), "DISC_01_item_key"), None),
            ((tp.join("ALBUM_01").join("DISC_01"), "ALBUM_01_item_key"), Some(MetaValue::Str("ALBUM_01_item_val".to_string()))),
            ((tp.join("ALBUM_01").join("DISC_01"), "ALBUM_01_self_key"), Some(MetaValue::Str("ALBUM_01_self_val".to_string()))),
            ((tp.join("ALBUM_01").join("DISC_01"), "NON_EXISTENT_KEY"), None),
            ((tp.join("ALBUM_01").join("DISC_01"), "ROOT_item_key"), None),
            ((tp.join("ALBUM_01").join("DISC_01"), "ROOT_self_key"), Some(MetaValue::Str("ROOT_self_val".to_string()))),
        ];

        for ((target_item_path, field_name), expected) in inputs_and_expected {
            let produced = lookup_parents(&media_lib, target_item_path, &LookupOptions::new(field_name)).unwrap();

            assert_eq!(expected, produced);
        }
    }

    #[test]
    fn test_lookup_children() {
        let (temp_media_root, media_lib) = default_setup("test_lookup_children");
        let tp = temp_media_root.path();

        let inputs_and_expected = vec![
            (
                (tp.join("ALBUM_01"), "const_key"),
                Some(
                    MetaValue::Seq(vec![
                        MetaValue::Str("const_val".to_string()),
                        MetaValue::Str("const_val".to_string()),
                    ]),
                ),
            ),
            (
                (tp.join("ALBUM_01"), "TRACK_01_item_key"),
                Some(
                    MetaValue::Seq(vec![
                        MetaValue::Seq(vec![MetaValue::Str("TRACK_01_item_val".to_string())]),
                        MetaValue::Seq(vec![MetaValue::Str("TRACK_01_item_val".to_string())]),
                    ]),
                ),
            ),
        ];

        for ((target_item_path, field_name), expected) in inputs_and_expected {
            let produced = lookup_children(&media_lib, target_item_path, &LookupOptions::new(field_name)).unwrap();

            assert_eq!(expected, produced);
        }

        // let produced = lookup_children(&media_lib, tp.join("ALBUM_01"), &LookupOptions::new("const_key")).unwrap();
    }
}
