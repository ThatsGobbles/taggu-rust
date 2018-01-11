use std::fs::{DirBuilder, File};
use std::path::Path;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;

use tempdir::TempDir;

const A_LABEL: &str = "ALBUM";
const D_LABEL: &str = "DISC";
const T_LABEL: &str = "TRACK";
const S_LABEL: &str = "SUBTRACK";

#[derive(Clone, Copy)]
enum ItemMetaType {
    Seq,
    Map,
}

#[derive(Clone)]
enum Entry {
    Dir(String, Vec<Entry>),
    File(String),
}

impl Entry {
    pub fn name(&self) -> &String {
        match *self {
            Entry::Dir(ref name, _) => name,
            Entry::File(ref name) => name,
        }
    }
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

fn create_test_dir_entries<P, S>(identifier: S, target_dir_path: P, subentries: Vec<Entry>, db: &DirBuilder)
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

        match subentry {
            Entry::File(ref name) => {
                File::create(target_dir_path.join(name).with_extension(MEDIA_FILE_EXT)).expect("Unable to create file");
            },
            Entry::Dir(ref name, ref new_subentries) => {
                let new_dir_path = target_dir_path.join(name);
                db.create(&new_dir_path).expect("Unable to create dir");

                create_test_dir_entries(name, new_dir_path, new_subentries.to_vec(), db);
            }
        }

        let entry_string = format!("- const_key: const_val\n  item_key: item_val\n  {}_item_key: {}_item_val\n", subentry.name(), subentry.name());
        item_meta_contents.push_str(&entry_string);
    }

    // println!("{}", item_meta_contents);

    // Create item meta file for all items in this directory.
    let mut item_meta_file = File::create(target_dir_path.join("item.yml")).expect("Unable to create item meta file");
    item_meta_file.write_all(item_meta_contents.as_bytes()).expect("Unable to write to item meta file");
}

pub fn create_temp_media_test_dir(name: &str /*, imt: ItemMetaType*/) -> TempDir {
    let root_dir = TempDir::new(name).expect("Unable to create temp directory");
    let db = DirBuilder::new();
    let entries = default_dir_hierarchy();

    create_test_dir_entries("ROOT", root_dir.path(), entries, &db);

    sleep(Duration::from_millis(1));
    root_dir
}
