use std::path::Path;
use std::collections::HashSet;

use library::MediaLibrary;
use helpers::{normalize};

trait LabelExtractor {
    fn extract_label<S: AsRef<str>>(&self, item_file_name: S) -> String;
}

pub enum LookupDirection {
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

pub fn lookup_field<P: AsRef<Path>>(
    media_library: &MediaLibrary,
    abs_item_path: P,
    options: &LookupOptions,
    )
{
    let abs_item_path = normalize(abs_item_path.as_ref());

    // Get meta file paths from item path.
    let meta_file_paths = media_library.meta_fps_from_item_fp(&abs_item_path).unwrap();

    for meta_file_path in meta_file_paths {
        // Open this meta file path and see if it contains the field we are looking for.
        match media_library.item_fps_from_meta_fp(&meta_file_path) {
            Ok(records) => {
                for (found_item_path, found_meta_block) in records {
                    if abs_item_path == found_item_path {
                        // We found a meta block for this path, check if the desired field is contained.
                        match found_meta_block.get(&options.field_name) {
                            Some(val) => {
                                println!("Found value: {:?}", val);
                                break;
                            },
                            None => {
                                println!("Value not found here");
                                continue;
                            }
                        }
                    }
                }
            },
            Err(_) => {},
        }

        // if let Ok(records) = media_library.item_fps_from_meta_fp(&meta_file_path) {
        //     for (found_item_path, found_meta_block) in records {
        //         if abs_item_path == found_item_path {
        //             // We found a meta block for this path, check if the desired field is contained.
        //             match found_meta_block.get(&options.field_name) {
        //                 Some(val) => { println!("Found value: {:?}", val); break; },
        //                 None => { println!("Value not found here"); continue; }
        //             }
        //         }
        //     }
        // }

        // for record in records {
        //     println!("{:?}", record);
        // }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{lookup_field, LookupOptions};
    use library::MediaLibrary;
    use library::selection::Selection;
    use library::sort_order::SortOrder;
    use metadata::MetaTarget;

    #[test]
    fn test_lookup_field() {
        let media_lib = MediaLibrary::new(
            Path::new("/home/lemoine/Music"),
            vec![(String::from("taggu_self.yml"), MetaTarget::Contains), (String::from("taggu_item.yml"), MetaTarget::Siblings)],
            Selection::Ext(String::from("flac")),
            SortOrder::Name,
        ).expect("Unable to create media library");

        // println!("\n\n");
        lookup_field(&media_lib, Path::new("/home/lemoine/Music/BASS AVENGERS/1.01. Nhato - Gotta Get Down.flac"), &LookupOptions::new("artist"));
        lookup_field(&media_lib, Path::new("/home/lemoine/Music/BASS AVENGERS/"), &LookupOptions::new("what field"));
        // println!("---------------------");
        // lookup_field(&media_lib, Path::new("/home/lemoine/Music/BASS AVENGERS"), "COOL");
        // println!("\n\n");
    }
}
