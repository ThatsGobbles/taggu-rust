use std::path::Path;
use std::collections::HashSet;

use library::MediaLibrary;
use helpers::normalize;
use metadata::MetaValue;
use error::*;

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
    ) -> Result<Option<MetaValue>>
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
                        println!("Found value: {:?}", val);
                        return Ok(Some(val.clone()))
                    },
                    None => {
                        println!("Value not found here");
                        continue 'item;
                    }
                }
            }
        }

        // match media_library.item_fps_from_meta_fp(&meta_file_path) {
        //     Ok(records) => {
        //         'item: for (found_item_path, found_meta_block) in records {
        //             if abs_item_path == found_item_path {
        //                 // We found a meta block for this path, check if the desired field is contained.
        //                 match found_meta_block.get(&options.field_name) {
        //                     Some(val) => {
        //                         println!("Found value: {:?}", val);
        //                         break 'meta;
        //                     },
        //                     None => {
        //                         println!("Value not found here");
        //                         continue 'item;
        //                     }
        //                 }
        //             }
        //         }
        //     },
        //     Err(_) => {
        //         // There was an error in looking up meta fps.
        //     },
        // }
    }

    // No error, but value was not found.
    Ok(None)
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
        lookup_field(&media_lib, Path::new("/home/lemoine/Music/DJ Snake - Encore/1.09. DJ Snake feat. Travi$ Scott, Migos, & G4shi - Oh Me Oh My.flac"), &LookupOptions::new("feat.artist"));
        // println!("---------------------");
        // lookup_field(&media_lib, Path::new("/home/lemoine/Music/BASS AVENGERS"), "COOL");
        // println!("\n\n");
    }
}
