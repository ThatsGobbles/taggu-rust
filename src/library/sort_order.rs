use std::path::PathBuf;
use std::time::SystemTime;
use std::cmp::Ordering;

pub enum SortOrder {
    Name,
    ModTime,
}

impl SortOrder {
    pub fn path_sort_cmp<P: Into<PathBuf>>(&self, abs_item_path_a: P, abs_item_path_b: P) -> Ordering {
        let abs_item_path_a = abs_item_path_a.into();
        let abs_item_path_b = abs_item_path_b.into();

        match *self {
            SortOrder::Name => abs_item_path_a.file_name().cmp(&abs_item_path_b.file_name()),
            SortOrder::ModTime => SortOrder::get_mtime(abs_item_path_a).cmp(&SortOrder::get_mtime(abs_item_path_b)),
        }
    }

    fn get_mtime<P: Into<PathBuf>>(abs_path: P) -> Option<SystemTime> {
        abs_path.into().metadata().and_then(|m| m.modified()).ok()
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;
    use std::fs::{File, DirBuilder};

    use super::SortOrder;

    #[test]
    fn test_get_mtime() {
        // Create temp directory.
        let temp = TempDir::new("").unwrap();
        let tp = temp.path();

        // Create and test temp files and directories.
        let db = DirBuilder::new();

        let path = tp.join("file.txt");
        File::create(&path).unwrap();
        assert!(SortOrder::get_mtime(&path).is_some());

        let path = tp.join("dir");
        db.create(&path).unwrap();
        assert!(SortOrder::get_mtime(&path).is_some());

        let path = tp.join("NON_EXISTENT");
        assert!(SortOrder::get_mtime(&path).is_none());
    }
}
