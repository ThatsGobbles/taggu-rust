use std::path::Path;
use std::time::SystemTime;
use std::cmp::Ordering;

#[derive(Copy, Clone)]
pub enum SortOrder {
    Name,
    ModTime,
}

impl SortOrder {
    pub fn path_sort_cmp<P: AsRef<Path>>(&self, abs_item_path_a: P, abs_item_path_b: P) -> Ordering {
        let abs_item_path_a = abs_item_path_a.as_ref();
        let abs_item_path_b = abs_item_path_b.as_ref();

        match *self {
            SortOrder::Name => abs_item_path_a.file_name().cmp(&abs_item_path_b.file_name()),
            SortOrder::ModTime => SortOrder::get_mtime(abs_item_path_a).cmp(&SortOrder::get_mtime(abs_item_path_b)),
        }
    }

    fn get_mtime<P: AsRef<Path>>(abs_path: P) -> Option<SystemTime> {
        abs_path.as_ref().metadata().and_then(|m| m.modified()).ok()
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;
    use std::fs::{File, DirBuilder};
    use std::thread::sleep;
    use std::time::Duration;

    use super::SortOrder;

    #[test]
    fn test_path_sort_cmp() {
        // Create temp directory.
        let temp = TempDir::new("").unwrap();
        let tp = temp.path();

        // Create and test temp files and directories.
        let db = DirBuilder::new();

        let fps = vec![
            tp.join("file_b"),
            tp.join("file_a"),
            tp.join("file_d"),
            tp.join("file_e"),
            tp.join("file_c"),
        ];

        for fp in &fps {
            // LEARN: Because we're iterating over a ref to a vector, the iter vars are also refs.
            File::create(fp).expect(&format!(r#"Unable to create file "{:?}""#, fp));
            sleep(Duration::from_millis(10));
        }

        // Test sorting by mod time.
        let sort_order = SortOrder::ModTime;

        for (o_i, o_val) in fps.iter().enumerate() {
            for (i_i, i_val) in fps.iter().enumerate() {
                assert_eq!(o_i.cmp(&i_i), sort_order.path_sort_cmp(o_val, i_val));
            }
        }

        // Test sorting by name.
        let sort_order = SortOrder::Name;

        for o_val in fps.iter() {
            for i_val in fps.iter() {
                assert_eq!(o_val.file_name().cmp(&i_val.file_name()), sort_order.path_sort_cmp(o_val, i_val));
            }
        }
    }

    #[test]
    fn test_get_mtime() {
        // Create temp directory.
        let temp = TempDir::new("").unwrap();
        let tp = temp.path();

        // Create and test temp files and directories.
        let db = DirBuilder::new();

        let f_path = tp.join("file.txt");
        File::create(&f_path).unwrap();
        assert!(SortOrder::get_mtime(&f_path).is_some());

        let d_path = tp.join("dir");
        db.create(&d_path).unwrap();
        assert!(SortOrder::get_mtime(&d_path).is_some());

        let x_path = tp.join("NON_EXISTENT");
        assert!(SortOrder::get_mtime(&x_path).is_none());

        // Test ordering.
        let path_a = tp.join("time_a.txt");
        File::create(&path_a).unwrap();

        sleep(Duration::from_millis(10));

        let path_b = tp.join("time_b.txt");
        File::create(&path_b).unwrap();

        let time_a = SortOrder::get_mtime(&path_a);
        let time_b = SortOrder::get_mtime(&path_b);

        assert!(time_a < time_b);
        assert!(time_b > time_a);
    }
}
