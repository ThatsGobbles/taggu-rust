use std::path::Path;
use std::path::PathBuf;
use std::path::Component;
use std::ffi::{OsStr, OsString};
use std::fs::DirEntry;
use std::marker::PhantomData;
use regex::Regex;

use error::MediaLibraryError;
use ::path::normalize;

enum MediaSelection {
    Ext(OsString),
    Regex(Regex),
    IsFile,
    IsDir,
    IsSymlink,
    And(Box<MediaSelection>, Box<MediaSelection>),
    Or(Box<MediaSelection>, Box<MediaSelection>),
    Xor(Box<MediaSelection>, Box<MediaSelection>),
    Not(Box<MediaSelection>),
}

struct MediaLibrary<FF, FS, O, PP>
where
    O: Ord,
    PP: AsRef<Path>,
    FF: Fn(PP) -> bool,
    FS: Fn(PP) -> O,
{
    root_dir: PathBuf,
    item_meta_fn: String,
    self_meta_fn: String,
    media_item_filter: FF,
    media_item_sort_key: FS,
    _o: PhantomData<O>,
    _p: PhantomData<PP>,
}

impl<FF, FS, O, PP> MediaLibrary<FF, FS, O, PP>
where
    FF: Fn(PP) -> bool,
    FS: Fn(PP) -> O,
    O: Ord,
    PP: AsRef<Path>,
{
    pub fn new<P: AsRef<Path>, S: AsRef<str>>(root_dir: P,
            item_meta_fn: S,
            self_meta_fn: S,
            media_item_filter: FF,
            media_item_sort_key: FS,
            ) -> Result<MediaLibrary<FF, FS, O, PP>, MediaLibraryError> {
        let root_dir = try!(root_dir.as_ref().to_path_buf().canonicalize());

        if !root_dir.is_dir() {
            return Err(MediaLibraryError::NotADir(root_dir))
        }

        Ok(MediaLibrary {
            root_dir: root_dir,
            item_meta_fn: item_meta_fn.as_ref().to_string(),
            self_meta_fn: self_meta_fn.as_ref().to_string(),
            media_item_filter: media_item_filter,
            media_item_sort_key: media_item_sort_key,
            _o: PhantomData,
            _p: PhantomData,
        })
    }

    pub fn default_media_item_filter<P: AsRef<Path>>(abs_item_path: P) -> bool {
        let abs_item_path = abs_item_path.as_ref();
        abs_item_path.is_dir()
            || (abs_item_path.is_file()
                    && abs_item_path.extension() == Some(OsStr::new("flac")))
    }

    pub fn default_media_item_sort_key<P: AsRef<Path>>(abs_item_path: P) -> PathBuf {
        abs_item_path.as_ref().to_path_buf()
    }

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

    pub fn co_norm<P: AsRef<Path>>(&self, rel_sub_path: P) -> Result<(PathBuf, PathBuf), MediaLibraryError> {
        let rel_sub_path = rel_sub_path.as_ref().to_path_buf();

        // If relative path is not actually relative, error.
        if rel_sub_path.has_root() || !rel_sub_path.is_relative() {
            return Err(MediaLibraryError::NonRelPath(rel_sub_path))
        }

        // Join to root directory and normalize to get absolute path.
        let abs_sub_path = normalize(&self.root_dir.join(&rel_sub_path));

        // Check that the new absolute path is actually a descendant of root directory.
        // If so, strip off the prefix and have that be the new relative sub path.
        let rel_sub_path = match abs_sub_path.strip_prefix(&self.root_dir) {
            Ok(r) => normalize(r),
            Err(_) => { return Err(MediaLibraryError::EscapedSubPath(abs_sub_path.clone(), self.root_dir.clone())) }
        };

        Ok((rel_sub_path, abs_sub_path))
    }

    pub fn get_contains_dir<P: AsRef<Path>>(&self, rel_item_path: P) -> Option<PathBuf> {
        if let Ok((rel, abs)) = self.co_norm(rel_item_path) {
            if abs.is_dir() {
                return Some(rel)
            }
        }

        None
    }

    pub fn get_siblings_dir<P: AsRef<Path>>(&self, rel_item_path: P) -> Option<PathBuf> {
        if let Ok((rel, abs)) = self.co_norm(rel_item_path) {
            // TODO: Remove .unwrap().
            let n_parent = normalize(rel.parent().unwrap());

            if n_parent != rel {
                return Some(n_parent)
            }
        }

        None
    }

    pub fn all_entries_in_dir<P: AsRef<Path>>(&self, rel_sub_dir_path: P) -> Vec<DirEntry> {
        let mut found_entries: Vec<DirEntry> = vec![];

        // Co-normalize and use new absolute path.
        if let Ok((_, abs_sub_dir_path)) = self.co_norm(rel_sub_dir_path) {
            if let Ok(dir_entries) = abs_sub_dir_path.read_dir() {
                for dir_entry in dir_entries {
                    if let Ok(dir_entry) = dir_entry {
                        found_entries.push(dir_entry);
                    }
                }
            }
        }

        found_entries
    }

    pub fn filtered_entries_in_dir<P: AsRef<Path>>(&self, rel_sub_dir_path: P) -> Vec<DirEntry> {
        let mut found_entries: Vec<DirEntry> = vec![];
        let pred = |e| (self.media_item_filter)(e);

        // LEARN: This causes a move from the original vector, which is fine in this case.
        // TODO: Make into iterator and use .collect().
        for dir_entry in self.all_entries_in_dir(rel_sub_dir_path) {
            if pred(&dir_entry.path()) {
                found_entries.push(dir_entry);
            }
        }

        found_entries
    }

    pub fn entries_to_abs_fps(dir_entries: &[DirEntry]) -> Vec<PathBuf> {
        dir_entries.iter().map(|x| { x.path() }).collect()
    }

    pub fn fuzzy_name_lookup<P: AsRef<Path>, S: AsRef<str>>(&self, rel_sub_dir_path: P, prefix: S) -> Option<PathBuf> {
        let res = self.co_norm(rel_sub_dir_path).ok();

        if let Some((rel, abs)) = res {
            if abs.is_dir() {

            }
        }

        None
    }

    // def fuzzy_name_lookup(cls, *, rel_sub_dir_path: pl.Path, prefix_item_name: str) -> str:
    //     rel_sub_dir_path, abs_sub_dir_path = cls.co_norm(rel_sub_path=rel_sub_dir_path)

    //     pattern = f'{prefix_item_name}*'
    //     results = tuple(abs_sub_dir_path.glob(pattern))

    //     if len(results) != 1:
    //         msg = (f'Incorrect number of matches for fuzzy lookup of "{prefix_item_name}" '
    //                f'in directory "{rel_sub_dir_path}"; '
    //                f'expected: 1, found: {len(results)}')
    //         logger.error(msg)
    //         raise tex.NonUniqueFuzzyFileLookup(msg)

    //     abs_found_path = results[0]
    //     return abs_found_path.name

    // pub fn meta_files_from_item<P: AsRef<Path>>(&self, rel_item_path: P) -> Vec<P> {
    //     vec![]
    // }
}

pub fn example() {
    let media_lib = MediaLibrary::new("/home/lemoine/Music",
            "taggu_item.yml",
            "taggu_self.yml",
            MediaLibrary::default_media_item_filter,
            MediaLibrary::default_media_item_sort_key,
    ).unwrap();

    // println!("UNFILTERED");
    // let mut a_entries = MediaLibrary::entries_to_abs_fps(&media_lib.all_entries_in_dir("BASS AVENGERS"));
    // a_entries.sort_by_key(|e| MediaLibrary::default_media_item_sort_key(e));
    // a_entries.sort_by_key(|e| (media_lib.media_item_sort_key)(e));
    // for dir_entry in a_entries {
    //     println!("{:?}", dir_entry);
    // }

    println!("FILTERED");
    let mut f_entries = MediaLibrary::entries_to_abs_fps(&media_lib.filtered_entries_in_dir("BASS AVENGERS"));
    // f_entries.sort_by_key(|e| MediaLibrary::default_media_item_sort_key(e));
    f_entries.sort_by_key(|&e| (media_lib.media_item_sort_key)(&e));
    for dir_entry in f_entries {
        println!("{:?}", dir_entry);
    }
}

#[cfg(test)]
mod tests {
    use super::MediaLibrary;
    use std::path::PathBuf;
    use tempdir::TempDir;

    #[test]
    fn test_co_norm_valid() {
        let temp = TempDir::new("media_lib").unwrap();
        let root_dir = temp.path();
        let media_lib = MediaLibrary::new(
            root_dir,
            "item.yml",
            "self.yml",
            MediaLibrary::default_media_item_filter,
        ).unwrap();

        let expected = (PathBuf::from("subdir"), root_dir.join("subdir").to_path_buf());
        let produced = media_lib.co_norm("subdir").unwrap();
        assert_eq!(expected, produced);

        let expected = (PathBuf::from("."), root_dir.to_path_buf());
        let produced = media_lib.co_norm(".").unwrap();
        assert_eq!(expected, produced);

        let expected = (PathBuf::from("."), root_dir.to_path_buf());
        let produced = media_lib.co_norm("").unwrap();
        assert_eq!(expected, produced);

        let expected = (
            PathBuf::from("subdir/subdir"),
            root_dir.join("subdir").join("subdir").to_path_buf()
        );
        let produced = media_lib.co_norm("subdir/subdir/").unwrap();
        assert_eq!(expected, produced);

        let expected = (
            PathBuf::from("subdir/subdir"),
            root_dir.join("subdir").join("subdir").to_path_buf()
        );
        let produced = media_lib.co_norm("subdir/extra/../subdir/").unwrap();
        assert_eq!(expected, produced);

        let expected = (
            PathBuf::from("subdir"),
            root_dir.join("subdir").to_path_buf()
        );
        let produced = media_lib.co_norm("subdir/./").unwrap();
        assert_eq!(expected, produced);
    }

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
