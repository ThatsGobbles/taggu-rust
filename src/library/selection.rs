use std::path::PathBuf;
use regex::Regex;
use std::ffi::OsStr;
use std::fs::DirEntry;

use super::super::path::normalize;

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

impl Selection {
    pub fn is_selected_path<P: Into<PathBuf>>(&self, abs_item_path: P) -> bool {
        let abs_item_path = normalize(&abs_item_path.into());

        if !abs_item_path.exists() {
            return false
        }

        match *self {
            Selection::Ext(ref e_ext) => abs_item_path.extension() == Some(&OsStr::new(e_ext)),
            Selection::Regex(ref r_exp) => {
                abs_item_path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .map_or(false, |f| r_exp.is_match(f))
            },
            Selection::IsFile => abs_item_path.is_file(),
            Selection::IsDir => abs_item_path.is_dir(),
            Selection::And(ref sel_a, ref sel_b) => sel_a.is_selected_path(&abs_item_path)
                && sel_b.is_selected_path(&abs_item_path),
            Selection::Or(ref sel_a, ref sel_b) => sel_a.is_selected_path(&abs_item_path)
                || sel_b.is_selected_path(&abs_item_path),
            Selection::Xor(ref sel_a, ref sel_b) => sel_a.is_selected_path(&abs_item_path)
                ^ sel_b.is_selected_path(&abs_item_path),
            Selection::Not(ref sel) => !sel.is_selected_path(&abs_item_path),
            Selection::True => true,
            Selection::False => false,
        }
    }

    pub fn selected_entries_in_dir<P: Into<PathBuf>>(&self, abs_dir_path: P) -> Vec<DirEntry> {
        let abs_dir_path = normalize(&abs_dir_path.into());

        let mut sel_entries: Vec<DirEntry> = vec![];

        abs_dir_path.read_dir().map(|dir_entries| {
            for dir_entry in dir_entries {
                if let Ok(dir_entry) = dir_entry {
                    if self.is_selected_path(dir_entry.path()) {
                        sel_entries.push(dir_entry);
                    }
                }
            }
        }).is_ok();

        sel_entries
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::fs::{DirBuilder, File};

    use tempdir::TempDir;
    use regex::Regex;

    use super::Selection;

    #[test]
    fn test_is_selected_path() {
        // Create temp directory.
        let temp = TempDir::new("test_is_selected_path").unwrap();
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
                let produced = selection.is_selected_path(&abs_path);
                // println!("{:?}, {:?}", abs_path, selection);
                assert_eq!(expected, produced);
            }
        }
    }
}
