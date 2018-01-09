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

pub type LookupOutcome = Option<MetaValue>;

pub fn lookup_field<P: AsRef<Path>>(
    media_library: &MediaLibrary,
    abs_item_path: P,
    options: &LookupOptions,
    ) -> Result<LookupOutcome>
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
    }

    // No error, but value was not found.
    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::fs::{DirBuilder, File};
    use std::collections::HashMap;
    use std::thread::sleep;
    use std::time::Duration;

    use tempdir::TempDir;

    use super::{lookup_field, LookupOptions};
    use library::{MediaLibrary, LibraryBuilder};
    use library::selection::Selection;
    use library::sort_order::SortOrder;
    use metadata::MetaTarget;

    const A_LABEL: &str = "ALBUM";
    const D_LABEL: &str = "DISC";
    const T_LABEL: &str = "TRACK";
    const S_LABEL: &str = "SUBTRACK";

    #[derive(Clone, Copy)]
    enum ItemMetaType {
        Seq,
        Map,
    }

    enum Entry {
        Dir(String, Vec<Entry>),
        File(String),
    }

    // enum TEntry<'a> {
    //     Dir(&'a str, &'a [Entry]),
    //     File(&'a str)
    // }

    // const TEST_DIR_ENTRIES: &[TEntry] = &[
    //     TEntry::File("ALBUM_04"),
    // ];

    const MEDIA_FILE_EXT: &str = "flac";

    fn default_dir_hierarchy() -> Vec<Entry> {
        vec![
            // Well-behaved album.
            Entry::Dir(format!("{}_01", A_LABEL), vec![
                Entry::Dir(format!("{}_01", D_LABEL), vec![
                    Entry::File(format!("{}_01", T_LABEL)),
                    Entry::File(format!("{}_02", T_LABEL)),
                    Entry::File(format!("{}_03", T_LABEL)),
                ]),
                Entry::Dir(format!("{}_02", D_LABEL), vec![
                    Entry::File(format!("{}_01", T_LABEL)),
                    Entry::File(format!("{}_02", T_LABEL)),
                    Entry::File(format!("{}_03", T_LABEL)),
                ]),
            ]),

            // Album with a disc and tracks, and loose tracks not on a disc.
            Entry::Dir(format!("{}_02", A_LABEL), vec![
                Entry::Dir(format!("{}_01", D_LABEL), vec![
                    Entry::File(format!("{}_01", T_LABEL)),
                    Entry::File(format!("{}_02", T_LABEL)),
                    Entry::File(format!("{}_03", T_LABEL)),
                ]),
                Entry::File(format!("{}_01", T_LABEL)),
                Entry::File(format!("{}_02", T_LABEL)),
                Entry::File(format!("{}_03", T_LABEL)),
            ]),

            // Album with discs and tracks, and subtracks on one disc.
            Entry::Dir(format!("{}_03", A_LABEL), vec![
                Entry::Dir(format!("{}_01", D_LABEL), vec![
                    Entry::File(format!("{}_01", T_LABEL)),
                    Entry::File(format!("{}_02", T_LABEL)),
                    Entry::File(format!("{}_03", T_LABEL)),
                ]),
                Entry::Dir(format!("{}_02", D_LABEL), vec![
                    Entry::Dir(format!("{}_01", T_LABEL), vec![
                        Entry::File(format!("{}_01", S_LABEL)),
                        Entry::File(format!("{}_02", S_LABEL)),
                    ]),
                    Entry::Dir(format!("{}_02", T_LABEL), vec![
                        Entry::File(format!("{}_01", S_LABEL)),
                        Entry::File(format!("{}_02", S_LABEL)),
                    ]),
                    Entry::File(format!("{}_03", T_LABEL)),
                    Entry::File(format!("{}_04", T_LABEL)),
                ]),
            ]),

            // Album that consists of one file.
            Entry::File(format!("{}_04", A_LABEL)),

            // A very messed-up album.
            Entry::Dir(format!("{}_05", A_LABEL), vec![
                Entry::Dir(format!("{}_01", D_LABEL), vec![
                    Entry::File(format!("{}_01", S_LABEL)),
                    Entry::File(format!("{}_02", S_LABEL)),
                    Entry::File(format!("{}_03", S_LABEL)),
                ]),
                Entry::Dir(format!("{}_02", D_LABEL), vec![
                    Entry::Dir(format!("{}_01", T_LABEL), vec![
                        Entry::File(format!("{}_01", S_LABEL)),
                        Entry::File(format!("{}_02", S_LABEL)),
                    ]),
                ]),
                Entry::File(format!("{}_01", T_LABEL)),
                Entry::File(format!("{}_02", T_LABEL)),
                Entry::File(format!("{}_03", T_LABEL)),
            ]),
        ]
    }

    fn create_temp_media_test_dir(name: &str /*, imt: ItemMetaType*/) -> TempDir {
        fn helper<P: AsRef<Path>>(curr_entry: &Entry, curr_cont_path: P, db: &DirBuilder /*, imt: ItemMetaType*/) {
            let curr_cont_path = curr_cont_path.as_ref();

            match *curr_entry {
                Entry::File(ref name) => {
                    File::create(curr_cont_path.join(name).with_extension(MEDIA_FILE_EXT)).expect("Unable to create file");
                },
                Entry::Dir(ref name, ref subentries) => {
                    let new_dir_path = curr_cont_path.join(name);
                    db.create(&new_dir_path).expect("Unable to create dir");

                    // match imt {
                    //     ItemMetaType::Seq => expr,
                    //     None => expr,
                    // }
                    // let items =

                    // Create all sub-entries.
                    for subentry in subentries {
                        helper(&subentry, &new_dir_path, db /*, imt*/);
                    }
                },
            }
        }

        let root_dir = TempDir::new(name).expect("Unable to create temp directory");
        let db = DirBuilder::new();
        let entries = default_dir_hierarchy();

        for entry in entries {
            helper(&entry, root_dir.path(), &db);
        }

        root_dir
    }

    #[test]
    fn test_lookup_field() {
        let temp_media_root = create_temp_media_test_dir("test_lookup_field");
        sleep(Duration::from_millis(1));

        let meta_target_specs = vec![
            (String::from("taggu_self.yml"), MetaTarget::Contains),
            (String::from("taggu_item.yml"), MetaTarget::Siblings),
        ];

        let media_lib = LibraryBuilder::new(temp_media_root.path(), meta_target_specs).selection(Selection::Ext(String::from("flac"))).create().expect("Unable to create media library");

        // println!("\n\n");
        lookup_field(&media_lib, Path::new("/home/lemoine/Music/BASS AVENGERS/1.01. Nhato - Gotta Get Down.flac"), &LookupOptions::new("artist"));
        lookup_field(&media_lib, Path::new("/home/lemoine/Music/BASS AVENGERS/"), &LookupOptions::new("what field"));
        lookup_field(&media_lib, Path::new("/home/lemoine/Music/DJ Snake - Encore/1.09. DJ Snake feat. Travi$ Scott, Migos, & G4shi - Oh Me Oh My.flac"), &LookupOptions::new("feat.artist"));
        // println!("---------------------");
        // lookup_field(&media_lib, Path::new("/home/lemoine/Music/BASS AVENGERS"), "COOL");
        // println!("\n\n");
    }
}
