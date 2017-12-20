pub mod selection;
pub mod sort_order;

use std::path::{Path, PathBuf};
use std::path::Component;
use std::ffi::OsString;
use std::fs::DirEntry;
use std::cmp::Ordering;
use std::time::SystemTime;
use std::collections::BTreeMap;

use regex::Regex;

use error::MediaLibraryError;
use path::normalize;
use metadata::{MetaBlock, MetaTarget};
use generator::gen_to_iter;
use yaml::{read_yaml_file, yaml_as_metadata};
use plexer::multiplex;

use self::selection::Selection;
use self::sort_order::SortOrder;

pub struct MediaLibrary {
    root_dir: PathBuf,
    meta_target_pairs: Vec<(String, MetaTarget)>,
    selection: Selection,
    sort_order: SortOrder,
}

impl MediaLibrary {

    // METHODS

    /// Creates a new `MediaLibrary`.
    /// The root path is canonicalized and converted into a PathBuf, and must point to a directory.
    pub fn new<P: AsRef<Path>>(
            root_dir: P,
            meta_target_pairs: Vec<(String, MetaTarget)>,
            selection: Selection,
            sort_order: SortOrder,
            ) -> Result<MediaLibrary, MediaLibraryError> {
        let root_dir = try!(root_dir.as_ref().canonicalize());

        if !root_dir.is_dir() {
            return Err(MediaLibraryError::NotADir(root_dir))
        }

        Ok(MediaLibrary {
            root_dir,
            meta_target_pairs,
            selection,
            sort_order,
        })
    }

    pub fn is_proper_sub_path<P: AsRef<Path>>(&self, abs_sub_path: P) -> bool {
        let abs_sub_path = normalize(abs_sub_path.as_ref());

        abs_sub_path.starts_with(&self.root_dir)
    }

    pub fn meta_fps_from_item_fp<P: AsRef<Path>>(&self, abs_item_path: P) -> Vec<PathBuf> {
        let abs_item_path = normalize(abs_item_path.as_ref());

        // Rule: item path must be proper.
        if !self.is_proper_sub_path(&abs_item_path) {
            return vec![]
        }

        // Rule: item path must exist.
        if !abs_item_path.exists() {
            return vec![]
        }

        let mut results: Vec<PathBuf> = vec![];

        for &(ref meta_file_name, ref meta_target) in &self.meta_target_pairs {
            if let Some(meta_target_dir_path) = meta_target.target_dir_path(&abs_item_path) {
                // Rule: target dir path must be proper.
                if !self.is_proper_sub_path(&meta_target_dir_path) {
                    continue;
                }

                let meta_file_path = meta_target_dir_path.join(meta_file_name);

                if !meta_file_path.exists() {
                    continue;
                }

                results.push(meta_file_path);
            }
        }

        results
    }

    pub fn item_fps_from_meta_fp<P: AsRef<Path>>(&self, abs_meta_path: P) -> Vec<(PathBuf, MetaBlock)> {
        println!("-------------------------------------------");
        let abs_meta_path = normalize(abs_meta_path.as_ref());

        // Rule: meta file path must be proper.
        if !self.is_proper_sub_path(&abs_meta_path) {
            return vec![]
        }

        // Rule: meta file path must exist and be a file.
        if !abs_meta_path.is_file() {
            return vec![]
        }

        let mut results: Vec<(PathBuf, MetaBlock)> = vec![];

        if let Some(working_dir_path) = abs_meta_path.parent() {
            // Rule: working dir path must be proper.
            if !self.is_proper_sub_path(&working_dir_path) {
                return vec![]
            }

            if let Some(found_meta_fn) = abs_meta_path.file_name().and_then(|s| s.to_str()) {
                // We have a meta file name, now try and match it to any of the file names in meta targets.
                if let Some(&(_, ref meta_target)) = self.meta_target_pairs.iter().find(|&&(ref s, _)| *s == found_meta_fn) {
                    // Read meta file, and parse.
                    if let Ok(y) = read_yaml_file(&abs_meta_path) {
                        if let Some(md) = yaml_as_metadata(&y, meta_target) {
                            let plex_results = multiplex(&md, &working_dir_path, &self.selection, self.sort_order);

                            for (plex_target, mb) in plex_results {
                                let item_path = plex_target.resolve(working_dir_path);

                                println!("{:?}", (item_path.clone(), mb));

                                results.push((item_path, mb.clone()));
                            }
                        }
                    }
                }
            }
        }

        results
    }

    // ASSOCIATED FUNCTIONS

    pub fn is_valid_item_name<S: AsRef<str>>(file_name: S) -> bool {
        let file_name = file_name.as_ref();
        let normed = normalize(Path::new(file_name));

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
    use std::collections::HashSet;
    use std::io::Write;

    use tempdir::TempDir;
    use regex::Regex;

    use metadata::{MetaTarget, MetaValue, MetaBlock};
    use library::{MediaLibrary, SortOrder};
    use library::selection::Selection;

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
    fn test_meta_fps_from_item_fp() {
        // Create temp directory.
        let temp = TempDir::new("test_meta_fps_from_item_fp").unwrap();
        let tp = temp.path();

        let db = DirBuilder::new();

        let meta_targets = vec![
            (String::from("self.yml"), MetaTarget::Contains),
            (String::from("item.yml"), MetaTarget::Siblings),
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
        db.create(tp.join("subdir")).unwrap();
        File::create(tp.join("item.flac")).unwrap();
        File::create(tp.join("subdir").join("subitem.flac")).unwrap();

        // Create meta files.
        let mut meta_file = File::create(tp.join("self.yml"))
            .expect("Unable to create metadata file");

        writeln!(meta_file, "title: PsyStyle Nation\nartist: [lapix, Massive New Krew]")
            .expect("Unable to write metadata file");

        let mut meta_file = File::create(tp.join("item.yml"))
            .expect("Unable to create metadata file");

        writeln!(meta_file, "item.flac:\n  title: Black Mamba\n  artist: lapix\nsubdir:\n  title: What Is This?")
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
        let found: Vec<_> = media_lib.meta_fps_from_item_fp(&tp);
        assert_eq!(vec![tp.join("self.yml")], found);

        let found: Vec<_> = media_lib.meta_fps_from_item_fp(tp.join("item.flac"));
        assert_eq!(vec![tp.join("item.yml")], found);

        let found: Vec<_> = media_lib.meta_fps_from_item_fp(tp.join("subdir"));
        assert_eq!(vec![tp.join("subdir").join("self.yml"), tp.join("item.yml")], found);

        let found: Vec<_> = media_lib.meta_fps_from_item_fp(tp.join("DOES_NOT_EXIST"));
        assert_eq!(Vec::<PathBuf>::new(), found);

        let found: Vec<_> = media_lib.meta_fps_from_item_fp(tp.join("subdir").join("subitem.flac"));
        assert_eq!(Vec::<PathBuf>::new(), found);
    }

    #[test]
    fn test_item_fps_from_meta_fp() {
        // Create temp directory.
        let temp = TempDir::new("test_meta_fps_from_item_fp").unwrap();
        let tp = temp.path();

        let db = DirBuilder::new();

        let meta_targets_map = vec![
            (String::from("self.yml"), MetaTarget::Contains),
            (String::from("item_map.yml"), MetaTarget::Siblings),
        ];
        let meta_targets_seq = vec![
            (String::from("self.yml"), MetaTarget::Contains),
            (String::from("item_seq.yml"), MetaTarget::Siblings),
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
        db.create(tp.join("subdir")).unwrap();
        File::create(tp.join("item.flac")).unwrap();
        File::create(tp.join("subdir").join("subitem.flac")).unwrap();

        // Create meta files.
        let mut meta_file = File::create(tp.join("self.yml"))
            .expect("Unable to create metadata file");

        writeln!(meta_file, "title: PsyStyle Nation\nartist: [lapix, Massive New Krew]")
            .expect("Unable to write metadata file");

        let mut meta_file = File::create(tp.join("item_map.yml"))
            .expect("Unable to create metadata file");

        writeln!(meta_file, "item.flac:\n  title: Black Mamba\n  artist: lapix\nsubdir:\n  title: What Is This?")
            .expect("Unable to write metadata file");

        let mut meta_file = File::create(tp.join("item_seq.yml"))
            .expect("Unable to create metadata file");

        writeln!(meta_file, "- title: Black Mamba\n  artist: lapix\n- title: What Is This?")
            .expect("Unable to write metadata file");

        let mut meta_file = File::create(tp.join("subdir").join("self.yml"))
            .expect("Unable to create metadata file");

        writeln!(meta_file, "title: A Subtrack?\nartist: Massive New Krew")
            .expect("Unable to write metadata file");

        // Create media library.
        let media_lib_map = MediaLibrary::new(
            &tp,
            meta_targets_map,
            selection.clone(),
            SortOrder::Name,
        ).expect("Unable to create media library");

        let media_lib_seq = MediaLibrary::new(
            &tp,
            meta_targets_seq,
            selection.clone(),
            SortOrder::ModTime,
        ).expect("Unable to create media library");

        // Run tests.
        let found: Vec<_> = media_lib_map.item_fps_from_meta_fp(tp.join("self.yml"));
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

        let found: Vec<_> = media_lib_map.item_fps_from_meta_fp(tp.join("item_map.yml"));
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

        let found: Vec<_> = media_lib_seq.item_fps_from_meta_fp(tp.join("item_seq.yml"));
        assert_eq!(
            vec![
                (tp.join("subdir"), btreemap![
                    String::from("artist") => MetaValue::String(String::from("lapix")),
                    String::from("title") => MetaValue::String(String::from("Black Mamba")),
                ]),
                (tp.join("item.flac"), btreemap![
                    String::from("title") => MetaValue::String(String::from("What Is This?")),
                ]),
            ],
            found
        );

        let found: Vec<_> = media_lib_map.item_fps_from_meta_fp(tp.join("subdir").join("self.yml"));
        assert_eq!(
            vec![
                (tp.join("subdir"), btreemap![
                    String::from("title") => MetaValue::String(String::from("A Subtrack?")),
                    String::from("artist") => MetaValue::String(String::from("Massive New Krew")),
                ])
            ],
            found
        );

        let found: Vec<_> = media_lib_map.item_fps_from_meta_fp(tp.join("DOES_NOT_EXIST"));
        assert_eq!(Vec::<(PathBuf, MetaBlock)>::new(), found);
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

