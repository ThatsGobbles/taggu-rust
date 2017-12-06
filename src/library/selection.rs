use std::path::PathBuf;
use regex::Regex;
use std::ffi::OsString;
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
            Selection::Ext(ref e_ext) => {
                if let Some(p_ext) = abs_item_path.extension() {
                    OsString::from(e_ext) == p_ext
                } else {
                    false
                }
            },
            Selection::Regex(ref r_exp) => {
                let maybe_fn = abs_item_path.file_name().and_then(|x| x.to_str());

                if let Some(fn_str) = maybe_fn {
                    r_exp.is_match(fn_str)
                } else {
                    false
                }
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
        });

        sel_entries
    }
}
