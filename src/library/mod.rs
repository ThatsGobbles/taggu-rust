pub mod selection;
pub mod sort_order;

use std::path::{Path, PathBuf};
use std::path::Component;
use std::ffi::OsString;
use std::fs::DirEntry;
use std::cmp::Ordering;
use std::time::SystemTime;

use regex::Regex;

use super::error::MediaLibraryError;
use super::path::normalize;
use super::metadata::{MetaBlock, MetaTarget};
use super::generator::gen_to_iter;
use super::plexer::plex;

use self::selection::Selection;
use self::sort_order::SortOrder;

pub struct MediaLibrary {
    root_dir: PathBuf,
    meta_targets: Vec<MetaTarget>,
    selection: Selection,
    sort_order: SortOrder,
}

impl MediaLibrary {

    // METHODS

    /// Creates a new `MediaLibrary`.
    /// The root path is canonicalized and converted into a PathBuf, and must point to a directory.
    pub fn new<P: Into<PathBuf>>(
            root_dir: P,
            meta_targets: Vec<MetaTarget>,
            selection: Selection,
            sort_order: SortOrder,
            ) -> Result<MediaLibrary, MediaLibraryError> {
        let root_dir = try!(root_dir.into().canonicalize());

        if !root_dir.is_dir() {
            return Err(MediaLibraryError::NotADir(root_dir))
        }

        Ok(MediaLibrary {
            root_dir,
            meta_targets,
            selection,
            sort_order,
        })
    }

    pub fn is_proper_sub_path<P: Into<PathBuf>>(&self, abs_sub_path: P) -> bool {
        let abs_sub_path = normalize(&abs_sub_path.into());

        abs_sub_path.starts_with(&self.root_dir)
    }

    pub fn is_selected_media_item<P: Into<PathBuf>>(&self, abs_item_path: P) -> bool {
        let abs_item_path = abs_item_path.into();

        self.is_proper_sub_path(&abs_item_path) && self.selection.is_selected_path(abs_item_path)
    }

    pub fn all_entries_in_dir<'a, P: Into<PathBuf> + 'a>(&'a self, abs_sub_dir_path: P) -> impl Iterator<Item = DirEntry> + 'a {
        let abs_sub_dir_path = normalize(&abs_sub_dir_path.into());

        let closure = move || {
            // LEARN: Why does this work when separated, but not when inlined?
            let iter = abs_sub_dir_path.read_dir();
            if let Ok(dir_entries) = iter {
                for dir_entry in dir_entries {
                    if let Ok(dir_entry) = dir_entry {
                        yield dir_entry;
                    }
                }
            }
        };

        gen_to_iter(closure)
    }

    pub fn selected_entries_in_dir<'a, P: Into<PathBuf> + 'a>(&'a self, abs_sub_dir_path: P) -> impl Iterator<Item = DirEntry> + 'a {
        let abs_sub_dir_path = normalize(&abs_sub_dir_path.into());

        self.all_entries_in_dir(abs_sub_dir_path).filter(move |x| self.is_selected_media_item(x.path()))
    }

    pub fn find_meta_target_by_fn<S: AsRef<str>>(&self, meta_file_name: S) -> Option<&MetaTarget> {
        let meta_file_name = meta_file_name.as_ref();
        self.meta_targets.iter().find(|ref mt| *mt.meta_file_name() == *meta_file_name)
    }

    pub fn meta_fps_from_item_fp<'a, P: Into<PathBuf> + 'a>(&'a self, abs_item_path: P) -> impl Iterator<Item = PathBuf> + 'a {
        let abs_item_path = normalize(&abs_item_path.into());

        let closure = move || {
            // Rule: item path must be proper.
            if !self.is_proper_sub_path(&abs_item_path) {
                return
            }

            // Rule: item path must exist.
            if !abs_item_path.exists() {
                return
            }

            for meta_target in &self.meta_targets {
                let temp = meta_target.meta_file_path(&abs_item_path);
                if let Some(meta_fp) = temp {
                    if self.is_proper_sub_path(&abs_item_path) {
                        yield meta_fp
                    }
                }
            }
        };
        gen_to_iter(closure)
    }

    pub fn item_fps_from_meta_fp<'a, P: Into<PathBuf> + 'a>(&'a self, abs_meta_path: P) -> impl Iterator<Item = (PathBuf, MetaBlock)> + 'a {
        let abs_meta_path = normalize(&abs_meta_path.into());

        let closure = move || {
            // Rule: meta file path must be proper.
            if !self.is_proper_sub_path(&abs_meta_path) {
                return
            }

            // Rule: meta file path must exist and be a file.
            if !abs_meta_path.is_file() {
                return
            }

            let temp = abs_meta_path.parent().map(|p| p.to_path_buf());
            if let Some(working_dir_path) = temp {
                // Rule: working dir path must be proper.
                if !self.is_proper_sub_path(&working_dir_path) {
                    return
                }

                // LEARN: Why can't the following be inlined?
                let temp = abs_meta_path.file_name().and_then(|s| s.to_str()).map(|s| s.to_string());
                if let Some(found_meta_fn) = temp {
                    // We have a meta file name, now try and match it to any of the file names in meta targets.
                    let temp = self.find_meta_target_by_fn(&found_meta_fn);
                    if let Some(meta_target) = temp {
                        // Use the meta file parent directory, and plex appropriately.
                        let temp = plex(working_dir_path, &meta_target, &self.selection, &self.sort_order);
                        for (p, mb) in temp {
                            yield (p, mb)
                        }
                    }
                }
            }
        };
        gen_to_iter(closure)
    }

    // ASSOCIATED FUNCTIONS

    pub fn is_valid_item_name<S: Into<String>>(file_name: S) -> bool {
        let file_name = file_name.into();
        let normed = normalize(Path::new(&file_name));

        // A valid item file name will have the same string repr before and after normalization.
        match normed.to_str() {
            Some(ns) if ns == file_name => {},
            _ => { return false },
        }

        let comps: Vec<_> = normed.components().collect();

        // A valid item file name has only one component, and it must be normal.
        if comps.len() != 1 {
            return false
        }

        match comps[0] {
            Component::Normal(_) => true,
            _ => false
        }
    }
}

// =================================================================================================
// TESTS
// =================================================================================================


#[cfg(test)]
mod tests {
    use std::path::{PathBuf};
    use std::fs::{File, DirBuilder};

    enum TP {
        F(PathBuf),
        D(PathBuf),
    }

    impl TP {
        fn create(&self) {
            let mut db = DirBuilder::new();
            db.recursive(true);

            match self {
                &TP::F(ref p) => {
                    if let Some(parent) = p.parent() {
                        db.create(parent).unwrap();
                    }
                    File::create(p).unwrap();
                },
                &TP::D(ref p) => { db.create(p).unwrap(); },
            }
        }

        fn to_path_buf(&self) -> PathBuf {
            match self {
                &TP::F(ref p) => p.clone(),
                &TP::D(ref p) => p.clone(),
            }
        }

        fn is_file(&self) -> bool {
            match self {
                &TP::F(_) => true,
                _ => false,
            }
        }

        fn is_dir(&self) -> bool {
            !self.is_file()
        }
    }

    mod media_library {
        use super::super::{MediaLibrary, SortOrder};
        use super::super::selection::Selection;
        use std::path::{PathBuf};
        use std::fs::{File, DirBuilder};
        use regex::Regex;
        use std::collections::HashSet;
        use std::io::Write;

        use tempdir::TempDir;

        use metadata::{MetaTarget, MetaValue};
        use super::TP;

        // METHODS

        #[test]
        fn test_is_proper_sub_path() {
            // Create temp directory.
            let temp = TempDir::new("test_is_proper_sub_path").unwrap();
            let tp = temp.path();

            let ml = MediaLibrary::new(
                tp,
                vec![],
                Selection::True,
                SortOrder::Name,
            ).unwrap();

            let inputs_and_expected = vec![
                (tp.join("sub"), true),
                (tp.join("sub").join("more"), true),
                (tp.join(".."), false),
                (tp.join("."), true),
                (TempDir::new("other").unwrap().path().to_path_buf(), false),
                (tp.join("sub").join("more").join("..").join("back"), true),
                (tp.join("sub").join("more").join("..").join(".."), true),
                (tp.join("sub").join("..").join(".."), false),
                (tp.join(".").join("sub").join("."), true),
            ];

            for (input, expected) in inputs_and_expected {
                let produced = ml.is_proper_sub_path(&input);
                assert_eq!(expected, produced);
            }
        }

        #[test]
        fn test_all_entries_in_dir() {
            // Create temp directory.
            let temp = TempDir::new("test_all_entries_in_dir").unwrap();
            let tp = temp.path();

            let paths_to_create = vec![
                TP::F(tp.join("file_a.flac")),
                TP::F(tp.join("file_b.flac")),
                TP::F(tp.join("file_c.flac")),
                TP::F(tp.join("file_d.yml")),
                TP::F(tp.join("file_e.jpg")),
            ];

            for path_to_create in &paths_to_create {
                path_to_create.create();
            }

            let ml = MediaLibrary::new(
                tp,
                vec![],
                Selection::True,
                SortOrder::Name,
            ).unwrap();

            let expected: HashSet<PathBuf> = paths_to_create.iter().map(|fp| fp.to_path_buf()).collect();
            let produced: HashSet<PathBuf> = ml.all_entries_in_dir(tp).map(|e| e.path().to_path_buf()).collect();

            assert_eq!(expected, produced);
        }

        #[test]
        fn test_selected_entries_in_dir() {
            // Create temp directory.
            let temp = TempDir::new("test_selected_entries_in_dir").unwrap();
            let tp = temp.path();

            let paths_to_create = vec![
                TP::F(tp.join("file_a.flac")),
                TP::F(tp.join("file_b.flac")),
                TP::F(tp.join("file_c.flac")),
                TP::D(tp.join("sub")),
                TP::F(tp.join("file_d.yml")),
                TP::F(tp.join("file_e.jpg")),
            ];

            for path_to_create in &paths_to_create {
                path_to_create.create();
            }

            let ml = MediaLibrary::new(
                tp,
                vec![],
                Selection::True,
                SortOrder::Name,
            ).unwrap();

            let expected: HashSet<PathBuf> = paths_to_create.iter().map(|fp| fp.to_path_buf()).collect();
            let produced: HashSet<PathBuf> = ml.selected_entries_in_dir(tp).map(|e| e.path().to_path_buf()).collect();

            assert_eq!(expected, produced);

            let ml = MediaLibrary::new(
                tp,
                vec![],
                Selection::Ext("flac".to_string()),
                SortOrder::Name,
            ).unwrap();

            let expected: HashSet<PathBuf> = paths_to_create.iter().take(3).map(|fp| fp.to_path_buf()).collect();
            let produced: HashSet<PathBuf> = ml.selected_entries_in_dir(tp).map(|e| e.path().to_path_buf()).collect();

            assert_eq!(expected, produced);

            let ml = MediaLibrary::new(
                tp,
                vec![],
                Selection::Or(
                    Box::new(Selection::IsDir),
                    Box::new(
                        Selection::And(
                            Box::new(Selection::IsFile),
                            Box::new(Selection::Ext("flac".to_string())),
                        ),
                    ),
                ),
                SortOrder::Name,
            ).unwrap();

            let expected: HashSet<PathBuf> = paths_to_create.iter().take(4).map(|fp| fp.to_path_buf()).collect();
            let produced: HashSet<PathBuf> = ml.selected_entries_in_dir(tp).map(|e| e.path().to_path_buf()).collect();

            assert_eq!(expected, produced);
        }

        #[test]
        fn test_meta_fps_from_item_fp() {
            // Create temp directory.
            let temp = TempDir::new("test_meta_fps_from_item_fp").unwrap();
            let tp = temp.path();

            let db = DirBuilder::new();

            let meta_targets = vec![
                MetaTarget::Container(String::from("self.yml")),
                MetaTarget::Alongside(String::from("item.yml")),
            ];
            let selection = Selection::Or(
                Box::new(Selection::IsDir),
                Box::new(
                    Selection::And(
                        Box::new(Selection::IsFile),
                        Box::new(Selection::Ext("flac".to_string())),
                    ),
                ),
            );

            // Create sample item files and directories.
            File::create(tp.join("item.flac")).unwrap();
            db.create(tp.join("subdir")).unwrap();
            File::create(tp.join("subdir").join("subitem.flac")).unwrap();

            // Create meta files.
            let mut meta_file = File::create(tp.join("item.yml"))
                .expect("Unable to create metadata file");

            writeln!(meta_file, "item.flac:\n  title: Black Mamba\n  artist: lapix\nsubdir:\n  title: What Is This?")
                .expect("Unable to write metadata file");

            let mut meta_file = File::create(tp.join("self.yml"))
                .expect("Unable to create metadata file");

            writeln!(meta_file, "title: PsyStyle Nation\nartist: [lapix, Massive New Krew]")
                .expect("Unable to write metadata file");

            let mut meta_file = File::create(tp.join("subdir").join("self.yml"))
                .expect("Unable to create metadata file");

            writeln!(meta_file, "title: A Subtrack?\nartist: Massive New Krew")
                .expect("Unable to write metadata file");

            // Create media library.
            let media_lib = MediaLibrary::new(
                &tp,
                meta_targets,
                selection,
                SortOrder::Name,
            ).expect("Unable to create media library");

            // Run tests.
            let found: Vec<_> = media_lib.meta_fps_from_item_fp(&tp).collect();
            assert_eq!(vec![tp.join("self.yml")], found);

            let found: Vec<_> = media_lib.meta_fps_from_item_fp(tp.join("item.flac")).collect();
            assert_eq!(vec![tp.join("item.yml")], found);

            let found: Vec<_> = media_lib.meta_fps_from_item_fp(tp.join("subdir")).collect();
            assert_eq!(vec![tp.join("subdir").join("self.yml"), tp.join("item.yml")], found);

            let found: Vec<_> = media_lib.meta_fps_from_item_fp(tp.join("DOES_NOT_EXIST")).collect();
            assert_eq!(Vec::<PathBuf>::new(), found);

            let found: Vec<_> = media_lib.meta_fps_from_item_fp(tp.join("subdir").join("subitem.flac")).collect();
            assert_eq!(Vec::<PathBuf>::new(), found);
        }

        #[test]
        fn test_item_fps_from_meta_fp() {
            // Create temp directory.
            let temp = TempDir::new("test_meta_fps_from_item_fp").unwrap();
            let tp = temp.path();

            let db = DirBuilder::new();

            let meta_targets = vec![
                MetaTarget::Container(String::from("self.yml")),
                MetaTarget::Alongside(String::from("item.yml")),
            ];
            let selection = Selection::Or(
                Box::new(Selection::IsDir),
                Box::new(
                    Selection::And(
                        Box::new(Selection::IsFile),
                        Box::new(Selection::Ext("flac".to_string())),
                    ),
                ),
            );

            // Create sample item files and directories.
            File::create(tp.join("item.flac")).unwrap();
            db.create(tp.join("subdir")).unwrap();
            File::create(tp.join("subdir").join("subitem.flac")).unwrap();

            // Create meta files.
            let mut meta_file = File::create(tp.join("item.yml"))
                .expect("Unable to create metadata file");

            writeln!(meta_file, "item.flac:\n  title: Black Mamba\n  artist: lapix\nsubdir:\n  title: What Is This?")
                .expect("Unable to write metadata file");

            let mut meta_file = File::create(tp.join("self.yml"))
                .expect("Unable to create metadata file");

            writeln!(meta_file, "title: PsyStyle Nation\nartist: [lapix, Massive New Krew]")
                .expect("Unable to write metadata file");

            let mut meta_file = File::create(tp.join("subdir").join("self.yml"))
                .expect("Unable to create metadata file");

            writeln!(meta_file, "title: A Subtrack?\nartist: Massive New Krew")
                .expect("Unable to write metadata file");

            // Create media library.
            let media_lib = MediaLibrary::new(
                &tp,
                meta_targets,
                selection,
                SortOrder::Name,
            ).expect("Unable to create media library");

            // Run tests.
            let found: Vec<_> = media_lib.item_fps_from_meta_fp(tp.join("self.yml")).collect();
            assert_eq!(
                vec![
                    (tp.to_path_buf(), btreemap![
                        String::from("title") => MetaValue::String(String::from("PsyStyle Nation")),
                        String::from("artist") =>
                            MetaValue::Sequence(vec![
                                MetaValue::String(String::from("lapix")),
                                MetaValue::String(String::from("Massive New Krew")),
                            ]),
                    ])
                ],
                found
            );

            let found: Vec<_> = media_lib.item_fps_from_meta_fp(tp.join("item.yml")).collect();
            assert_eq!(
                vec![
                    (tp.join("item.flac"), btreemap![
                        String::from("artist") => MetaValue::String(String::from("lapix")),
                        String::from("title") => MetaValue::String(String::from("Black Mamba")),
                    ]),
                    (tp.join("subdir"), btreemap![
                        String::from("title") => MetaValue::String(String::from("What Is This?")),
                    ]),
                ],
                found
            );

            // let found: Vec<_> = media_lib.item_fps_from_meta_fp(tp.join("item.flac")).collect();
            // assert_eq!(vec![tp.join("item.yml")], found);

            // let found: Vec<_> = media_lib.item_fps_from_meta_fp(tp.join("subdir")).collect();
            // assert_eq!(vec![tp.join("subdir").join("self.yml"), tp.join("item.yml")], found);

            // let found: Vec<_> = media_lib.item_fps_from_meta_fp(tp.join("DOES_NOT_EXIST")).collect();
            // assert_eq!(Vec::<PathBuf>::new(), found);

            // let found: Vec<_> = media_lib.item_fps_from_meta_fp(tp.join("subdir").join("subitem.flac")).collect();
            // assert_eq!(Vec::<PathBuf>::new(), found);
        }

        // ASSOCIATED FUNCTIONS

        #[test]
        fn test_new() {
            // Create temp directory.
            let temp = TempDir::new("test_new").unwrap();
            let tp = temp.path();

            let db = DirBuilder::new();
            let dir_path = tp.join("test");

            db.create(&dir_path).unwrap();

            let ml = MediaLibrary::new(
                dir_path,
                vec![],
                Selection::True,
                SortOrder::Name,
            );

            assert!(ml.is_ok());

            let dir_path = tp.join("DOES_NOT_EXIST");

            let ml = MediaLibrary::new(
                dir_path,
                vec![],
                Selection::True,
                SortOrder::Name,
            );

            assert!(ml.is_err());
        }

        #[test]
        fn test_is_valid_item_name() {
            let inputs_and_expected = vec![
                ("simple", true),
                ("simple.ext", true),
                ("spaces ok", true),
                ("questions?", true),
                ("exclamation!", true),
                ("period.", true),
                (".period", true),
                ("", false),
                (".", false),
                ("..", false),
                ("/simple", false),
                ("./simple", false),
                ("simple/", false),
                ("simple/.", false),
                ("/", false),
                ("/simple/more", false),
                ("simple/more", false),
            ];

            for (input, expected) in inputs_and_expected {
                let produced = MediaLibrary::is_valid_item_name(input);
                assert_eq!(expected, produced);
            }
        }
    }
}

