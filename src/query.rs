use std::path::Path;

use library::MediaLibrary;

trait LabelExtractor {
    fn extract_label<S: AsRef<str>>(&self, item_file_name: S) -> String;
}

pub fn yield_field<P: AsRef<Path>, S: AsRef<str>>(
    media_library: MediaLibrary,
    abs_item_path: P,
    field_name: S,
    )
{
    // Get meta file paths from item path.
    let target_meta_fps = media_library.meta_fps_from_item_fp(abs_item_path);

    for target_meta_fp in target_meta_fps {
        println!("{:?}", target_meta_fp);
    }
}
