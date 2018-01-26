// TODO: Wrap in #[cfg(test)].

use std::fs::{DirBuilder, File};
use std::path::Path;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;

use tempdir::TempDir;

use library::{Library, LibraryBuilder};
use library::selection::Selection;
use library::sort_order::SortOrder;
use metadata::target::MetaTarget;

enum TEntry<'a> {
    Dir(&'a str, &'a [TEntry<'a>]),
    File(&'a str)
}

impl<'a> TEntry<'a> {
    pub fn name(&self) -> &str {
        match *self {
            TEntry::Dir(ref name, _) => name,
            TEntry::File(ref name) => name,
        }
    }
}

const TEST_DIR_ENTRIES: &[TEntry] = &[
    // Well-behaved album.
    TEntry::Dir("ALBUM_01", &[
        TEntry::Dir("DISC_01", &[
            TEntry::File("TRACK_01"),
            TEntry::File("TRACK_02"),
            TEntry::File("TRACK_03"),
        ]),
        TEntry::Dir("DISC_02", &[
            TEntry::File("TRACK_01"),
            TEntry::File("TRACK_02"),
            TEntry::File("TRACK_03"),
        ]),
    ]),

    // Album with a disc and tracks, and loose tracks not on a disc.
    TEntry::Dir("ALBUM_02", &[
        TEntry::Dir("DISC_01", &[
            TEntry::File("TRACK_01"),
            TEntry::File("TRACK_02"),
            TEntry::File("TRACK_03"),
        ]),
        TEntry::File("TRACK_01"),
        TEntry::File("TRACK_02"),
        TEntry::File("TRACK_03"),
    ]),

    // Album with discs and tracks, and subtracks on one disc.
    TEntry::Dir("ALBUM_03", &[
        TEntry::Dir("DISC_01", &[
            TEntry::File("TRACK_01"),
            TEntry::File("TRACK_02"),
            TEntry::File("TRACK_03"),
        ]),
        TEntry::Dir("DISC_02", &[
            TEntry::Dir("TRACK_01", &[
                TEntry::File("SUBTRACK_01"),
                TEntry::File("SUBTRACK_02"),
            ]),
            TEntry::Dir("TRACK_02", &[
                TEntry::File("SUBTRACK_01"),
                TEntry::File("SUBTRACK_02"),
            ]),
            TEntry::File("TRACK_03"),
            TEntry::File("TRACK_04"),
        ]),
    ]),

    // Album that consists of one file.
    TEntry::File("ALBUM_04"),

    // A very messed-up album.
    TEntry::Dir("ALBUM_05", &[
        TEntry::Dir("DISC_01", &[
            TEntry::File("SUBTRACK_01"),
            TEntry::File("SUBTRACK_02"),
            TEntry::File("SUBTRACK_03"),
        ]),
        TEntry::Dir("DISC_02", &[
            TEntry::Dir("TRACK_01", &[
                TEntry::File("SUBTRACK_01"),
                TEntry::File("SUBTRACK_02"),
            ]),
        ]),
        TEntry::File("TRACK_01"),
        TEntry::File("TRACK_02"),
        TEntry::File("TRACK_03"),
    ]),
];

const MEDIA_FILE_EXT: &str = "flac";

// LEARN: Why unable to use IntoIterator<Item = Entry>?
fn create_test_dir_entries<'a, P, S>(identifier: S, target_dir_path: P, subentries: &[TEntry<'a>], db: &DirBuilder)
where P: AsRef<Path>,
      S: AsRef<str>,
{
    let identifier = identifier.as_ref();
    let target_dir_path = target_dir_path.as_ref();

    // Create self meta file for this directory.
    let mut self_meta_file = File::create(target_dir_path.join("self.yml")).expect("Unable to create self meta file");
    writeln!(self_meta_file, "const_key: const_val\nself_key: self_val\n{}_self_key: {}_self_val", identifier, identifier).expect("Unable to write to self meta file");

    // Create all sub-entries, and collect info to create item metadata.
    let mut item_meta_contents = String::new();
    for subentry in subentries.into_iter() {
        // helper(&subentry, &target_dir_path, db /*, imt*/);

        match *subentry {
            TEntry::File(name) => {
                File::create(target_dir_path.join(name).with_extension(MEDIA_FILE_EXT)).expect("Unable to create file");
            },
            TEntry::Dir(name, new_subentries) => {
                let new_dir_path = target_dir_path.join(name);
                db.create(&new_dir_path).expect("Unable to create dir");

                create_test_dir_entries(name, new_dir_path, new_subentries, db);
            }
        }

        let entry_string = format!("- const_key: const_val\n  item_key: item_val\n  {}_item_key: {}_item_val\n", subentry.name(), subentry.name());
        item_meta_contents.push_str(&entry_string);
    }

    // Create item meta file for all items in this directory.
    let mut item_meta_file = File::create(target_dir_path.join("item.yml")).expect("Unable to create item meta file");
    item_meta_file.write_all(item_meta_contents.as_bytes()).expect("Unable to write to item meta file");
}

pub fn create_temp_media_test_dir(name: &str /*, imt: ItemMetaType*/) -> TempDir {
    let root_dir = TempDir::new(name).expect("Unable to create temp directory");
    let db = DirBuilder::new();

    create_test_dir_entries("ROOT", root_dir.path(), TEST_DIR_ENTRIES, &db);

    sleep(Duration::from_millis(1));
    root_dir
}

pub fn default_setup(name: &str) -> (TempDir, Library) {
    let temp_media_root = create_temp_media_test_dir(name);

    let meta_target_specs = vec![
        (String::from("self.yml"), MetaTarget::Contains),
        (String::from("item.yml"), MetaTarget::Siblings),
    ];

    let selection = Selection::Or(
        Box::new(Selection::Ext(String::from("flac"))),
        Box::new(Selection::IsDir),
    );
    let media_lib = LibraryBuilder::new(temp_media_root.path(), meta_target_specs).selection(selection).create().expect("Unable to create media library");

    (temp_media_root, media_lib)
}
