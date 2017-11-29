use std::path::Path;
use std::path::PathBuf;
use std::path::Component;
use std::ffi::OsString;
use std::fs::DirEntry;
use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

use regex::Regex;

use error::MediaLibraryError;
use path::normalize;
use metadata::MetaBlock;

use generator::gen_to_iter;

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

////////////////////////////////////////////////////////////////////////////////////////////////////

pub fn example() {
    // let selection = Selection::Or(
    //     Box::new(Selection::IsDir),
    //     Box::new(Selection::And(
    //         Box::new(Selection::IsFile),
    //         Box::new(Selection::Ext("flac".to_string())),
    //     )),
    // );

    // let media_lib = MediaLibrary::new("/home/lemoine/Music",
    //         "taggu_item.yml",
    //         "taggu_self.yml",
    //         // Selection::IsFile,
    //         selection,
    //         SortOrder::Name,
    // ).unwrap();

    // println!("UNFILTERED");
    // let a_entries: Vec<DirEntry> = media_lib.all_entries_in_dir("BASS AVENGERS").collect();
    // for dir_entry in a_entries {
    //     println!("{:?}", dir_entry);
    // }

    // println!("FILTERED");
    // let f_entries: Vec<DirEntry> = media_lib.filtered_entries_in_dir("BASS AVENGERS").collect();
    // for dir_entry in f_entries {
    //     println!("{:?}", dir_entry);
    // }

    // println!("SOME SORTING");
    // let s_entries: Vec<DirEntry> = media_lib.sort_entries(media_lib.filtered_entries_in_dir("BASS AVENGERS"));
    // for dir_entry in s_entries {
    //     println!("{:?}", dir_entry);
    // }

    // let selection = Selection::Or(
    //     Box::new(Selection::IsDir),
    //     Box::new(Selection::And(
    //         Box::new(Selection::IsFile),
    //         Box::new(Selection::Ext("flac".to_string())),
    //     )),
    // );

    // let media_lib = MediaLibrary::new("/home/lemoine/Music",
    //         "taggu_item.yml",
    //         "taggu_self.yml",
    //         // Selection::IsFile,
    //         selection,
    //         SortOrder::ModTime,
    // ).unwrap();

    // println!("UNFILTERED, SORTED BY MTIME");
    // let m_entries: Vec<DirEntry> = media_lib.sort_entries(media_lib.all_entries_in_dir("BASS AVENGERS"));
    // for dir_entry in m_entries {
    //     println!("{:?}", dir_entry);
    // }
}

#[cfg(test)]
mod tests {
    use super::{MediaLibrary, SortOrder, Selection};
    use std::path::PathBuf;
    use tempdir::TempDir;

    #[test]
    fn test_is_valid_item_name() {
        assert_eq!(true, MediaLibrary::is_valid_item_name("simple"));
        assert_eq!(true, MediaLibrary::is_valid_item_name("simple.ext"));
        assert_eq!(true, MediaLibrary::is_valid_item_name("spaces ok"));
        assert_eq!(true, MediaLibrary::is_valid_item_name("questions?"));
        assert_eq!(true, MediaLibrary::is_valid_item_name("exclamation!"));
        assert_eq!(true, MediaLibrary::is_valid_item_name("period."));
        assert_eq!(true, MediaLibrary::is_valid_item_name(".period"));

        assert_eq!(false, MediaLibrary::is_valid_item_name(""));
        assert_eq!(false, MediaLibrary::is_valid_item_name("."));
        assert_eq!(false, MediaLibrary::is_valid_item_name(".."));
        assert_eq!(false, MediaLibrary::is_valid_item_name("/simple"));
        assert_eq!(false, MediaLibrary::is_valid_item_name("simple/"));
        assert_eq!(false, MediaLibrary::is_valid_item_name("/"));
        assert_eq!(false, MediaLibrary::is_valid_item_name("/simple/more"));
        assert_eq!(false, MediaLibrary::is_valid_item_name("simple/more"));
    }
}
