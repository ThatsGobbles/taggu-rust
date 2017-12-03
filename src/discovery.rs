use std::path::{Path, PathBuf};

use library::{MetaTarget, MediaLibrary};
use metadata::MetaBlock;
use generator::gen_to_iter;

pub fn meta_fps_from_item_fp<'a, P: Into<PathBuf> + 'a>(media_library: &MediaLibrary, abs_item_path: P) -> impl Iterator<Item = &'a Path> + 'a {
    // meta_specs: tt.MetaSourceSpecGen = library_context.yield_meta_source_specs()
    // for meta_spec in meta_specs:
    //     meta_file_name: pl.Path = meta_spec.meta_file_name
    //     dir_getter: tt.DirGetter = meta_spec.dir_getter

    //     # This loop will normally execute either zero or one time.
    //     for rel_meta_dir in dir_getter(rel_item_path):
    //         rel_meta_dir, abs_meta_dir = library_context.co_norm(rel_sub_path=rel_meta_dir)

    //         rel_meta_path = rel_meta_dir / meta_file_name
    //         abs_meta_path = abs_meta_dir / meta_file_name

    //         if abs_meta_path.is_file():
    //             logger.info(f'Found meta file "{rel_meta_path}" for item "{rel_item_path}"')
    //             yield rel_meta_path
    //         else:
    //             logger.debug(f'Meta file "{rel_meta_path}" does not exist for item "{rel_item_path}"')

    // yield tt.MetaSourceSpec(meta_file_name=pl.Path(cls.get_self_meta_file_name()),
    //                         dir_getter=cls.yield_contains_dir,
    //                         multiplexer=cls.yield_self_meta_pairs)
    // yield tt.MetaSourceSpec(meta_file_name=pl.Path(cls.get_item_meta_file_name()),
    //                         dir_getter=cls.yield_siblings_dir,
    //                         multiplexer=cls.yield_item_meta_pairs)

    let closure = move || {
        if false {
            yield Path::new("foo.txt")
        }
    };
    gen_to_iter(closure)
}

pub fn item_fps_from_meta_fp<'a, P: Into<PathBuf> + 'a>(abs_item_path: P) -> impl Iterator<Item = (&'a Path, MetaBlock)> + 'a {
    let closure = move || {
        if false {
            yield (Path::new("foo.txt"), MetaBlock::new())
        }
    };
    gen_to_iter(closure)
}
