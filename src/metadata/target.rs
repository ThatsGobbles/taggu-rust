use std::path::{Path, PathBuf};

use helpers::normalize;

/// Represents one or more item targets that a given set of metadata provides data for.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum MetaTarget {
    // NOTE: Order of enum values matter, this will be the order of resolution for metadata sources.
    Contains,
    Siblings,
}

impl MetaTarget {
    pub fn target_dir_path<P: AsRef<Path>>(&self, abs_meta_path: P) -> Option<PathBuf> {
        let abs_meta_path = normalize(&abs_meta_path.as_ref());

        if !abs_meta_path.exists() {
            return None
        }

        match *self {
            MetaTarget::Siblings => abs_meta_path.parent().map(|f| f.to_path_buf()),
            MetaTarget::Contains => match abs_meta_path.is_dir() {
                true => Some(abs_meta_path),
                false => None,
            },
        }
    }
}
