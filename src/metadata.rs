use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::fs::DirEntry;

use helpers::normalize;
use library::sort_order::SortOrder;
use library::selection::Selection;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub enum MetaKey {
    Null,
    String(String),
}

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum MetaValue {
    Null,
    String(String),
    Sequence(Vec<MetaValue>),
    Mapping(BTreeMap<MetaKey, MetaValue>),
}

/// Represents one or more item targets that a given set of metadata provides data for.
#[derive(Debug)]
pub enum MetaTarget {
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
            MetaTarget::Contains => {
                if abs_meta_path.is_dir() { Some(abs_meta_path) }
                else { None }
            },
        }
    }
}

pub type MetaBlock = BTreeMap<String, MetaValue>;
pub type MetaBlockSeq = Vec<MetaBlock>;
pub type MetaBlockMap = BTreeMap<String, MetaBlock>;

/// A data structure-level representation of all possible metadata types and their formats.
/// This is intended to be independent of the text-level representation of the metadata.
#[derive(Debug)]
pub enum Metadata {
    Contains(MetaBlock),
    SiblingsSeq(MetaBlockSeq),
    SiblingsMap(MetaBlockMap),
}

impl Metadata {
    fn get_relevant_dir_entries<P: AsRef<Path>>(working_dir_path: P, selection: &Selection, opt_sort_order: Option<SortOrder>) -> Vec<DirEntry> {
        let working_dir_path = working_dir_path.as_ref();

        let mut dir_entries = selection.selected_entries_in_dir(working_dir_path);

        if let Some(sort_order) = opt_sort_order {
            dir_entries.sort_unstable_by(|a, b| sort_order.path_sort_cmp(a.path(), b.path()));
        }

        dir_entries
    }

    fn get_relevant_paths<P: AsRef<Path>>(working_dir_path: P, selection: &Selection, opt_sort_order: Option<SortOrder>) -> Vec<PathBuf> {
        Metadata::get_relevant_dir_entries(working_dir_path, selection, opt_sort_order).iter().map(|e| e.path()).collect()
    }

    fn get_relevant_names<P: AsRef<Path>>(working_dir_path: P, selection: &Selection, opt_sort_order: Option<SortOrder>) -> Vec<String> {
        Metadata::get_relevant_paths(working_dir_path, selection, opt_sort_order)
            .iter()
            .filter_map(|p| p.file_name())
            .map(|o_str| o_str.to_string_lossy().to_string())
            .collect()
    }

    pub fn source_item_names<P: AsRef<Path>>(
        &self,
        working_dir_path: P,
        selection: &Selection,
        sort_order: SortOrder,
        ) -> Vec<String>
    {
        match *self {
            Metadata::Contains(_) => vec![],
            Metadata::SiblingsSeq(_) => Metadata::get_relevant_names(working_dir_path, selection, Some(sort_order)),
            Metadata::SiblingsMap(_) => Metadata::get_relevant_names(working_dir_path, selection, None),
        }
    }
}
