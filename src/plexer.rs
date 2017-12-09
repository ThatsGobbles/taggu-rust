// This module provides an interface to "match up" media items with metadata blocks.

use std::path::{Path, PathBuf};
use std::collections::HashSet;

use yaml_rust::{Yaml};

use library::{MediaLibrary};
use library::selection::Selection;
use library::sort_order::SortOrder;
use generator::gen_to_iter;
use metadata::{
    MetaBlock,
    MetaTarget,
    yaml_as_self_metadata,
    yaml_as_item_map_metadata,
    yaml_as_item_seq_metadata,
};
use yaml::read_yaml_file;

pub fn plex<'a, P: Into<PathBuf> + 'a>(
        working_dir_path: P,
        meta_target: &'a MetaTarget,
        selection: &'a Selection,
        sort_order: &'a SortOrder
    ) -> impl Iterator<Item = (PathBuf, MetaBlock)> + 'a
{
    // Assume meta file path exists, and is a proper subpath.
    // We also assume that the meta path filename matches the meta target type.
    let closure = move || {
        // Get meta file name and construct meta file path.
        let working_dir_path = working_dir_path.into();
        let meta_file_name = meta_target.meta_file_name();
        let meta_file_path = working_dir_path.join(meta_file_name);

        // Try to read and parse YAML meta file.
        let temp = read_yaml_file(&meta_file_path);
        if let Ok(yaml) = temp {
            match *meta_target {
                MetaTarget::Alongside(_) => {
                    // Metadata in this style has two possible formats.
                    // Check for both of them.
                    let temp = yaml_as_item_map_metadata(&yaml);
                    if let Some(imd) = temp {
                        // Metadata is a mapping of item file names to meta blocks.

                        // Create a mutable set of item names found in this directory.
                        let mut selected_items: HashSet<String> = {
                            selection
                            .selected_entries_in_dir(&working_dir_path)
                            .iter()
                            .filter_map(|e| {
                                e.path()
                                .file_name()
                                .and_then(|f| f.to_str())
                                .map(|f| f.to_string())
                            })
                            .collect()
                        };

                        for (item_file_name, mb) in imd {
                            // Check if the file name is valid.
                            if !MediaLibrary::is_valid_item_name(item_file_name.clone()) {
                                error!(r#"Item name "{}" is invalid"#, item_file_name);
                                continue;
                            }

                            // Check if the item name from metadata is found in the set.
                            if !selected_items.remove(&item_file_name) {
                                error!(r#"Item name "{}" was not found in the directory"#, item_file_name);
                                continue;
                            }

                            // TODO: Check if item path exists!
                            let item_file_path = working_dir_path.join(item_file_name);
                            yield (item_file_path, mb)
                        }
                    } else {
                        let temp = yaml_as_item_seq_metadata(&yaml);
                        if let Some(isd) = temp {
                            // Metadata is a sequence of meta blocks.
                            // Each should correspond one-to-one with a valid item in the working dir.
                            let mut selected_item_paths: Vec<PathBuf> = {
                                selection
                                .selected_entries_in_dir(&working_dir_path)
                                .iter()
                                .map(|e| e.path())
                                .collect()
                            };
                            selected_item_paths.sort_unstable_by(|a, b| sort_order.path_sort_cmp(a, b));

                            let sorted_selected_item_paths = selected_item_paths;

                            if isd.len() != sorted_selected_item_paths.len() {
                                warn!("Lengths do not match!");
                            }

                            for (item_file_path, mb) in sorted_selected_item_paths.into_iter().zip(isd) {
                                // No need to check if item file path exists,
                                // since it was returned by directory iteration.
                                yield (item_file_path, mb)
                            }
                        }
                    }
                },
                MetaTarget::Container(_) => {
                    // This will yield only a single item path: the working directory path itself.
                    let temp = yaml_as_self_metadata(&yaml);
                    if let Some(mb) = temp {
                        let temp = working_dir_path.clone();
                        yield (temp, mb)
                    }
                },
            }
        }
    };

    gen_to_iter(closure)
}

// =================================================================================================
// TESTS
// =================================================================================================


#[cfg(test)]
mod tests {
    use std::path::{PathBuf};
    use std::fs::{File, DirBuilder};
    use std::io::Write;
    use std::thread::sleep;
    use std::time::Duration;

    use tempdir::TempDir;

    use super::plex;
    use library::selection::Selection;
    use library::sort_order::SortOrder;
    use metadata::{MetaTarget, MetaValue};

    #[test]
    fn test_plex() {
        // Create temp directory.
        let temp = TempDir::new("test_plex").unwrap();
        let root_dir = temp.path();

        let self_meta_target = MetaTarget::Container(String::from("taggu_self.yml"));
        let item_meta_target = MetaTarget::Alongside(String::from("taggu_item.yml"));

        let selection: Selection = Selection::Ext(String::from("item"));

        // Test self-metadata.
        {
            let temp = TempDir::new_in(&root_dir, "self_meta").unwrap();

            let working_dir_path = temp.path();

            let self_meta_path = working_dir_path.join(self_meta_target.meta_file_name());
            let mut self_meta_file =
                File::create(&self_meta_path)
                    .expect("Unable to create metadata file");

            writeln!(self_meta_file, "artist: lapix\ntitle: Beyond the Limits\ndate: 2014-12-30")
                .expect("Unable to write metadata file");

            let results: Vec<_> =
                plex(&working_dir_path, &self_meta_target, &selection, &SortOrder::Name)
                    .collect();

            assert!(results.len() == 1);

            let (ref t_path, ref t_mb) = results[0];

            assert!(*t_path == working_dir_path);
            assert!(*t_mb == btreemap![
                String::from("artist") => MetaValue::String(String::from("lapix")),
                String::from("date") => MetaValue::String(String::from("2014-12-30")),
                String::from("title") => MetaValue::String(String::from("Beyond the Limits")),
            ]);
        };
        // Test sequenced item-metadata.
        {
            let temp = TempDir::new_in(&root_dir, "item_meta").unwrap();

            let working_dir_path = temp.path();

            let item_meta_path = working_dir_path.join(item_meta_target.meta_file_name());
            let mut item_meta_file =
                File::create(&item_meta_path)
                    .expect("Unable to create metadata file");

            writeln!(item_meta_file, "- title: Foolish Hero\n- title: Beyond the Limits\n  feat.artist: Luschel")
                .expect("Unable to write metadata file");

            let item_file_names = vec!["b.item", "a.item"];

            for item_file_name in &item_file_names {
                let item_file_path = working_dir_path.join(item_file_name);
                File::create(&item_file_path).expect("Unable to create item file");
            }

            let results: Vec<_> =
                plex(&working_dir_path, &item_meta_target, &selection, &SortOrder::Name)
                    .collect();

            assert!(results.len() == 2);

            let (ref t_path, ref t_mb) = results[0];

            assert!(*t_path == working_dir_path.join("a.item"));
            assert!(*t_mb == btreemap![
                String::from("title") => MetaValue::String(String::from("Foolish Hero")),
            ]);

            let (ref t_path, ref t_mb) = results[1];

            assert!(*t_path == working_dir_path.join("b.item"));
            assert!(*t_mb == btreemap![
                String::from("feat.artist") => MetaValue::String(String::from("Luschel")),
                String::from("title") => MetaValue::String(String::from("Beyond the Limits")),
            ]);
        };
        // Test mapped item-metadata.
        {
            let temp = TempDir::new_in(&root_dir, "item_meta").unwrap();

            let working_dir_path = temp.path();

            let item_meta_path = working_dir_path.join(item_meta_target.meta_file_name());
            let mut item_meta_file =
                File::create(&item_meta_path)
                    .expect("Unable to create metadata file");

            writeln!(item_meta_file, "a.item:\n  title: Foolish Hero\nb.item:\n  title: Beyond the Limits\n  feat.artist: Luschel")
                .expect("Unable to write metadata file");

            let item_file_names = vec!["b.item", "a.item"];

            for item_file_name in &item_file_names {
                let item_file_path = working_dir_path.join(item_file_name);
                File::create(&item_file_path).expect("Unable to create item file");
            }

            let results: Vec<_> =
                plex(&working_dir_path, &item_meta_target, &selection, &SortOrder::Name)
                    .collect();

            assert!(results.len() == 2);

            let (ref t_path, ref t_mb) = results[0];

            assert!(*t_path == working_dir_path.join("a.item"));
            assert!(*t_mb == btreemap![
                String::from("title") => MetaValue::String(String::from("Foolish Hero")),
            ]);

            let (ref t_path, ref t_mb) = results[1];

            assert!(*t_path == working_dir_path.join("b.item"));
            assert!(*t_mb == btreemap![
                String::from("feat.artist") => MetaValue::String(String::from("Luschel")),
                String::from("title") => MetaValue::String(String::from("Beyond the Limits")),
            ]);
        };
    }
}
