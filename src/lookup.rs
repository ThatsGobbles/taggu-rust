// use std::path::{Path, PathBuf};
// use std::collections::HashMap;

// use library::Library;
// use helpers::normalize;
// use metadata::{MetaValue, MetaBlock};
// use error::*;

// pub type MetadataCache = HashMap<PathBuf, MetaBlock>;
// pub type MetaFileCache = HashMap<PathBuf, MetadataCache>;

// trait LabelExtractor {
//     fn extract_label<S: AsRef<str>>(&self, item_file_name: S) -> String;
// }

// pub type LookupResult = Result<Option<MetaValue>>;

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

//     pub fn lookup_origin<P: AsRef<Path>, S: AsRef<str>>(
//         &mut self,
//         abs_item_path: P,
//         field_name: S,
//         ) -> LookupResult
//     {
//         let abs_item_path = normalize(abs_item_path.as_ref());

//         // Get meta file paths from item path.
//         let meta_file_paths = self.media_lib.meta_fps_from_item_fp(&abs_item_path)?;

//         for meta_file_path in meta_file_paths {
//             // Ensure meta file path is cached.
//             self.cache_meta_file(&meta_file_path, false)?;

//             let field_result = {
//                 self.cache.get(&meta_file_path)
//                     .and_then(|mc| mc.get(&abs_item_path))
//                     .and_then(|mb| mb.get(field_name.as_ref()))
//             };

//             match field_result {
//                 Some(val) => { return Ok(Some(val.clone())); },
//                 None => { continue; }
//             }
//         }

//         // No error, but value was not found.
//         Ok(None)
//     }

//     pub fn lookup_parents<P: AsRef<Path>, S: AsRef<str>>(
//         &mut self,
//         abs_item_path: P,
//         field_name: S,
//         ) -> LookupResult
//     {
//         let mut curr_item_path = normalize(abs_item_path.as_ref());

//         while let Some(curr_item_parent) = curr_item_path.parent().map(Path::to_path_buf) {
//             if !self.media_lib.is_proper_sub_path(&curr_item_parent) {
//                 break;
//             }

//             match self.lookup_origin(&curr_item_parent, field_name.as_ref())? {
//                 Some(results) => { return Ok(Some(results)); },
//                 None => {},
//             }

//             curr_item_path = curr_item_parent;
//         }

//         // No error, but value was not found.
//         Ok(None)
//     }

//     pub fn lookup_children<P: AsRef<Path>, S: AsRef<str>>(
//         &mut self,
//         abs_item_path: P,
//         field_name: S,
//         ) -> LookupResult
//     {
//         let curr_item_path = normalize(abs_item_path.as_ref());

//         // A non-directory has no children; this is a leaf (and a base case).
//         if !curr_item_path.is_dir() {
//             return Ok(None);
//         }

//         let mut agg_results: Vec<MetaValue> = vec![];

//         // println!("Calling lookup_children for: {:?}", curr_item_path);

//         let field_name = field_name.as_ref();

//         // Look at the metadata for each child contained in this directory, in the expected order.
//         for child_abs_item_path in self.media_lib.children_paths(&curr_item_path)? {
//             // println!("Checking child: {:?}", child_abs_item_path);
//             // TODO: Do we want to short circuit on error here?
//             let child_results = self.lookup_origin(&child_abs_item_path, field_name)?;

//             match child_results {
//                 Some(ref child_values) => {
//                     // println!("Found result: {:?}", child_results.clone());
//                     // Found the value, add it to the results and do not recurse further on this path.
//                     agg_results.push(child_values.clone());
//                 },
//                 None => {
//                     // println!("Not found here, trying subchildren");
//                     // Recurse down this path.
//                     // Note that this will produce a list.
//                     let sub_result = self.lookup_children(&child_abs_item_path, field_name)?;

//                     match sub_result {
//                         Some(sub_values) => { agg_results.push(sub_values); },
//                         None => {
//                             // println!("Not found at all");
//                             // TODO: Do nothing, or return null here?
//                             // Do nothing, this is a hole in the aggregation.
//                         },
//                     }
//                 }
//             }
//         }

//         // TODO: If an enpty list would be returned, would it be better to return None?
//         Ok(Some(MetaValue::Seq(agg_results)))
//     }

//     pub fn cache_meta_files<I, P>(&mut self, meta_fps: I, force: bool) -> Result<()>
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

//     pub fn cache_meta_file<P: AsRef<Path>>(&mut self, meta_fp: P, force: bool) -> Result<()> {
//         self.cache_meta_files(&[meta_fp], force)
//     }

//     pub fn cache_item_files<I, P>(&mut self, item_fps: I, force: bool) -> Result<()>
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

//     pub fn cache_item_file<P: AsRef<Path>>(&mut self, item_fp: P, force: bool) -> Result<()> {
//         self.cache_item_files(&[item_fp], force)
//     }

//     pub fn clear(&mut self) {
//         self.cache.clear();
//     }

//     pub fn clear_meta_files<I, P>(&mut self, meta_fps: I) -> Result<()>
//     where I: IntoIterator<Item = P>,
//           P: AsRef<Path>,
//     {
//         for meta_fp in meta_fps.into_iter() {
//             let meta_fp = meta_fp.as_ref();
//             let _ = self.cache.remove(meta_fp);
//         }

//         Ok(())
//     }

//     pub fn clear_meta_file<P: AsRef<Path>>(&mut self, meta_fp: P) -> Result<()> {
//         self.clear_meta_files(&[meta_fp])
//     }

//     // pub fn clear_item_files<I, P>(&mut self, item_fps: I) -> Result<()>
//     // where I: IntoIterator<Item = P>,
//     //       P: AsRef<Path>,
//     // {
//     //     for item_fp in item_fps.into_iter() {
//     //         let item_fp = item_fp.as_ref();

//     //         // Get the meta files that could provide info for this item.
//     //         // TODO: Remove duplicates.
//     //         let mut meta_fps = self.media_lib.meta_fps_from_item_fp(&item_fp)?;

//     //         self.clear_meta_files(&meta_fps)?;
//     //     }

//     //     Ok(())
//     // }

//     // pub fn clear_item_file<P: AsRef<Path>>(&mut self, item_fp: P) -> Result<()> {
//     //     self.clear_item_files(&[item_fp])
//     // }
// }

// #[cfg(test)]
// mod tests {
//     use std::path::{Path, PathBuf};
//     use std::collections::HashSet;

//     use super::{LookupContext, MetaFileCache};
//     use metadata::MetaValue;
//     use test_helpers::default_setup;

//     fn extract_all_meta_fps(raw_cache: &MetaFileCache) -> HashSet<PathBuf> {
//         raw_cache.keys().into_iter().cloned().collect()
//     }

//     fn extract_all_item_fps(raw_cache: &MetaFileCache) -> HashSet<PathBuf> {
//         raw_cache
//             .values()
//             .into_iter()
//             .flat_map(|item_fp_to_mb| item_fp_to_mb.keys())
//             .cloned()
//             .collect()
//     }

//     fn extract_sub_item_fps<P: AsRef<Path>>(raw_cache: &MetaFileCache, meta_fp: P) -> HashSet<PathBuf> {
//         let meta_fp = meta_fp.as_ref();

//         raw_cache
//             .get(meta_fp)
//             .expect("key not found in cache")
//             .keys()
//             .into_iter()
//             .cloned()
//             .collect()
//     }

//     // enum EqualityTarget {
//     //     AllMetas,
//     //     AllItems,
//     //     SubItems(PathBuf),
//     // }

//     // fn test_fp_set_equality(expected_fps: HashSet<PathBuf>, raw_cache: &MetaFileCache, target: EqualityTarget) {
//     //     let produced_fps = match target {
//     //         EqualityTarget::AllMetas => extract_all_meta_fps(raw_cache),
//     //         EqualityTarget::AllItems => extract_all_item_fps(raw_cache),
//     //         EqualityTarget::SubItems(ref meta_fp) => extract_sub_item_fps(raw_cache, &meta_fp),
//     //     };

//     //     assert_eq!(expected_fps, produced_fps);
//     // }

//     #[test]
//     fn test_new() {
//         let (_, media_lib) = default_setup("test_new");

//         let lookup_ctx = LookupContext::new(&media_lib);

//         assert!(lookup_ctx.cache.is_empty());
//     }

//     #[test]
//     fn test_lookup_origin() {
//         let (temp_media_root, media_lib) = default_setup("test_lookup_origin");
//         let tp = temp_media_root.path();

//         let mut lookup_ctx = LookupContext::new(&media_lib);

//         let item_fp = tp.join("ALBUM_01").join("DISC_01");
//         let expected = Some(MetaValue::Str("const_val".to_string()));
//         let produced = lookup_ctx.lookup_origin(&item_fp, "const_key").expect("Unable to perform lookup");
//         assert_eq!(expected, produced);

//         let expected_meta_fps = hashset![
//             // The item file will never be searched!
//             // tp.join("ALBUM_01").join("item.yml"),
//             tp.join("ALBUM_01").join("DISC_01").join("self.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         lookup_ctx.clear();

//         let item_fp = tp.join("ALBUM_01").join("DISC_01");
//         let expected = Some(MetaValue::Str("item_val".to_string()));
//         let produced = lookup_ctx.lookup_origin(&item_fp, "item_key").expect("Unable to perform lookup");
//         assert_eq!(expected, produced);

//         let expected_meta_fps = hashset![
//             tp.join("ALBUM_01").join("item.yml"),
//             // This gets cached and checked first, but does not contain the field.
//             tp.join("ALBUM_01").join("DISC_01").join("self.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         lookup_ctx.clear();

//         let item_fp = tp.join("ALBUM_01").join("DISC_01");
//         let expected = None;
//         let produced = lookup_ctx.lookup_origin(&item_fp, "NON_EXISTENT_FIELD").expect("Unable to perform lookup");
//         assert_eq!(expected, produced);

//         let expected_meta_fps = hashset![
//             tp.join("ALBUM_01").join("item.yml"),
//             tp.join("ALBUM_01").join("DISC_01").join("self.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);
//     }

//     #[test]
//     fn test_lookup_parents() {
//         let (temp_media_root, media_lib) = default_setup("test_lookup_parents");
//         let tp = temp_media_root.path();

//         let mut lookup_ctx = LookupContext::new(&media_lib);

//         let item_fp = tp.join("ALBUM_01").join("DISC_01");
//         let expected = Some(MetaValue::Str("const_val".to_string()));
//         let produced = lookup_ctx.lookup_parents(&item_fp, "const_key").expect("Unable to perform lookup");
//         assert_eq!(expected, produced);

//         let expected_meta_fps = hashset![
//             // The item file will never be searched!
//             // tp.join("item.yml"),
//             tp.join("ALBUM_01").join("self.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         lookup_ctx.clear();

//         let item_fp = tp.join("ALBUM_01").join("DISC_01");
//         let expected = Some(MetaValue::Str("item_val".to_string()));
//         let produced = lookup_ctx.lookup_parents(&item_fp, "item_key").expect("Unable to perform lookup");
//         assert_eq!(expected, produced);

//         let expected_meta_fps = hashset![
//             tp.join("ALBUM_01").join("self.yml"),
//             tp.join("item.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         lookup_ctx.clear();

//         let item_fp = tp.join("ALBUM_01").join("DISC_01");
//         let expected = None;
//         let produced = lookup_ctx.lookup_parents(&item_fp, "NON_EXISTENT_FIELD").expect("Unable to perform lookup");
//         assert_eq!(expected, produced);

//         let expected_meta_fps = hashset![
//             tp.join("ALBUM_01").join("self.yml"),
//             tp.join("item.yml"),
//             tp.join("self.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);
//     }

//     #[test]
//     fn test_lookup_children() {
//         let (temp_media_root, media_lib) = default_setup("test_lookup_children");
//         let tp = temp_media_root.path();

//         let mut lookup_ctx = LookupContext::new(&media_lib);

//         let item_fp = tp.join("ALBUM_01");
//         let expected = Some(MetaValue::Seq(vec![
//             MetaValue::Str("const_val".to_string()),
//             MetaValue::Str("const_val".to_string()),
//         ]));
//         let produced = lookup_ctx.lookup_children(&item_fp, "const_key").expect("Unable to perform lookup");
//         assert_eq!(expected, produced);

//         let expected_meta_fps = hashset![
//             // Note that self.yaml is accessed, but not item.yml.
//             // This is due to having "upward" meta target precedence for each child.
//             tp.join("ALBUM_01").join("DISC_01").join("self.yml"),
//             tp.join("ALBUM_01").join("DISC_02").join("self.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         lookup_ctx.clear();

//         let item_fp = tp.join("ALBUM_01");
//         let expected = Some(MetaValue::Seq(vec![
//             MetaValue::Seq(vec![
//                 MetaValue::Str("TRACK_01_item_val".to_string()),
//             ]),
//             MetaValue::Seq(vec![
//                 MetaValue::Str("TRACK_01_item_val".to_string()),
//             ]),
//         ]));
//         let produced = lookup_ctx.lookup_children(&item_fp, "TRACK_01_item_key").expect("Unable to perform lookup");
//         assert_eq!(expected, produced);

//         let expected_meta_fps = hashset![
//             tp.join("ALBUM_01").join("item.yml"),
//             tp.join("ALBUM_01").join("DISC_01").join("self.yml"),
//             tp.join("ALBUM_01").join("DISC_01").join("item.yml"),
//             tp.join("ALBUM_01").join("DISC_02").join("self.yml"),
//             tp.join("ALBUM_01").join("DISC_02").join("item.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);
//     }

//     #[test]
//     fn test_cache_meta_file() {
//         let (temp_media_root, media_lib) = default_setup("test_cache_meta_file");
//         let tp = temp_media_root.path();

//         let mut lookup_ctx = LookupContext::new(&media_lib);

//         let meta_fp = tp.join("ALBUM_01").join("item.yml");
//         lookup_ctx.cache_meta_file(&meta_fp, false).expect("Unable to cache meta file");

//         let expected_meta_fps = hashset![
//             tp.join("ALBUM_01").join("item.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         let expected_item_fps = hashset![
//             tp.join("ALBUM_01").join("DISC_01"),
//             tp.join("ALBUM_01").join("DISC_02"),
//         ];
//         let produced_item_fps: HashSet<PathBuf> = extract_sub_item_fps(&lookup_ctx.cache, &meta_fp);
//         assert_eq!(expected_item_fps, produced_item_fps);

//         let meta_fp = tp.join("ALBUM_01").join("item.yml");
//         lookup_ctx.cache_meta_file(&meta_fp, false).expect("Unable to cache meta file");

//         let expected_meta_fps = hashset![
//             tp.join("ALBUM_01").join("item.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         let expected_item_fps = hashset![
//             tp.join("ALBUM_01").join("DISC_01"),
//             tp.join("ALBUM_01").join("DISC_02"),
//         ];
//         let produced_item_fps: HashSet<PathBuf> = extract_sub_item_fps(&lookup_ctx.cache, &meta_fp);
//         assert_eq!(expected_item_fps, produced_item_fps);

//         let meta_fp = tp.join("ALBUM_01").join("self.yml");
//         lookup_ctx.cache_meta_file(&meta_fp, false).expect("Unable to cache meta file");

//         let expected_meta_fps = hashset![
//             tp.join("ALBUM_01").join("item.yml"),
//             tp.join("ALBUM_01").join("self.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         let expected_item_fps = hashset![
//             tp.join("ALBUM_01"),
//         ];
//         let produced_item_fps: HashSet<PathBuf> = extract_sub_item_fps(&lookup_ctx.cache, &meta_fp);
//         assert_eq!(expected_item_fps, produced_item_fps);
//     }

//     #[test]
//     fn test_cache_item_file() {
//         let (temp_media_root, media_lib) = default_setup("test_cache_item_file");
//         let tp = temp_media_root.path();

//         let mut lookup_ctx = LookupContext::new(&media_lib);

//         let item_fp = tp.join("ALBUM_01").join("DISC_01");
//         lookup_ctx.cache_item_file(&item_fp, false).expect("Unable to cache item file");

//         let expected_meta_fps = hashset![
//             tp.join("ALBUM_01").join("item.yml"),
//             tp.join("ALBUM_01").join("DISC_01").join("self.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         let expected_item_fps = hashset![
//             tp.join("ALBUM_01").join("DISC_01"),
//             // All item files pointed to by the item's meta file are cached.
//             tp.join("ALBUM_01").join("DISC_02"),
//         ];
//         let produced_item_fps: HashSet<PathBuf> = extract_all_item_fps(&lookup_ctx.cache);
//         assert_eq!(expected_item_fps, produced_item_fps);

//         let item_fp = tp.join("ALBUM_01").join("DISC_02");
//         lookup_ctx.cache_item_file(&item_fp, false).expect("Unable to cache item file");

//         let expected_meta_fps = hashset![
//             // This should already be present from the first lookup.
//             tp.join("ALBUM_01").join("item.yml"),
//             tp.join("ALBUM_01").join("DISC_01").join("self.yml"),
//             tp.join("ALBUM_01").join("DISC_02").join("self.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         let expected_item_fps = hashset![
//             tp.join("ALBUM_01").join("DISC_01"),
//             // All item files pointed to by the item's meta file are cached.
//             tp.join("ALBUM_01").join("DISC_02"),
//         ];
//         let produced_item_fps: HashSet<PathBuf> = extract_all_item_fps(&lookup_ctx.cache);
//         assert_eq!(expected_item_fps, produced_item_fps);

//         let item_fp = tp.join("ALBUM_01").join("DISC_01").join("TRACK_01.flac");
//         lookup_ctx.cache_item_file(&item_fp, false).expect("Unable to cache item file");

//         let expected_meta_fps = hashset![
//             // This should already be present from the first lookup.
//             tp.join("ALBUM_01").join("item.yml"),
//             tp.join("ALBUM_01").join("DISC_01").join("self.yml"),
//             tp.join("ALBUM_01").join("DISC_01").join("item.yml"),
//             tp.join("ALBUM_01").join("DISC_02").join("self.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         let expected_item_fps = hashset![
//             tp.join("ALBUM_01").join("DISC_01"),
//             // All item files pointed to by the item's meta file are cached.
//             tp.join("ALBUM_01").join("DISC_01").join("TRACK_01.flac"),
//             tp.join("ALBUM_01").join("DISC_01").join("TRACK_02.flac"),
//             tp.join("ALBUM_01").join("DISC_01").join("TRACK_03.flac"),
//             tp.join("ALBUM_01").join("DISC_02"),
//         ];
//         let produced_item_fps: HashSet<PathBuf> = extract_all_item_fps(&lookup_ctx.cache);
//         assert_eq!(expected_item_fps, produced_item_fps);
//     }

//     #[test]
//     fn test_clear() {
//         let (temp_media_root, media_lib) = default_setup("test_clear");
//         let tp = temp_media_root.path();

//         let mut lookup_ctx = LookupContext::new(&media_lib);

//         lookup_ctx.cache_item_file(tp.join("ALBUM_01"), false).expect("Unable to cache item file");
//         lookup_ctx.cache_item_file(tp.join("ALBUM_02"), false).expect("Unable to cache item file");
//         lookup_ctx.cache_item_file(tp.join("ALBUM_03"), false).expect("Unable to cache item file");
//         lookup_ctx.cache_item_file(tp.join("ALBUM_05"), false).expect("Unable to cache item file");

//         let expected_meta_fps = hashset![
//             tp.join("item.yml"),
//             tp.join("ALBUM_01").join("self.yml"),
//             tp.join("ALBUM_02").join("self.yml"),
//             tp.join("ALBUM_03").join("self.yml"),
//             tp.join("ALBUM_05").join("self.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         lookup_ctx.clear();

//         let expected_meta_fps = hashset![];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);
//     }

//     #[test]
//     fn test_clear_meta_file() {
//         let (temp_media_root, media_lib) = default_setup("test_clear_meta_file");
//         let tp = temp_media_root.path();

//         let mut lookup_ctx = LookupContext::new(&media_lib);

//         lookup_ctx.cache_item_file(tp.join("ALBUM_01"), false).expect("Unable to cache item file");
//         lookup_ctx.cache_item_file(tp.join("ALBUM_02"), false).expect("Unable to cache item file");
//         lookup_ctx.cache_item_file(tp.join("ALBUM_03"), false).expect("Unable to cache item file");
//         lookup_ctx.cache_item_file(tp.join("ALBUM_05"), false).expect("Unable to cache item file");

//         let expected_meta_fps = hashset![
//             tp.join("item.yml"),
//             tp.join("ALBUM_01").join("self.yml"),
//             tp.join("ALBUM_02").join("self.yml"),
//             tp.join("ALBUM_03").join("self.yml"),
//             tp.join("ALBUM_05").join("self.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         let expected_item_fps = hashset![
//             tp.join("ALBUM_01"),
//             tp.join("ALBUM_02"),
//             tp.join("ALBUM_03"),
//             tp.join("ALBUM_04.flac"),
//             tp.join("ALBUM_05"),
//         ];
//         let produced_item_fps: HashSet<PathBuf> = extract_all_item_fps(&lookup_ctx.cache);
//         assert_eq!(expected_item_fps, produced_item_fps);

//         lookup_ctx.clear_meta_file(tp.join("item.yml")).expect("Unable to clear cache");

//         let expected_meta_fps = hashset![
//             tp.join("ALBUM_01").join("self.yml"),
//             tp.join("ALBUM_02").join("self.yml"),
//             tp.join("ALBUM_03").join("self.yml"),
//             tp.join("ALBUM_05").join("self.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         let expected_item_fps = hashset![
//             tp.join("ALBUM_01"),
//             tp.join("ALBUM_02"),
//             tp.join("ALBUM_03"),
//             tp.join("ALBUM_05"),
//         ];
//         let produced_item_fps: HashSet<PathBuf> = extract_all_item_fps(&lookup_ctx.cache);
//         assert_eq!(expected_item_fps, produced_item_fps);

//         lookup_ctx.clear_meta_file(tp.join("ALBUM_01").join("self.yml")).expect("Unable to clear cache");

//         let expected_meta_fps = hashset![
//             tp.join("ALBUM_02").join("self.yml"),
//             tp.join("ALBUM_03").join("self.yml"),
//             tp.join("ALBUM_05").join("self.yml"),
//         ];
//         let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&lookup_ctx.cache);
//         assert_eq!(expected_meta_fps, produced_meta_fps);

//         let expected_item_fps = hashset![
//             tp.join("ALBUM_02"),
//             tp.join("ALBUM_03"),
//             tp.join("ALBUM_05"),
//         ];
//         let produced_item_fps: HashSet<PathBuf> = extract_all_item_fps(&lookup_ctx.cache);
//         assert_eq!(expected_item_fps, produced_item_fps);
//     }
// }
