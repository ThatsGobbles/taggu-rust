use std::path::Path;
use std::path::PathBuf;
use std::path::Component;
use std::ffi::OsString;
use std::fs::DirEntry;
use std::ops::Generator;

use regex::Regex;

use error::MediaLibraryError;
use path::normalize;

use generator::gen_to_iter;

enum MediaSelection {
    Ext(String),
    Regex(Regex),
    IsFile,
    IsDir,
    And(Box<MediaSelection>, Box<MediaSelection>),
    Or(Box<MediaSelection>, Box<MediaSelection>),
    Xor(Box<MediaSelection>, Box<MediaSelection>),
    Not(Box<MediaSelection>),
}

fn is_media_path<P: AsRef<Path>>(abs_item_path: P, sel: &MediaSelection) -> bool {
    // Assume that the path is already normalized.
    let abs_item_path = abs_item_path.as_ref();

    // TODO: Test if the path exists? If so, return false.
    if !abs_item_path.exists() {
        return false
    }

    match sel {
        &MediaSelection::Ext(ref e_ext) => {
            if let Some(p_ext) = abs_item_path.extension() {
                OsString::from(e_ext) == p_ext
            } else {
                false
            }
        },
        &MediaSelection::Regex(ref r_exp) => {
            let maybe_fn = abs_item_path.file_name().map(|x| x.to_str());

            if let Some(Some(fn_str)) = maybe_fn {
                r_exp.is_match(fn_str)
            } else {
                false
            }
        },
        &MediaSelection::IsFile => abs_item_path.is_file(),
        &MediaSelection::IsDir => abs_item_path.is_dir(),
        &MediaSelection::And(ref sel_a, ref sel_b) => is_media_path(abs_item_path, &sel_a) && is_media_path(abs_item_path, &sel_b),
        &MediaSelection::Or(ref sel_a, ref sel_b) => is_media_path(abs_item_path, &sel_a) || is_media_path(abs_item_path, &sel_b),
        &MediaSelection::Xor(ref sel_a, ref sel_b) => is_media_path(abs_item_path, &sel_a) ^ is_media_path(abs_item_path, &sel_b),
        &MediaSelection::Not(ref sel) => !is_media_path(abs_item_path, &sel),
    }
}

struct MediaLibrary {
    root_dir: PathBuf,
    item_meta_fn: String,
    self_meta_fn: String,
    media_selection: MediaSelection,
}

impl MediaLibrary {
    pub fn new<P: AsRef<Path>, S: AsRef<str>>(root_dir: P,
            item_meta_fn: S,
            self_meta_fn: S,
            media_selection: MediaSelection,
            ) -> Result<MediaLibrary, MediaLibraryError> {
        let root_dir = try!(root_dir.as_ref().to_path_buf().canonicalize());

        if !root_dir.is_dir() {
            return Err(MediaLibraryError::NotADir(root_dir))
        }

        Ok(MediaLibrary {
            root_dir: root_dir,
            item_meta_fn: item_meta_fn.as_ref().to_string(),
            self_meta_fn: self_meta_fn.as_ref().to_string(),
            media_selection,
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

    fn is_valid_media_item<P: AsRef<Path>>(&self, abs_item_path: P) -> bool {
        // Assume path is absolute and normalized.
        let sel = &self.media_selection;
        is_media_path(abs_item_path, sel)
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

    pub fn all_entries_in_dir<'a, P: AsRef<Path> + 'a>(&'a self, rel_sub_dir_path: P) -> impl Iterator<Item = DirEntry> + 'a {
        // Co-normalize and use new absolute path.
        let closure = move || match self.co_norm(rel_sub_dir_path) {
            Ok((_, abs_sub_dir_path)) => {
                let iter = abs_sub_dir_path.read_dir();
                if let Ok(dir_entries) = iter {
                    for dir_entry in dir_entries {
                        if let Ok(dir_entry) = dir_entry {
                            yield dir_entry;
                        }
                    }
                };
            },
            _ => {
                let d: Vec<DirEntry> = vec![];

                for x in d {
                    yield x;
                };
            }
        };

        gen_to_iter(closure)
    }

    pub fn filtered_entries_in_dir<'a, P: AsRef<Path> + 'a>(&'a self, rel_sub_dir_path: P) -> impl Iterator<Item = DirEntry> + 'a {
        self.all_entries_in_dir(rel_sub_dir_path).filter(move |x| self.is_valid_media_item(x.path()))
    }
}

pub fn example() {
    let selection = MediaSelection::Or(
        Box::new(MediaSelection::IsDir),
        Box::new(MediaSelection::And(
            Box::new(MediaSelection::IsFile),
            Box::new(MediaSelection::Ext("flac".to_string())),
        )),
    );

    let media_lib = MediaLibrary::new("/home/lemoine/Music",
            "taggu_item.yml",
            "taggu_self.yml",
            // MediaSelection::IsFile,
            selection,
    ).unwrap();

    println!("UNFILTERED");
    let a_entries: Vec<DirEntry> = media_lib.all_entries_in_dir("BASS AVENGERS").collect();
    for dir_entry in a_entries {
        println!("{:?}", dir_entry);
    }

    println!("FILTERED");
    let f_entries: Vec<DirEntry> = media_lib.filtered_entries_in_dir("BASS AVENGERS").collect();
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
