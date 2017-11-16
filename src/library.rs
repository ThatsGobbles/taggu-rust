use std::path::Path;
use std::path::PathBuf;
use std::path::Component;
// use std::fs;
// use std::cmp;
// use std::ops::Fn;
use std::fmt;
// use regex::Regex;

use error::MediaLibraryError;
use ::path::normalize;

// const EXT: &str = "flac";

// fn default_media_item_filter(abs_path: &Path) -> bool {
//     abs_path.is_dir() || (abs_path.is_file() && abs_path.extension() == Some(ffi::OsStr::new(EXT)))
// }

// enum MediaFileCriteria<'a> {
//     Ext(&'a str),
//     Prefix(&'a str),
//     Suffix(&'a str),
//     Regex(&'a Regex),
//     IsDir,
//     IsFile,
//     And(&'a MediaFileCriteria<'a>, &'a MediaFileCriteria<'a>),
//     Or(&'a MediaFileCriteria<'a>, &'a MediaFileCriteria<'a>),
//     Not(&'a MediaFileCriteria<'a>),
// }

struct MediaLibrary {
    root_dir: PathBuf,
    item_meta_fn: String,
    self_meta_fn: String,
    // media_item_filter: fn(&Path) -> bool,
    // media_item_sort_key: FS,
}

impl MediaLibrary {
    pub fn new<P: AsRef<Path>, S: AsRef<str>>(root_dir: P,
            item_meta_fn: S,
            self_meta_fn: S)
            -> Result<MediaLibrary, MediaLibraryError> {
        let root_dir = try!(root_dir.as_ref().to_path_buf().canonicalize());

        if !root_dir.is_dir() {
            return Err(MediaLibraryError::NotADir(root_dir))
        }

        Ok(MediaLibrary {
            root_dir: root_dir,
            item_meta_fn: item_meta_fn.as_ref().to_string(),
            self_meta_fn: self_meta_fn.as_ref().to_string(),
        })
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
}

impl fmt::Debug for MediaLibrary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MediaLibrary {{ root_dir: {:?}, item_meta_fn: {:?}, self_meta_fn: {:?} }}", self.root_dir, self.item_meta_fn, self.self_meta_fn)
    }
}

pub fn example() {
    let m = MediaLibrary::new("/home/lemoine/Music", "taggu_item.yml", "taggu_self.yml");

    let m = match m {
        Ok(x) => x,
        Err(err) => { println!("Error: {}", err); return (); },
    };

    println!("{:?}", m);

    fn success(norm_tup: (PathBuf, PathBuf)) -> Result<(PathBuf, PathBuf), MediaLibraryError> {
        println!("{:?}", norm_tup);
        Ok(norm_tup)
    }

    fn error(err: MediaLibraryError) -> Result<(PathBuf, PathBuf), MediaLibraryError> {
        println!("{}", err);
        Err(err)
    }

    let _ = m.co_norm("/Psystyle Nation").and_then(success).or_else(error);
    let _ = m.co_norm("Psystyle Nation").and_then(success).or_else(error);
    let _ = m.co_norm("../Psystyle Nation").and_then(success).or_else(error);
    let _ = m.co_norm("BASS AVENGERS/../Saturdays/TRACK01//").and_then(success).or_else(error);
    let _ = m.co_norm(".").and_then(success).or_else(error);
    let _ = m.co_norm("..").and_then(success).or_else(error);
}

#[cfg(test)]
mod tests {
    use super::MediaLibrary;
    use std::path::Path;
    use std::path::PathBuf;
    use tempdir::TempDir;

    #[test]
    fn test_co_norm_valid() {
        let temp = TempDir::new("media_lib").unwrap();
        let root_dir = temp.path();
        let media_lib = MediaLibrary::new(root_dir, "item.yml", "self.yml").unwrap();

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
    }
}
