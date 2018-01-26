use std::path::{Path, PathBuf};
use std::collections::HashMap;

use error::*;
use metadata::MetaBlock;
use library::Library;

pub type MetadataCache = HashMap<PathBuf, MetaBlock>;
pub type MetaFileCache = HashMap<PathBuf, MetadataCache>;

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
    use test_helpers::default_setup;

    use super::{
        LookupCacher,
        MetadataCache,
        MetaFileCache,
    };

    #[test]
    fn test_new() {
        let (temp_media_root, media_lib) = default_setup("test_new");
        let tp = temp_media_root.path();

        let mut cacher = LookupCacher::new(&media_lib);

        assert!(cacher.cache.is_empty());

        cacher.cache_meta_file(tp.join("ALBUM_01").join("item.yml"), false).expect("unable to cache meta file");

        assert_eq!(1, cacher.cache.len());
    }
}
