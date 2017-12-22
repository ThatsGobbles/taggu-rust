use std::path::Path;

use library::MediaLibrary;
use helpers::{normalize};

trait LabelExtractor {
    fn extract_label<S: AsRef<str>>(&self, item_file_name: S) -> String;
}

pub fn yield_field<P: AsRef<Path>, S: AsRef<str>>(
    media_library: &MediaLibrary,
    abs_item_path: P,
    field_name: S,
    )
{
    let abs_item_path = normalize(abs_item_path.as_ref());

    // Get meta file paths from item path.
    let meta_file_paths = media_library.meta_fps_from_item_fp(abs_item_path).unwrap();

    for meta_file_path in meta_file_paths {
        println!("{:?}", meta_file_path);

        // Open this meta file path and see if it contains the field we are looking for.
        let records = media_library.item_fps_from_meta_fp(&meta_file_path);

        println!("{:?}", records);

        // for record in records {
        //     println!("{:?}", record);
        // }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{yield_field};
    use library::MediaLibrary;
    use library::selection::Selection;
    use library::sort_order::SortOrder;
    use metadata::MetaTarget;

    #[test]
    fn test_yield_field() {
        let media_lib = MediaLibrary::new(
            Path::new("/home/lemoine/Music"),
            vec![(String::from("taggu_self.yml"), MetaTarget::Contains), (String::from("taggu_item.yml"), MetaTarget::Siblings)],
            Selection::Ext(String::from("flac")),
            SortOrder::Name,
        ).expect("Unable to create media library");

        println!("\n\n");
        yield_field(&media_lib, Path::new("/home/lemoine/Music/BASS AVENGERS/1.01. Nhato - Gotta Get Down.flac"), "COOL");
        println!("---------------------");
        yield_field(&media_lib, Path::new("/home/lemoine/Music/BASS AVENGERS"), "COOL");
        println!("\n\n");
    }
}
