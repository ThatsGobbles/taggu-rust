// This module provides an interface to "match up" media items with metadata blocks.

use std::path::{Path, PathBuf};
use std::collections::HashSet;

use yaml_rust::{Yaml};

use library::{MediaLibrary, MetaTarget};
use generator::gen_to_iter;
use metadata::{
    MetaBlock,
    yaml_as_self_metadata,
    yaml_as_item_map_metadata,
    yaml_as_item_seq_metadata,
};
use yaml::read_yaml_file;

// TODO: Refactor out reference to MediaLibrary!
pub fn plex<'a, P: Into<PathBuf> + 'a>(working_dir_path: P, meta_target: &'a MetaTarget, media_lib: &'a MediaLibrary) -> impl Iterator<Item = (PathBuf, MetaBlock)> + 'a {
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
                            media_lib
                            .selected_entries_in_dir(&working_dir_path)
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
                            let sorted_selected_items = media_lib.sort_entries(media_lib.selected_entries_in_dir(&working_dir_path));

                            if isd.len() != sorted_selected_items.len() {
                                warn!("Lengths do not match!");
                            }

                            for (item_dir_entry, mb) in sorted_selected_items.into_iter().zip(isd) {
                                // No need to check if item file path exists,
                                // since it was returned by directory iteration.
                                let item_file_path = item_dir_entry.path().to_path_buf();
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

    use super::plex;

    #[test]
    fn test_plex() {

    }
}
