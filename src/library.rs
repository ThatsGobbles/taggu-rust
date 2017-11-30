use std::path::Path;
use std::path::PathBuf;
use std::path::Component;
use std::ffi::OsString;
use std::fs::DirEntry;
use std::cmp::Ordering;
use std::time::SystemTime;

use regex::Regex;

use error::MediaLibraryError;
use path::normalize;
use metadata::MetaBlock;

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

pub struct MediaLibrary {
    root_dir: PathBuf,
    item_meta_fn: String,
    self_meta_fn: String,
    selection: Selection,
    sort_order: SortOrder,
}

impl MediaLibrary {
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

    pub fn is_valid_sub_path<P: Into<PathBuf>>(&self, abs_sub_path: P) -> bool {
        let abs_sub_path = normalize(&abs_sub_path.into());

        abs_sub_path.starts_with(&self.root_dir)
    }

    pub fn is_selected_media_item<P: Into<PathBuf>>(&self, abs_item_path: P) -> bool {
        let abs_item_path = normalize(&abs_item_path.into());

        self.is_valid_sub_path(&abs_item_path) && MediaLibrary::is_media_path(&abs_item_path, &self.selection)
    }

    pub fn get_contains_dir<P: Into<PathBuf>>(&self, abs_item_path: P) -> Option<PathBuf> {
        let abs_item_path = normalize(&abs_item_path.into());

        if self.is_valid_sub_path(&abs_item_path) && abs_item_path.is_dir() {
            return Some(abs_item_path)
        }

        None
    }

    pub fn get_siblings_dir<P: Into<PathBuf>>(&self, abs_item_path: P) -> Option<PathBuf> {
        let abs_item_path = normalize(&abs_item_path.into());

        // Assume that .parent() returns None if given a root or prefix.
        if let Some(parent_dir) = abs_item_path.parent() {
            if self.is_valid_sub_path(&parent_dir) {
                return Some(parent_dir.to_path_buf())
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// Helper methods
////////////////////////////////////////////////////////////////////////////////////////////////////

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
    use std::path::PathBuf;
    use tempdir::TempDir;
    use std::fs::{File, DirBuilder};
    use regex::Regex;

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
}
