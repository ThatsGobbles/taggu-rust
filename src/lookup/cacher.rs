use std::path::{Path, PathBuf};
use std::collections::HashMap;

use error::*;
use metadata::MetaBlock;
use library::Library;

pub type MetadataCache = HashMap<PathBuf, MetaBlock>;
pub type MetaFileCache = HashMap<PathBuf, MetadataCache>;

pub struct NewLookupCacher {
    cache: MetaFileCache,
}

impl NewLookupCacher {
    pub fn new() -> Self {
        NewLookupCacher { cache: hashmap![] }
    }

    pub fn cache_entry<MP: Into<PathBuf>, IP: Into<PathBuf>>(
        &mut self,
        meta_fp: MP,
        item_fp: IP,
        meta_block: MetaBlock,
        ) -> bool
    {
        let mut sub_map = self.cache.entry(meta_fp.into()).or_insert(hashmap![]);
        let old_val: Option<_> = sub_map.insert(item_fp.into(), meta_block);

        old_val.is_none()
    }
}

pub struct LookupCacher<'a> {
    cache: MetaFileCache,
    media_lib: &'a Library,
}

impl<'a> LookupCacher<'a> {
    pub fn new(media_lib: &'a Library) -> Self {
        LookupCacher {
            cache: hashmap![],
            media_lib,
        }
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::collections::HashSet;

    use tempdir::TempDir;

    use test_helpers::default_setup;
    use library::Library;

    use super::{
        LookupCacher,
        MetadataCache,
        MetaFileCache,
    };

    enum EqualityTarget {
        AllMetas,
        AllItems,
        SubItems(PathBuf),
    }

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

        let mut cacher = LookupCacher::new(&media_lib);

        assert!(cacher.cache.is_empty());
    }

    #[test]
    fn test_cache_meta_file() {
        let (temp_media_root, media_lib) = default_setup("test_cache_meta_file");
        let tp = temp_media_root.path();

        let mut cacher = LookupCacher::new(&media_lib);

        let meta_fp = tp.join("ALBUM_01").join("item.yml");
        cacher.cache_meta_file(&meta_fp, false).expect("unable to cache meta file");

        let expected_meta_fps = hashset![
            tp.join("ALBUM_01").join("item.yml"),
        ];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&cacher.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);

        let expected_item_fps = hashset![
            tp.join("ALBUM_01").join("DISC_01"),
            tp.join("ALBUM_01").join("DISC_02"),
        ];
        let produced_item_fps: HashSet<PathBuf> = extract_sub_item_fps(&cacher.cache, &meta_fp);
        assert_eq!(expected_item_fps, produced_item_fps);

        let meta_fp = tp.join("ALBUM_01").join("item.yml");
        cacher.cache_meta_file(&meta_fp, false).expect("unable to cache meta file");

        let expected_meta_fps = hashset![
            tp.join("ALBUM_01").join("item.yml"),
        ];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&cacher.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);

        let expected_item_fps = hashset![
            tp.join("ALBUM_01").join("DISC_01"),
            tp.join("ALBUM_01").join("DISC_02"),
        ];
        let produced_item_fps: HashSet<PathBuf> = extract_sub_item_fps(&cacher.cache, &meta_fp);
        assert_eq!(expected_item_fps, produced_item_fps);

        let meta_fp = tp.join("ALBUM_01").join("self.yml");
        cacher.cache_meta_file(&meta_fp, false).expect("unable to cache meta file");

        let expected_meta_fps = hashset![
            tp.join("ALBUM_01").join("item.yml"),
            tp.join("ALBUM_01").join("self.yml"),
        ];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&cacher.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);

        let expected_item_fps = hashset![
            tp.join("ALBUM_01"),
        ];
        let produced_item_fps: HashSet<PathBuf> = extract_sub_item_fps(&cacher.cache, &meta_fp);
        assert_eq!(expected_item_fps, produced_item_fps);
    }

    #[test]
    fn test_cache_item_file() {
        let (temp_media_root, media_lib) = default_setup("test_cache_item_file");
        let tp = temp_media_root.path();

        let mut cacher = LookupCacher::new(&media_lib);

        let item_fp = tp.join("ALBUM_01").join("DISC_01");
        cacher.cache_item_file(&item_fp, false).expect("unable to cache item file");

        let expected_meta_fps = hashset![
            tp.join("ALBUM_01").join("item.yml"),
            tp.join("ALBUM_01").join("DISC_01").join("self.yml"),
        ];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&cacher.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);

        let expected_item_fps = hashset![
            tp.join("ALBUM_01").join("DISC_01"),
            // All item files pointed to by the item's meta file are cached.
            tp.join("ALBUM_01").join("DISC_02"),
        ];
        let produced_item_fps: HashSet<PathBuf> = extract_all_item_fps(&cacher.cache);
        assert_eq!(expected_item_fps, produced_item_fps);

        let item_fp = tp.join("ALBUM_01").join("DISC_02");
        cacher.cache_item_file(&item_fp, false).expect("unable to cache item file");

        let expected_meta_fps = hashset![
            // This should already be present from the first lookup.
            tp.join("ALBUM_01").join("item.yml"),
            tp.join("ALBUM_01").join("DISC_01").join("self.yml"),
            tp.join("ALBUM_01").join("DISC_02").join("self.yml"),
        ];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&cacher.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);

        let expected_item_fps = hashset![
            tp.join("ALBUM_01").join("DISC_01"),
            // All item files pointed to by the item's meta file are cached.
            tp.join("ALBUM_01").join("DISC_02"),
        ];
        let produced_item_fps: HashSet<PathBuf> = extract_all_item_fps(&cacher.cache);
        assert_eq!(expected_item_fps, produced_item_fps);

        let item_fp = tp.join("ALBUM_01").join("DISC_01").join("TRACK_01.flac");
        cacher.cache_item_file(&item_fp, false).expect("unable to cache item file");

        let expected_meta_fps = hashset![
            // This should already be present from the first lookup.
            tp.join("ALBUM_01").join("item.yml"),
            tp.join("ALBUM_01").join("DISC_01").join("self.yml"),
            tp.join("ALBUM_01").join("DISC_01").join("item.yml"),
            tp.join("ALBUM_01").join("DISC_02").join("self.yml"),
        ];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&cacher.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);

        let expected_item_fps = hashset![
            tp.join("ALBUM_01").join("DISC_01"),
            // All item files pointed to by the item's meta file are cached.
            tp.join("ALBUM_01").join("DISC_01").join("TRACK_01.flac"),
            tp.join("ALBUM_01").join("DISC_01").join("TRACK_02.flac"),
            tp.join("ALBUM_01").join("DISC_01").join("TRACK_03.flac"),
            tp.join("ALBUM_01").join("DISC_02"),
        ];
        let produced_item_fps: HashSet<PathBuf> = extract_all_item_fps(&cacher.cache);
        assert_eq!(expected_item_fps, produced_item_fps);
    }

    #[test]
    fn test_clear() {
        let (temp_media_root, media_lib) = default_setup("test_clear");
        let tp = temp_media_root.path();

        let mut cacher = LookupCacher::new(&media_lib);

        cacher.cache_item_file(tp.join("ALBUM_01"), false).expect("unable to cache item file");
        cacher.cache_item_file(tp.join("ALBUM_02"), false).expect("unable to cache item file");
        cacher.cache_item_file(tp.join("ALBUM_03"), false).expect("unable to cache item file");
        cacher.cache_item_file(tp.join("ALBUM_05"), false).expect("unable to cache item file");

        let expected_meta_fps = hashset![
            tp.join("item.yml"),
            tp.join("ALBUM_01").join("self.yml"),
            tp.join("ALBUM_02").join("self.yml"),
            tp.join("ALBUM_03").join("self.yml"),
            tp.join("ALBUM_05").join("self.yml"),
        ];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&cacher.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);

        cacher.clear().expect("unable to clear cache");

        let expected_meta_fps = hashset![];
        let produced_meta_fps: HashSet<PathBuf> = extract_all_meta_fps(&cacher.cache);
        assert_eq!(expected_meta_fps, produced_meta_fps);
    }
}
