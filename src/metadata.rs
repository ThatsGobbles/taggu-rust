use std::collections::BTreeMap;
use std::path::PathBuf;

use path::normalize;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub enum MetaKey {
    Null,
    String(String),
}

#[derive(PartialEq, Debug, Clone)]
pub enum MetaValue {
    Null,
    String(String),
    Sequence(Vec<MetaValue>),
    Mapping(BTreeMap<MetaKey, MetaValue>),
}

/// Represents one or more item targets that a given set of metadata provides data for.
pub enum MetaTarget {
    Contains,
    Siblings,
}

impl MetaTarget {
    pub fn target_dir_path<P: AsRef<PathBuf>>(&self, abs_item_path: P) -> Option<PathBuf> {
        let abs_item_path = normalize(&abs_item_path.as_ref());

        if !abs_item_path.exists() {
            return None
        }

        match *self {
            MetaTarget::Siblings => abs_item_path.parent().map(|f| f.to_path_buf()),
            MetaTarget::Contains => {
                if abs_item_path.is_dir() { Some(abs_item_path) }
                else { None }
            },
        }
    }
}

pub type MetaBlock = BTreeMap<String, MetaValue>;
pub type MetaBlockSeq = Vec<MetaBlock>;
pub type MetaBlockMap = BTreeMap<String, MetaBlock>;

/// Defines all possible formats for 'self' metadata.
pub enum SelfMetaFormat {
    Def(MetaBlock),
}

/// Defines all possible formats for 'item' metadata.
pub enum ItemMetaFormat {
    Seq(MetaBlockSeq),
    Map(MetaBlockMap),
}

/// A data structure-level representation of all possible metadata types and their formats.
/// This is intended to be independent of the text-level representation of the metadata.
pub enum Metadata {
    SelfMetadata(SelfMetaFormat),
    ItemMetadata(ItemMetaFormat),
}
