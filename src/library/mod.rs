pub mod selection;
pub mod sort_order;

use std::path::{Path, PathBuf};
use std::error::Error;
use std::io::Error as IoError;
use std::fmt::{Formatter, Result as FmtResult, Display};

use helpers::normalize;
use metadata::{MetaBlock, MetaTarget};
use yaml::{read_yaml_file, yaml_as_metadata, YamlError};
use plexer::multiplex;

use self::selection::Selection;
use self::sort_order::SortOrder;

#[derive(Debug)]
pub enum MediaLibraryError {
    NotADir(PathBuf),
    NotAFile(PathBuf),
    DoesNotExist(PathBuf),
    IoError(IoError),
    InvalidSubPath(PathBuf, PathBuf),
    YamlError(YamlError),
    UnknownTarget,
    // NonAbsPath(path::PathBuf),
    // NonRelPath(path::PathBuf),
}

impl Error for MediaLibraryError {
    // LEARN: This is meant to be a static description of the error, without any dynamic creation.
    fn description(&self) -> &str {
        match *self {
            MediaLibraryError::NotADir(_) => "File path did not point to an existing directory",
            MediaLibraryError::NotAFile(_) => "File path did not point to an existing file",
            MediaLibraryError::DoesNotExist(_) => "File path does not exist",
            MediaLibraryError::IoError(ref e) => e.description(),
            MediaLibraryError::InvalidSubPath(_, _) => "Sub path was not a descendant of root directory",
            MediaLibraryError::YamlError(ref e) => e.description(),
            MediaLibraryError::UnknownTarget => "Meta target was not found",
            // MediaLibraryError::NonAbsPath(_) => "File path was expected to be absolute",
            // MediaLibraryError::NonRelPath(_) => "File path was expected to be relative",
        }
    }
}

impl Display for MediaLibraryError {
    // LEARN: This is the place to put dynamically-created error messages.
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match *self {
            MediaLibraryError::NotADir(ref p) => write!(f, r##"Path "{}" is not an existing directory"##, p.to_string_lossy()),
            MediaLibraryError::NotAFile(ref p) => write!(f, r##"Path "{}" is not an existing file"##, p.to_string_lossy()),
            MediaLibraryError::DoesNotExist(ref p) => write!(f, r##"Path "{}" is does not exist"##, p.to_string_lossy()),
            MediaLibraryError::IoError(ref e) => e.fmt(f),
            MediaLibraryError::InvalidSubPath(ref p, ref r) => {
                write!(f, r##"Sub path "{}" is not a descendant of root directory "{}""##,
                    p.to_string_lossy(),
                    r.to_string_lossy(),
                )
            },
            MediaLibraryError::YamlError(ref e) => e.fmt(f),
            MediaLibraryError::UnknownTarget => self.description().fmt(f),
            // MediaLibraryError::NonAbsPath(ref p) => write!(f, r##"Path {:?} is not absolute"##, p),
            // MediaLibraryError::NonRelPath(ref p) => write!(f, r##"Path {:?} is not relative"##, p),
        }
    }
}

impl From<IoError> for MediaLibraryError {
    // LEARN: This makes it easy to compose other error types into our own error type.
    fn from(err: IoError) -> MediaLibraryError {
        MediaLibraryError::IoError(err)
    }
}

impl From<YamlError> for MediaLibraryError {
    // LEARN: This makes it easy to compose other error types into our own error type.
    fn from(err: YamlError) -> MediaLibraryError {
        MediaLibraryError::YamlError(err)
    }
}

pub struct MediaLibrary {
    root_dir: PathBuf,
    meta_target_pairs: Vec<(String, MetaTarget)>,
    selection: Selection,
    sort_order: SortOrder,
}

impl MediaLibrary {
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

    pub fn meta_fps_from_item_fp<P: AsRef<Path>>(&self, abs_item_path: P) -> Result<Vec<PathBuf>, MediaLibraryError> {
        let abs_item_path = normalize(abs_item_path.as_ref());

        // Rule: item path must be proper.
        if !self.is_proper_sub_path(&abs_item_path) {
            error!(r#"Item path "{}" is not a proper subpath of "{}""#, abs_item_path.to_string_lossy(), self.root_dir.to_string_lossy());
            return Err(MediaLibraryError::InvalidSubPath(abs_item_path, self.root_dir.clone()))
        }

        // Rule: item path must exist.
        if !abs_item_path.exists() {
            error!(r#"Item path "{}" does not exist"#, abs_item_path.to_string_lossy());
            return Err(MediaLibraryError::DoesNotExist(abs_item_path))
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

        Ok(results)
    }

    pub fn item_fps_from_meta_fp<P: AsRef<Path>>(&self, abs_meta_path: P) -> Result<Vec<(PathBuf, MetaBlock)>, MediaLibraryError> {
        let abs_meta_path = normalize(abs_meta_path.as_ref());

        // Rule: meta file path must be proper.
        if !self.is_proper_sub_path(&abs_meta_path) {
            error!(r#"Item path "{}" is not a proper subpath of "{}""#, abs_meta_path.to_string_lossy(), self.root_dir.to_string_lossy());
            return Err(MediaLibraryError::InvalidSubPath(abs_meta_path, self.root_dir.clone()))
        }

        // Rule: meta file path must exist and be a file.
        if !abs_meta_path.is_file() {
            error!(r#"Item path "{}" is not a valid file"#, abs_meta_path.to_string_lossy());
            return Err(MediaLibraryError::NotAFile(abs_meta_path))
        }

        let mut results: Vec<(PathBuf, MetaBlock)> = vec![];

        if let Some(working_dir_path) = abs_meta_path.parent() {
            // // Rule: working dir path must be proper.
            // if !self.is_proper_sub_path(&working_dir_path) {
            //     return vec![]
            // }

            if let Some(found_meta_fn) = abs_meta_path.file_name().and_then(|s| s.to_str()) {
                // We have a meta file name, now try and match it to any of the file names in meta targets.
                match self.meta_target_pairs.iter().find(|&&(ref s, _)| *s == found_meta_fn) {
                    Some(&(_, ref meta_target)) => {
                        // Read meta file, and parse.
                        let yaml_data = read_yaml_file(&abs_meta_path)?;

                        match yaml_as_metadata(&yaml_data, meta_target) {
                            Some(md) => {
                                let plex_results = multiplex(&md, &working_dir_path, &self.selection, self.sort_order, true);

                                for (plex_target, mb) in plex_results {
                                    let item_path = plex_target.resolve(working_dir_path);

                                    results.push((item_path, mb.clone()));
                                }
                            },
                            None => { println!("NO METADATA FOUND!!!"); },
                        }

                        // if let Some(md) = yaml_as_metadata(&yaml_data, meta_target) {
                        //     let plex_results = multiplex(&md, &working_dir_path, &self.selection, self.sort_order, true);

                        //     for (plex_target, mb) in plex_results {
                        //         let item_path = plex_target.resolve(working_dir_path);

                        //         results.push((item_path, mb.clone()));
                        //     }
                        // }
                    },
                    None => {
                        return Err(MediaLibraryError::UnknownTarget)
                    },
                }
            }
        }

        Ok(results)
    }
}

// =================================================================================================
// TESTS
// =================================================================================================


#[cfg(test)]
mod tests {
    use std::path::{PathBuf};
    use std::fs::{File, DirBuilder};
    use std::io::Write;
    use std::thread::sleep;
    use std::time::Duration;

    use tempdir::TempDir;

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
        sleep(Duration::from_millis(5));
        File::create(tp.join("item.flac")).unwrap();
        sleep(Duration::from_millis(5));
        File::create(tp.join("subdir").join("subitem.flac")).unwrap();
        sleep(Duration::from_millis(5));

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
        let found: Vec<_> = media_lib.meta_fps_from_item_fp(&tp).expect("Unable to get meta fps");
        assert_eq!(vec![tp.join("self.yml")], found);

        let found: Vec<_> = media_lib.meta_fps_from_item_fp(tp.join("item.flac")).expect("Unable to get meta fps");
        assert_eq!(vec![tp.join("item.yml")], found);

        let found: Vec<_> = media_lib.meta_fps_from_item_fp(tp.join("subdir")).expect("Unable to get meta fps");
        assert_eq!(vec![tp.join("subdir").join("self.yml"), tp.join("item.yml")], found);

        assert!(media_lib.meta_fps_from_item_fp(tp.join("DOES_NOT_EXIST")).is_err());

        let found: Vec<_> = media_lib.meta_fps_from_item_fp(tp.join("subdir").join("subitem.flac")).expect("Unable to get meta fps");
        assert_eq!(Vec::<PathBuf>::new(), found);
    }

    #[test]
    fn test_item_fps_from_meta_fp() {
        // Create temp directory.
        let temp = TempDir::new("test_item_fps_from_meta_fp").unwrap();
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
        sleep(Duration::from_millis(5));
        File::create(tp.join("item.flac")).unwrap();
        sleep(Duration::from_millis(5));
        File::create(tp.join("subdir").join("subitem.flac")).unwrap();
        sleep(Duration::from_millis(5));

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
        let found: Vec<_> = media_lib_map.item_fps_from_meta_fp(tp.join("self.yml")).expect("Unable to get item fps");
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

        let found: Vec<_> = media_lib_map.item_fps_from_meta_fp(tp.join("item_map.yml")).expect("Unable to get item fps");
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

        let found: Vec<_> = media_lib_seq.item_fps_from_meta_fp(tp.join("item_seq.yml")).expect("Unable to get item fps");
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

        let found: Vec<_> = media_lib_map.item_fps_from_meta_fp(tp.join("subdir").join("self.yml")).expect("Unable to get item fps");
        assert_eq!(
            vec![
                (tp.join("subdir"), btreemap![
                    String::from("title") => MetaValue::String(String::from("A Subtrack?")),
                    String::from("artist") => MetaValue::String(String::from("Massive New Krew")),
                ])
            ],
            found
        );

        assert!(media_lib_map.item_fps_from_meta_fp(tp.join("DOES_NOT_EXIST")).is_err());
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
}

