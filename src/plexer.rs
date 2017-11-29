// This module provides an interface to "match up" media items with metadata blocks.

use std::path::{Path, PathBuf};

use library::MediaLibrary;
use generator::gen_to_iter;
use metadata::MetaBlock;

pub fn iter_self_meta_plexer<'a, P: AsRef<Path> + 'a>(media_lib: &'a MediaLibrary, rel_sub_dir_path: P) -> impl Iterator<Item = (PathBuf, MetaBlock)> + 'a {
    let closure = move || {
        if false {
            yield (PathBuf::new(), MetaBlock::new())
        }
    };
    gen_to_iter(closure)
}

pub fn iter_item_meta_plexer<'a, P: AsRef<Path> + 'a>(media_lib: &'a MediaLibrary, rel_sub_dir_path: P) -> impl Iterator<Item = (PathBuf, MetaBlock)> + 'a {
    let closure = move || {
        if false {
            yield (PathBuf::new(), MetaBlock::new())
        }
    };
    gen_to_iter(closure)
}
