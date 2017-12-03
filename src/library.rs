use std::path::{Path, PathBuf};
use std::path::Component;
use std::ffi::OsString;
use std::fs::DirEntry;
use std::cmp::Ordering;
use std::time::SystemTime;

use regex::Regex;

use error::MediaLibraryError;
use path::normalize;

use generator::gen_to_iter;

#[derive(Debug)]
pub enum Selection {
    Ext(String),
    Regex(Regex),
    IsFile,
    IsDir,
    And(Box<Selection>, Box<Selection>),
    Or(Box<Selection>, Box<Selection>),
    Xor(Box<Selection>, Box<Selection>),
    Not(Box<Selection>),
    True,
    False,
}

pub enum SortOrder {
    Name,
    ModTime,
}

pub enum MetaTarget {
    Alongside,
    Container,
}

impl MetaTarget {
    pub fn priority_order<'a>() -> impl Iterator<Item = MetaTarget> + 'a {
        gen_to_iter(|| {
            yield MetaTarget::Container;
            yield MetaTarget::Alongside;
        })
    }
}

// pub struct MediaLibrary {
//     root_dir: PathBuf,
//     meta_targets: Vec<MetaTarget>,
//     selection: Selection,
//     sort_order: SortOrder,
// }

pub struct MediaLibrary {
    root_dir: PathBuf,
    item_meta_fn: String,
    self_meta_fn: String,
    selection: Selection,
    sort_order: SortOrder,
}

impl MediaLibrary {

// *************************************************************************************************
// Methods
// *************************************************************************************************

    /// Creates a new `MediaLibrary`.
    /// The root path is canonicalized and converted into a PathBuf, and must point to a directory.
    pub fn new<P: Into<PathBuf>, S: Into<String>>(
            root_dir: P,
            item_meta_fn: S,
            self_meta_fn: S,
            selection: Selection,
            sort_order: SortOrder,
            ) -> Result<MediaLibrary, MediaLibraryError> {
        let root_dir = try!(root_dir.into().canonicalize());

        if !root_dir.is_dir() {
            return Err(MediaLibraryError::NotADir(root_dir))
        }

        Ok(MediaLibrary {
            root_dir,
            item_meta_fn: item_meta_fn.into(),
            self_meta_fn: self_meta_fn.into(),
            selection,
            sort_order,
        })
    }

    pub fn is_proper_sub_path<P: Into<PathBuf>>(&self, abs_sub_path: P) -> bool {
        let abs_sub_path = normalize(&abs_sub_path.into());

        abs_sub_path.starts_with(&self.root_dir)
    }

    pub fn is_selected_media_item<P: Into<PathBuf>>(&self, abs_item_path: P) -> bool {
        let abs_item_path = normalize(&abs_item_path.into());

        self.is_proper_sub_path(&abs_item_path) && MediaLibrary::is_media_path(&abs_item_path, &self.selection)
    }

    /// Returns the 'contains' directory path, if any, for a given item path.
    /// An item path has a 'contains' directory path if ALL of the following apply:
    /// 1) The item path is a proper subpath.
    /// 2) The item path points to an existing directory.
    pub fn get_contains_dir<P: Into<PathBuf>>(&self, abs_item_path: P) -> Option<PathBuf> {
        let abs_item_path = normalize(&abs_item_path.into());

        if self.is_proper_sub_path(&abs_item_path) && abs_item_path.is_dir() {
            return Some(abs_item_path)
        }

        None
    }

    /// Returns the 'siblings' directory path, if any, for a given item path.
    /// An item path has a 'siblings' directory path if ALL of the following apply:
    /// 1) The item path is a proper subpath.
    /// 2) The item path exists.
    /// 3) The item path has a valid parent directory.
    /// 4) The item path's parent directory is a proper subpath.
    pub fn get_siblings_dir<P: Into<PathBuf>>(&self, abs_item_path: P) -> Option<PathBuf> {
        let abs_item_path = normalize(&abs_item_path.into());

        if self.is_proper_sub_path(&abs_item_path) && abs_item_path.exists() {
            // Assume that .parent() returns None if given a root or prefix.
            if let Some(parent_dir) = abs_item_path.parent() {
                if self.is_proper_sub_path(&parent_dir) {
                    return Some(parent_dir.to_path_buf())
                }
            }
        }

        None
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

    pub fn sort_entries<I: IntoIterator<Item = DirEntry>>(&self, entries: I) -> Vec<DirEntry> {
        // LEARN: Why does the commented-out code not work?
        // let cmp = |a, b| dir_entry_sort_cmp(a, b, &self.sort_order);
        let mut res: Vec<DirEntry> = entries.into_iter().collect();
        // res.sort_by(cmp);
        res.sort_by(|a, b| MediaLibrary::dir_entry_sort_cmp(a, b, &self.sort_order));
        res
    }

    pub fn get_meta_file_name(&self, mt: &MetaTarget) -> &String {
        match *mt {
            MetaTarget::Alongside => &self.item_meta_fn,
            MetaTarget::Container => &self.self_meta_fn,
        }
    }

    pub fn get_target_meta_dir<P: Into<PathBuf>>(&self, abs_item_path: P, mt: &MetaTarget) -> Option<PathBuf> {
        let abs_item_path = normalize(&abs_item_path.into());

        if self.is_proper_sub_path(&abs_item_path) && abs_item_path.exists() {
            match *mt {
                MetaTarget::Container => {
                    if abs_item_path.is_dir() {
                        return Some(abs_item_path)
                    }
                },
                MetaTarget::Alongside => {
                    if let Some(parent_dir) = abs_item_path.parent() {
                        if self.is_proper_sub_path(&parent_dir) {
                            return Some(parent_dir.to_path_buf())
                        }
                    }
                }
            }
        }

        None
    }

// *************************************************************************************************
// Associated functions
// *************************************************************************************************

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

    fn is_media_path<P: Into<PathBuf>>(abs_item_path: P, sel: &Selection) -> bool {
        let abs_item_path = normalize(&abs_item_path.into());

        if !abs_item_path.exists() {
            return false
        }

        match sel {
            &Selection::Ext(ref e_ext) => {
                if let Some(p_ext) = abs_item_path.extension() {
                    OsString::from(e_ext) == p_ext
                } else {
                    false
                }
            },
            &Selection::Regex(ref r_exp) => {
                let maybe_fn = abs_item_path.file_name().map(|x| x.to_str());

                if let Some(Some(fn_str)) = maybe_fn {
                    r_exp.is_match(fn_str)
                } else {
                    false
                }
            },
            &Selection::IsFile => abs_item_path.is_file(),
            &Selection::IsDir => abs_item_path.is_dir(),
            &Selection::And(ref sel_a, ref sel_b) => {
                MediaLibrary::is_media_path(&abs_item_path, &sel_a)
                && MediaLibrary::is_media_path(&abs_item_path, &sel_b)
            },
            &Selection::Or(ref sel_a, ref sel_b) => {
                MediaLibrary::is_media_path(&abs_item_path, &sel_a)
                || MediaLibrary::is_media_path(&abs_item_path, &sel_b)
            },
            &Selection::Xor(ref sel_a, ref sel_b) => {
                MediaLibrary::is_media_path(&abs_item_path, &sel_a)
                ^ MediaLibrary::is_media_path(&abs_item_path, &sel_b)
            },
            &Selection::Not(ref sel) => !MediaLibrary::is_media_path(&abs_item_path, &sel),
            &Selection::True => true,
            &Selection::False => false,
        }
    }

    fn get_mtime<P: Into<PathBuf>>(abs_path: P) -> Option<SystemTime> {
        let abs_path = abs_path.into();
        if let Ok(metadata) = abs_path.metadata() {
            if let Ok(mtime) = metadata.modified() {
                return Some(mtime)
            }
        }

        None
    }

    fn path_sort_cmp<P: Into<PathBuf>>(abs_item_path_a: P, abs_item_path_b: P, sort_ord: &SortOrder) -> Ordering {
        let abs_item_path_a = abs_item_path_a.into();
        let abs_item_path_b = abs_item_path_b.into();

        match sort_ord {
            &SortOrder::Name => abs_item_path_a.file_name().cmp(&abs_item_path_b.file_name()),
            &SortOrder::ModTime => {
                let m_time_a = MediaLibrary::get_mtime(abs_item_path_a);
                let m_time_b = MediaLibrary::get_mtime(abs_item_path_b);

                m_time_a.cmp(&m_time_b)
            },
        }
    }

    fn dir_entry_sort_cmp(dir_entry_a: &DirEntry, dir_entry_b: &DirEntry, sort_ord: &SortOrder) -> Ordering {
        MediaLibrary::path_sort_cmp(dir_entry_a.path(), dir_entry_b.path(), &sort_ord)
    }
}

#[cfg(test)]
mod tests {
    use super::{MediaLibrary, SortOrder, Selection};
    use std::path::{PathBuf, Path};
    use tempdir::TempDir;
    use std::fs::{File, DirBuilder};
    use regex::Regex;
    use std::collections::HashSet;

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

// *************************************************************************************************
// Methods
// *************************************************************************************************

    #[test]
    fn test_is_proper_sub_path() {
        // Create temp directory.
        let temp = TempDir::new("test_is_proper_sub_path").unwrap();
        let tp = temp.path();

        let ml = MediaLibrary::new(
            tp,
            "item_meta.yml",
            "self_meta.yml",
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
    fn test_get_contains_dir() {
        // Create temp directory.
        let temp = TempDir::new("test_get_contains_dir").unwrap();
        let tp = temp.path();

        let ml = MediaLibrary::new(
            tp,
            "item_meta.yml",
            "self_meta.yml",
            Selection::True,
            SortOrder::Name,
        ).unwrap();

        // Generate desired file and dir paths.
        let fns = vec!["a", "b", "c"];

        let db = DirBuilder::new();

        let mut inputs_and_expected: Vec<(PathBuf, Option<PathBuf>)> = vec![
            (tp.to_path_buf(), Some(tp.to_path_buf())),
        ];

        let mut curr_dir: PathBuf = tp.to_path_buf();
        for _ in (0..3) {
            // Create files in current directory.
            for f in &fns {
                let curr_f = curr_dir.join(f);
                File::create(&curr_f).unwrap();
                inputs_and_expected.push((curr_f, None));
            }

            // Create the next directory.
            curr_dir = curr_dir.join("sub");
            db.create(&curr_dir);
            inputs_and_expected.push((curr_dir.clone(), Some(curr_dir.clone())));
        }

        inputs_and_expected.push((tp.join("DOES_NOT_EXIST"), None));

        for (input, expected) in inputs_and_expected {
            let produced = ml.get_contains_dir(&input);
            assert_eq!(expected, produced);
        }
    }

    #[test]
    fn test_get_siblings_dir() {
        // Create temp directory.
        let temp = TempDir::new("test_get_siblings_dir").unwrap();
        let tp = temp.path();

        let ml = MediaLibrary::new(
            tp,
            "item_meta.yml",
            "self_meta.yml",
            Selection::True,
            SortOrder::Name,
        ).unwrap();

        // Generate desired file and dir paths.
        let fns = vec!["a", "b", "c"];

        let db = DirBuilder::new();

        let mut inputs_and_expected: Vec<(PathBuf, Option<PathBuf>)> = vec![
            (tp.to_path_buf(), None),
        ];

        let mut curr_dir: PathBuf = tp.to_path_buf();
        for _ in (0..3) {
            // Create files in current directory.
            for f in &fns {
                let curr_f = curr_dir.join(f);
                File::create(&curr_f).unwrap();
                inputs_and_expected.push(
                    (curr_f, Some(curr_dir.to_path_buf()))
                );
            }

            // Create the next directory.
            let old_dir = curr_dir.to_path_buf();
            curr_dir = curr_dir.join("sub");
            db.create(&curr_dir);
            inputs_and_expected.push((curr_dir.to_path_buf(), Some(old_dir)));
        }

        inputs_and_expected.push((tp.join("DOES_NOT_EXIST"), None));

        for (input, expected) in inputs_and_expected {
            let produced = ml.get_siblings_dir(&input);
            assert_eq!(expected, produced);
        }
    }

    #[test]
    fn test_all_entries_in_dir() {
        // Create temp directory.
        let temp = TempDir::new("test_all_entries_in_dir").unwrap();
        let tp = temp.path();

        // let paths_to_create = vec![
        //     tp.join("file_a.flac"),
        //     tp.join("file_b.flac"),
        //     tp.join("file_c.flac"),
        //     tp.join("file_d.yml"),
        //     tp.join("file_e.jpg"),
        // ];

        // for path_to_create in &paths_to_create {
        //     File::create(&path_to_create).unwrap();
        // }

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
            "item_meta.yml",
            "self_meta.yml",
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
            "item_meta.yml",
            "self_meta.yml",
            Selection::True,
            SortOrder::Name,
        ).unwrap();

        let expected: HashSet<PathBuf> = paths_to_create.iter().map(|fp| fp.to_path_buf()).collect();
        let produced: HashSet<PathBuf> = ml.selected_entries_in_dir(tp).map(|e| e.path().to_path_buf()).collect();

        assert_eq!(expected, produced);

        let ml = MediaLibrary::new(
            tp,
            "item_meta.yml",
            "self_meta.yml",
            Selection::Ext("flac".to_string()),
            SortOrder::Name,
        ).unwrap();

        let expected: HashSet<PathBuf> = paths_to_create.iter().take(3).map(|fp| fp.to_path_buf()).collect();
        let produced: HashSet<PathBuf> = ml.selected_entries_in_dir(tp).map(|e| e.path().to_path_buf()).collect();

        assert_eq!(expected, produced);

        let ml = MediaLibrary::new(
            tp,
            "item_meta.yml",
            "self_meta.yml",
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

// *************************************************************************************************
// Associated functions
// *************************************************************************************************

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
            "item_meta.yml",
            "self_meta.yml",
            Selection::True,
            SortOrder::Name,
        );

        assert!(ml.is_ok());

        let dir_path = tp.join("DOES_NOT_EXIST");

        let ml = MediaLibrary::new(
            dir_path,
            "item_meta.yml",
            "self_meta.yml",
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

    #[test]
    fn test_is_media_path() {
        // Create temp directory.
        let temp = TempDir::new("test_is_media_path").unwrap();
        let tp = temp.path();

        // Generate desired file and dir paths.
        let mut paths_and_flags: Vec<(PathBuf, bool)> = Vec::new();

        let exts = vec!["flac", "ogg",];
        let suffixes = vec!["_a", "_b", "_aa",];

        for suffix in &suffixes {
            let f_path = tp.join(format!("file{}", suffix));
            paths_and_flags.push((f_path, false));

            let d_path = tp.join(format!("dir{}", suffix));
            paths_and_flags.push((d_path, true));

            for ext in &exts {
                let f_path = tp.join(format!("file{}.{}", suffix, ext));
                paths_and_flags.push((f_path, false));

                let d_path = tp.join(format!("dir{}.{}", suffix, ext));
                paths_and_flags.push((d_path, true));
            }
        }

        // Create the files and dirs.
        let db = DirBuilder::new();
        for &(ref path, is_dir) in &paths_and_flags {
            if is_dir {
                db.create(path).unwrap();
            } else {
                File::create(path).unwrap();
            }
        }

        // Test cases and indices of paths that should pass.
        let selections_and_true_indices = vec![
            (Selection::IsFile, vec![0: usize, 2, 4, 6, 8, 10, 12, 14, 16]),
            (Selection::IsDir, vec![1, 3, 5, 7, 9, 11, 13, 15, 17]),
            (Selection::Ext("flac".to_string()), vec![2, 3, 8, 9, 14, 15]),
            (Selection::Ext("ogg".to_string()), vec![4, 5, 10, 11, 16, 17]),
            (Selection::Regex(Regex::new(r".*_a\..*").unwrap()), vec![2, 3, 4, 5]),
            (Selection::And(
                Box::new(Selection::IsFile),
                Box::new(Selection::Ext("ogg".to_string())),
            ), vec![4, 10, 16]),
            (Selection::Or(
                Box::new(Selection::Ext("ogg".to_string())),
                Box::new(Selection::Ext("flac".to_string())),
            ), vec![2, 3, 4, 5, 8, 9, 10, 11, 14, 15, 16, 17]),
            (Selection::Or(
                Box::new(Selection::IsDir),
                Box::new(Selection::And(
                    Box::new(Selection::IsFile),
                    Box::new(Selection::Ext("flac".to_string())),
                )),
            ), vec![1, 2, 3, 5, 7, 8, 9, 11, 13, 14, 15, 17]),
            // TODO: Add Xor case.
            (Selection::Not(
                Box::new(Selection::IsFile),
            ), vec![1, 3, 5, 7, 9, 11, 13, 15, 17]),
            (Selection::Not(
                Box::new(Selection::Ext("flac".to_string())),
            ), vec![0, 1, 4, 5, 6, 7, 10, 11, 12, 13, 16, 17]),
            (Selection::True, (0..18).collect()),
            (Selection::False, vec![]),
        ];

        // Run the tests.
        for (selection, true_indices) in selections_and_true_indices {
            for (index, &(ref abs_path, _)) in paths_and_flags.iter().enumerate() {
                let expected = true_indices.contains(&index);
                let produced = MediaLibrary::is_media_path(&abs_path, &selection);
                // println!("{:?}, {:?}", abs_path, selection);
                assert_eq!(expected, produced);
            }
        }
    }

    #[test]
    fn test_get_mtime() {
        // Create temp directory.
        let temp = TempDir::new("test_get_mtime").unwrap();
        let tp = temp.path();

        // Create and test temp files and directories.
        let db = DirBuilder::new();

        let path = tp.join("file.txt");
        File::create(&path).unwrap();
        assert!(MediaLibrary::get_mtime(&path).is_some());

        let path = tp.join("dir");
        db.create(&path).unwrap();
        assert!(MediaLibrary::get_mtime(&path).is_some());

        let path = tp.join("NON_EXISTENT");
        assert!(MediaLibrary::get_mtime(&path).is_none());
    }
}
