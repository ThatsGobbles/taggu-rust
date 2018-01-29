use std::path::{Path, PathBuf};
use std::collections::{HashSet, HashMap};

use library::Library;
use helpers::normalize;
use metadata::{MetaValue, MetaBlock};
use error::*;

pub type MetadataCache = HashMap<PathBuf, MetaBlock>;
pub type MetaFileCache = HashMap<PathBuf, MetadataCache>;

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

pub struct LookupContext<'a> {
    media_lib: &'a Library,
    cache: MetaFileCache,
}

impl<'a> LookupContext<'a> {
    pub fn new(media_lib: &'a Library) -> LookupContext<'a> {
        LookupContext {
            media_lib,
            cache: hashmap![],
        }
    }

    pub fn lookup_origin<P: AsRef<Path>, S: AsRef<str>>(
        &mut self,
        abs_item_path: P,
        field_name: S,
        ) -> LookupResult
    {
        let abs_item_path = normalize(abs_item_path.as_ref());

        // Get meta file paths from item path.
        let meta_file_paths = self.media_lib.meta_fps_from_item_fp(&abs_item_path)?;

        for meta_file_path in meta_file_paths {
            // Ensure meta file path is cached.
            self.cache_meta_file(&meta_file_path, false)?;

            let field_result = {
                self.cache.get(&meta_file_path)
                    .and_then(|mc| mc.get(&abs_item_path))
                    .and_then(|mb| mb.get(field_name.as_ref()))
            };

            match field_result {
                Some(val) => {
                    return Ok(Some(val.clone()))
                },
                None => {
                    continue;
                }
            }
        }

        // No error, but value was not found.
        Ok(None)
    }

    pub fn lookup_parents<P: AsRef<Path>, S: AsRef<str>>(
        &mut self,
        abs_item_path: P,
        field_name: S,
        ) -> LookupResult
    {
        let mut curr_item_path = normalize(abs_item_path.as_ref());

        while let Some(curr_item_parent) = curr_item_path.parent().map(Path::to_path_buf) {
            if !self.media_lib.is_proper_sub_path(&curr_item_parent) {
                break;
            }

            match self.lookup_origin(&curr_item_parent, field_name.as_ref())? {
                Some(results) => { return Ok(Some(results)); },
                None => {},
            }

            curr_item_path = curr_item_parent;
        }

        // No error, but value was not found.
        Ok(None)
    }

    pub fn lookup_children<P: AsRef<Path>, S: AsRef<str>>(
        &mut self,
        abs_item_path: P,
        field_name: S,
        ) -> LookupResult
    {
        let curr_item_path = normalize(abs_item_path.as_ref());

        // A non-directory has no children; this is a leaf (and a base case).
        if !curr_item_path.is_dir() {
            return Ok(None);
        }

        let mut agg_results: Vec<MetaValue> = vec![];

        // println!("Calling lookup_children for: {:?}", curr_item_path);

        let field_name = field_name.as_ref();

        // Look at the metadata for each child contained in this directory, in the expected order.
        for child_abs_item_path in self.media_lib.children_paths(&curr_item_path)? {
            // println!("Checking child: {:?}", child_abs_item_path);
            // TODO: Do we want to short circuit on error here?
            let child_results = self.lookup_origin(&child_abs_item_path, field_name)?;

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
                    let sub_result = self.lookup_children(&child_abs_item_path, field_name)?;

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

    pub fn cache_meta_files<I, P>(&mut self, meta_fps: I, force: bool) -> Result<()>
    where I: IntoIterator<Item = P>,
          P: AsRef<Path>,
    {
        for meta_fp in meta_fps.into_iter() {
            let meta_fp = meta_fp.as_ref();

            // Check if the entry is already cached, and skip if cache request is not forced.
            if !force && self.cache.contains_key(meta_fp) {
                continue;
            }

            // Remove the old entry from the cache.
            // TODO: Create .remove_cached_meta_file().
            let _ = self.cache.remove(meta_fp);

            // Temporary metadata cache, filled in below.
            let mut temp: MetadataCache = hashmap![];

            for (item_fp, meta_block) in self.media_lib.item_fps_from_meta_fp(meta_fp)? {
                temp.insert(item_fp, meta_block);
            }

            self.cache.insert(meta_fp.to_path_buf(), temp);
        }

        Ok(())
    }

    pub fn cache_meta_file<P: AsRef<Path>>(&mut self, meta_fp: P, force: bool) -> Result<()> {
        self.cache_meta_files(&[meta_fp], force)
    }

    pub fn cache_item_files<I, P>(&mut self, item_fps: I, force: bool) -> Result<()>
    where I: IntoIterator<Item = P>,
          P: AsRef<Path>,
    {
        for item_fp in item_fps.into_iter() {
            let item_fp = item_fp.as_ref();

            // Get the meta files that could provide info for this item.
            // TODO: Remove duplicates.
            let mut meta_fps = self.media_lib.meta_fps_from_item_fp(&item_fp)?;

            self.cache_meta_files(&meta_fps, force)?;
        }

        Ok(())
    }

    pub fn cache_item_file<P: AsRef<Path>>(&mut self, item_fp: P, force: bool) -> Result<()> {
        self.cache_item_files(&[item_fp], force)
    }

    pub fn clear(&mut self) -> Result<()> {
        self.cache.clear();
        Ok(())
    }

    pub fn clear_meta_files<I, P>(&mut self, meta_fps: I) -> Result<()>
    where I: IntoIterator<Item = P>,
          P: AsRef<Path>,
    {
        for meta_fp in meta_fps.into_iter() {
            let meta_fp = meta_fp.as_ref();
            let _ = self.cache.remove(meta_fp);
        }

        Ok(())
    }

    pub fn clear_meta_file<P: AsRef<Path>>(&mut self, meta_fp: P) -> Result<()> {
        self.clear_meta_files(&[meta_fp])
    }

    pub fn clear_item_files<I, P>(&mut self, item_fps: I) -> Result<()>
    where I: IntoIterator<Item = P>,
          P: AsRef<Path>,
    {
        for item_fp in item_fps.into_iter() {
            let item_fp = item_fp.as_ref();

            // Get the meta files that could provide info for this item.
            // TODO: Remove duplicates.
            let mut meta_fps = self.media_lib.meta_fps_from_item_fp(&item_fp)?;

            self.clear_meta_files(&meta_fps)?;
        }

        Ok(())
    }

    pub fn clear_item_file<P: AsRef<Path>>(&mut self, item_fp: P) -> Result<()> {
        self.clear_item_files(&[item_fp])
    }

    pub fn get_meta_file<P: AsRef<Path>>(&mut self, meta_fp: P) -> Result<Option<&MetadataCache>> {
        self.cache_meta_file(&meta_fp, false)?;

        let result = self.cache.get(meta_fp.as_ref());

        Ok(result)
    }
}

pub type LookupResult = Result<Option<MetaValue>>;

// pub fn lookup_origin<P: AsRef<Path>>(
//     media_library: &Library,
//     abs_item_path: P,
//     options: &LookupOptions,
//     ) -> LookupResult
// {
//     let abs_item_path = normalize(abs_item_path.as_ref());

//     // Get meta file paths from item path.
//     let meta_file_paths = media_library.meta_fps_from_item_fp(&abs_item_path)?;

//     'meta: for meta_file_path in meta_file_paths {
//         // Open this meta file path and see if it contains the field we are looking for.
//         let records = media_library.item_fps_from_meta_fp(&meta_file_path)?;

//         // Search found item paths for a match to target item path.
//         'item: for (found_item_path, found_meta_block) in records {
//             if abs_item_path == found_item_path {
//                 // Found a match for this path, check if the desired field is contained in meta block.
//                 match found_meta_block.get(&options.field_name) {
//                     Some(val) => {
//                         return Ok(Some(val.clone()))
//                     },
//                     None => {
//                         continue 'item;
//                     }
//                 }
//             }
//         }
//     }

//     // No error, but value was not found.
//     Ok(None)
// }

// pub fn lookup_parents<P: AsRef<Path>>(
//     media_library: &Library,
//     abs_item_path: P,
//     options: &LookupOptions,
//     ) -> LookupResult
// {
//     let mut curr_item_path = normalize(abs_item_path.as_ref());

//     while let Some(curr_item_parent) = curr_item_path.parent().map(Path::to_path_buf) {
//         if !media_library.is_proper_sub_path(&curr_item_parent) {
//             break;
//         }

//         match lookup_origin(media_library, &curr_item_parent, options)? {
//             Some(results) => { return Ok(Some(results)); },
//             None => {},
//         }

//         curr_item_path = curr_item_parent;
//     }

//     // No error, but value was not found.
//     Ok(None)
// }

// pub fn lookup_children<P: AsRef<Path>>(
//     media_library: &Library,
//     abs_item_path: P,
//     options: &LookupOptions,
//     ) -> LookupResult
// {
//     let curr_item_path = normalize(abs_item_path.as_ref());

//     // A non-directory has no children; this is a leaf (and a base case).
//     if !curr_item_path.is_dir() {
//         return Ok(None);
//     }

//     let mut agg_results: Vec<MetaValue> = vec![];

//     // println!("Calling lookup_children for: {:?}", curr_item_path);

//     // Look at the metadata for each child contained in this directory, in the expected order.
//     for child_abs_item_path in media_library.children_paths(&curr_item_path)? {
//         // println!("Checking child: {:?}", child_abs_item_path);
//         // TODO: Do we want to short circuit on error here?
//         let child_results = lookup_origin(media_library, &child_abs_item_path, options)?;

//         match child_results {
//             Some(ref child_values) => {
//                 // println!("Found result: {:?}", child_results.clone());
//                 // Found the value, add it to the results and do not recurse further on this path.
//                 agg_results.push(child_values.clone());
//             },
//             None => {
//                 // println!("Not found here, trying subchildren");
//                 // Recurse down this path.
//                 // Note that this will produce a list.
//                 let sub_result = lookup_children(media_library, &child_abs_item_path, options)?;

//                 match sub_result {
//                     Some(sub_values) => { agg_results.push(sub_values); },
//                     None => {
//                         // println!("Not found at all");
//                         // TODO: Do nothing, or return null here?
//                         // Do nothing, this is a hole in the aggregation.
//                     },
//                 }
//             }
//         }
//     }

//     // TODO: If an enpty list would be returned, would it be better to return None?
//     Ok(Some(MetaValue::Seq(agg_results)))
// }

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::collections::HashSet;
    use std::thread::sleep;
    use std::time::Duration;

    use tempdir::TempDir;

    use super::{LookupContext, MetadataCache, MetaFileCache};
    use library::{Library, LibraryBuilder};
    use library::selection::Selection;
    use metadata::MetaValue;
    use metadata::target::MetaTarget;
    use test_helpers::{create_temp_media_test_dir, default_setup};

//     #[test]
//     fn test_lookup_origin() {
//         let (temp_media_root, media_lib) = default_setup("test_lookup_origin");
//         let tp = temp_media_root.path();

//         let inputs_and_expected = vec![
//             ((tp.join("ALBUM_01").join("DISC_01"), "const_key"), Some(MetaValue::Str("const_val".to_string()))),
//             ((tp.join("ALBUM_01").join("DISC_01"), "DISC_01_self_key"), Some(MetaValue::Str("DISC_01_self_val".to_string()))),
//             ((tp.join("ALBUM_01").join("DISC_01"), "DISC_01_item_key"), Some(MetaValue::Str("DISC_01_item_val".to_string()))),
//             ((tp.join("ALBUM_01").join("DISC_01"), "ALBUM_01_item_key"), None),
//             ((tp.join("ALBUM_01").join("DISC_01"), "ALBUM_01_self_key"), None),
//             ((tp.join("ALBUM_01").join("DISC_01"), "NON_EXISTENT_KEY"), None),
//             ((tp.to_path_buf(), "ROOT_item_key"), None),
//             ((tp.to_path_buf(), "ROOT_self_key"), Some(MetaValue::Str("ROOT_self_val".to_string()))),
//         ];

//         for ((target_item_path, field_name), expected) in inputs_and_expected {
//             let produced = lookup_origin(&media_lib, target_item_path, &LookupOptions::new(field_name)).unwrap();

//             assert_eq!(expected, produced);
//         }
//     }

//     #[test]
//     fn test_lookup_parents() {
//         let (temp_media_root, media_lib) = default_setup("test_lookup_origin");
//         let tp = temp_media_root.path();

//         let inputs_and_expected = vec![
//             // TODO: Need a test to demo self meta overriding item meta.
//             ((tp.join("ALBUM_01").join("DISC_01"), "const_key"), Some(MetaValue::Str("const_val".to_string()))),
//             ((tp.join("ALBUM_01").join("DISC_01"), "DISC_01_self_key"), None),
//             ((tp.join("ALBUM_01").join("DISC_01"), "DISC_01_item_key"), None),
//             ((tp.join("ALBUM_01").join("DISC_01"), "ALBUM_01_item_key"), Some(MetaValue::Str("ALBUM_01_item_val".to_string()))),
//             ((tp.join("ALBUM_01").join("DISC_01"), "ALBUM_01_self_key"), Some(MetaValue::Str("ALBUM_01_self_val".to_string()))),
//             ((tp.join("ALBUM_01").join("DISC_01"), "NON_EXISTENT_KEY"), None),
//             ((tp.join("ALBUM_01").join("DISC_01"), "ROOT_item_key"), None),
//             ((tp.join("ALBUM_01").join("DISC_01"), "ROOT_self_key"), Some(MetaValue::Str("ROOT_self_val".to_string()))),
//         ];

//         for ((target_item_path, field_name), expected) in inputs_and_expected {
//             let produced = lookup_parents(&media_lib, target_item_path, &LookupOptions::new(field_name)).unwrap();

//             assert_eq!(expected, produced);
//         }
//     }

//     #[test]
//     fn test_lookup_children() {
//         let (temp_media_root, media_lib) = default_setup("test_lookup_children");
//         let tp = temp_media_root.path();

//         let inputs_and_expected = vec![
//             (
//                 (tp.join("ALBUM_01"), "const_key"),
//                 Some(
//                     MetaValue::Seq(vec![
//                         MetaValue::Str("const_val".to_string()),
//                         MetaValue::Str("const_val".to_string()),
//                     ]),
//                 ),
//             ),
//             (
//                 (tp.join("ALBUM_01"), "TRACK_01_item_key"),
//                 Some(
//                     MetaValue::Seq(vec![
//                         MetaValue::Seq(vec![MetaValue::Str("TRACK_01_item_val".to_string())]),
//                         MetaValue::Seq(vec![MetaValue::Str("TRACK_01_item_val".to_string())]),
//                     ]),
//                 ),
//             ),
//         ];

//         for ((target_item_path, field_name), expected) in inputs_and_expected {
//             let produced = lookup_children(&media_lib, target_item_path, &LookupOptions::new(field_name)).unwrap();

//             assert_eq!(expected, produced);
//         }

//         // let produced = lookup_children(&media_lib, tp.join("ALBUM_01"), &LookupOptions::new("const_key")).unwrap();
//     }

    fn extract_all_meta_fps(raw_cache: &MetaFileCache) -> HashSet<PathBuf> {
        raw_cache.keys().into_iter().cloned().collect()
    }

    fn extract_all_item_fps(raw_cache: &MetaFileCache) -> HashSet<PathBuf> {
        raw_cache
            .values()
            .into_iter()
            .flat_map(|item_fp_to_mb| item_fp_to_mb.keys())
            .cloned()
            .collect()
    }

    fn extract_sub_item_fps<P: AsRef<Path>>(raw_cache: &MetaFileCache, meta_fp: P) -> HashSet<PathBuf> {
        let meta_fp = meta_fp.as_ref();

        raw_cache
            .get(meta_fp)
            .expect("key not found in cache")
            .keys()
            .into_iter()
            .cloned()
            .collect()
    }

    enum EqualityTarget {
        AllMetas,
        AllItems,
        SubItems(PathBuf),
    }

    fn test_fp_set_equality(expected_fps: HashSet<PathBuf>, raw_cache: &MetaFileCache, target: EqualityTarget) {
        let produced_fps = match target {
            EqualityTarget::AllMetas => extract_all_meta_fps(raw_cache),
            EqualityTarget::AllItems => extract_all_item_fps(raw_cache),
            EqualityTarget::SubItems(ref meta_fp) => extract_sub_item_fps(raw_cache, &meta_fp),
        };

        assert_eq!(expected_fps, produced_fps);
    }

    #[test]
    fn test_new() {
        let (temp_media_root, media_lib) = default_setup("test_new");
        let tp = temp_media_root.path();

        let mut lookup_ctx = LookupContext::new(&media_lib);

        assert!(lookup_ctx.cache.is_empty());
    }

    #[test]
    fn test_cache_meta_file() {
        let (temp_media_root, media_lib) = default_setup("test_cache_meta_file");
        let tp = temp_media_root.path();

        let mut lookup_ctx = LookupContext::new(&media_lib);

        let meta_fp = tp.join("ALBUM_01").join("item.yml");
        lookup_ctx.cache_meta_file(&meta_fp, false).expect("unable to cache meta file");

        let expected_meta_fps = hashset![
            tp.join("ALBUM_01").join("item.yml"),
        ];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);

        let expected_item_fps = hashset![
            tp.join("ALBUM_01").join("DISC_01"),
            tp.join("ALBUM_01").join("DISC_02"),
        ];
        let produced_item_fps: HashSet<PathBuf> = extract_sub_item_fps(&lookup_ctx.cache, &meta_fp);
        assert_eq!(expected_item_fps, produced_item_fps);

        let meta_fp = tp.join("ALBUM_01").join("item.yml");
        lookup_ctx.cache_meta_file(&meta_fp, false).expect("unable to cache meta file");

        let expected_meta_fps = hashset![
            tp.join("ALBUM_01").join("item.yml"),
        ];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);

        let expected_item_fps = hashset![
            tp.join("ALBUM_01").join("DISC_01"),
            tp.join("ALBUM_01").join("DISC_02"),
        ];
        let produced_item_fps: HashSet<PathBuf> = extract_sub_item_fps(&lookup_ctx.cache, &meta_fp);
        assert_eq!(expected_item_fps, produced_item_fps);

        let meta_fp = tp.join("ALBUM_01").join("self.yml");
        lookup_ctx.cache_meta_file(&meta_fp, false).expect("unable to cache meta file");

        let expected_meta_fps = hashset![
            tp.join("ALBUM_01").join("item.yml"),
            tp.join("ALBUM_01").join("self.yml"),
        ];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);

        let expected_item_fps = hashset![
            tp.join("ALBUM_01"),
        ];
        let produced_item_fps: HashSet<PathBuf> = extract_sub_item_fps(&lookup_ctx.cache, &meta_fp);
        assert_eq!(expected_item_fps, produced_item_fps);
    }

    #[test]
    fn test_cache_item_file() {
        let (temp_media_root, media_lib) = default_setup("test_cache_item_file");
        let tp = temp_media_root.path();

        let mut lookup_ctx = LookupContext::new(&media_lib);

        let item_fp = tp.join("ALBUM_01").join("DISC_01");
        lookup_ctx.cache_item_file(&item_fp, false).expect("unable to cache item file");

        let expected_meta_fps = hashset![
            tp.join("ALBUM_01").join("item.yml"),
            tp.join("ALBUM_01").join("DISC_01").join("self.yml"),
        ];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);

        let expected_item_fps = hashset![
            tp.join("ALBUM_01").join("DISC_01"),
            // All item files pointed to by the item's meta file are cached.
            tp.join("ALBUM_01").join("DISC_02"),
        ];
        let produced_item_fps: HashSet<PathBuf> = extract_all_item_fps(&lookup_ctx.cache);
        assert_eq!(expected_item_fps, produced_item_fps);

        let item_fp = tp.join("ALBUM_01").join("DISC_02");
        lookup_ctx.cache_item_file(&item_fp, false).expect("unable to cache item file");

        let expected_meta_fps = hashset![
            // This should already be present from the first lookup.
            tp.join("ALBUM_01").join("item.yml"),
            tp.join("ALBUM_01").join("DISC_01").join("self.yml"),
            tp.join("ALBUM_01").join("DISC_02").join("self.yml"),
        ];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);

        let expected_item_fps = hashset![
            tp.join("ALBUM_01").join("DISC_01"),
            // All item files pointed to by the item's meta file are cached.
            tp.join("ALBUM_01").join("DISC_02"),
        ];
        let produced_item_fps: HashSet<PathBuf> = extract_all_item_fps(&lookup_ctx.cache);
        assert_eq!(expected_item_fps, produced_item_fps);

        let item_fp = tp.join("ALBUM_01").join("DISC_01").join("TRACK_01.flac");
        lookup_ctx.cache_item_file(&item_fp, false).expect("unable to cache item file");

        let expected_meta_fps = hashset![
            // This should already be present from the first lookup.
            tp.join("ALBUM_01").join("item.yml"),
            tp.join("ALBUM_01").join("DISC_01").join("self.yml"),
            tp.join("ALBUM_01").join("DISC_01").join("item.yml"),
            tp.join("ALBUM_01").join("DISC_02").join("self.yml"),
        ];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);

        let expected_item_fps = hashset![
            tp.join("ALBUM_01").join("DISC_01"),
            // All item files pointed to by the item's meta file are cached.
            tp.join("ALBUM_01").join("DISC_01").join("TRACK_01.flac"),
            tp.join("ALBUM_01").join("DISC_01").join("TRACK_02.flac"),
            tp.join("ALBUM_01").join("DISC_01").join("TRACK_03.flac"),
            tp.join("ALBUM_01").join("DISC_02"),
        ];
        let produced_item_fps: HashSet<PathBuf> = extract_all_item_fps(&lookup_ctx.cache);
        assert_eq!(expected_item_fps, produced_item_fps);
    }

    #[test]
    fn test_clear() {
        let (temp_media_root, media_lib) = default_setup("test_clear");
        let tp = temp_media_root.path();

        let mut lookup_ctx = LookupContext::new(&media_lib);

        lookup_ctx.cache_item_file(tp.join("ALBUM_01"), false).expect("unable to cache item file");
        lookup_ctx.cache_item_file(tp.join("ALBUM_02"), false).expect("unable to cache item file");
        lookup_ctx.cache_item_file(tp.join("ALBUM_03"), false).expect("unable to cache item file");
        lookup_ctx.cache_item_file(tp.join("ALBUM_05"), false).expect("unable to cache item file");

        let expected_meta_fps = hashset![
            tp.join("item.yml"),
            tp.join("ALBUM_01").join("self.yml"),
            tp.join("ALBUM_02").join("self.yml"),
            tp.join("ALBUM_03").join("self.yml"),
            tp.join("ALBUM_05").join("self.yml"),
        ];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);

        lookup_ctx.clear().expect("unable to clear cache");

        let expected_meta_fps = hashset![];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);
    }
}
