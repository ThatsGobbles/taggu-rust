pub mod reader;
pub mod target;

use std::path::{Path, PathBuf};
use std::collections::{BTreeMap, HashMap};
use std::fs::DirEntry;

use library::sort_order::SortOrder;
use library::selection::Selection;
use error::*;
use generator::GenConverter;

pub type MetaBlock = BTreeMap<String, MetaValue>;
pub type MetaBlockSeq = Vec<MetaBlock>;
pub type MetaBlockMap = BTreeMap<String, MetaBlock>;

/// Mapping of item file paths to their complete metadata blocks.
pub type PathMetaListing = HashMap<PathBuf, MetaBlock>;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
pub enum MetaTarget {
    Contains,
    Siblings,
}

impl MetaTarget {
    pub fn get_target_meta_path<P: AsRef<Path>>(&self, item_path: P) -> Result<PathBuf> {
        let item_path: &Path = item_path.as_ref();

        ensure!(item_path.exists(), ErrorKind::DoesNotExist(item_path.to_path_buf()));

        let meta_path = match *self {
            MetaTarget::Contains => {
                ensure!(item_path.is_dir(), ErrorKind::NotADirectory(item_path.to_path_buf()));

                item_path.join("taggu_self.yml")
            },
            MetaTarget::Siblings => {
                match item_path.parent() {
                    Some(item_path_parent) => item_path_parent.join("taggu_item.yml"),
                    None => bail!(ErrorKind::CappedAtRoot),
                }
            }
        };

        ensure!(meta_path.exists(), ErrorKind::DoesNotExist(meta_path.to_path_buf()));
        ensure!(meta_path.is_file(), ErrorKind::NotAFile(meta_path.to_path_buf()));

        Ok(meta_path)
    }

    pub fn get_target_metadata<P: AsRef<Path>>(&self, item_path: P) -> Result<PathMetaListing> {
        Ok(hashmap![])
    }
}

/// A data structure-level representation of all possible metadata types and their formats.
/// This is intended to be independent of the text-level representation of the metadata.
#[derive(Debug)]
pub enum Metadata {
    Contains(MetaBlock),
    SiblingsSeq(MetaBlockSeq),
    SiblingsMap(MetaBlockMap),
}

impl Metadata {
    fn get_relevant_dir_entries<P: AsRef<Path>>(working_dir_path: P, selection: &Selection, opt_sort_order: Option<SortOrder>) -> Result<Vec<DirEntry>> {
        let working_dir_path = working_dir_path.as_ref();

        let mut dir_entries = selection.selected_entries_in_dir(working_dir_path)?;

        if let Some(sort_order) = opt_sort_order {
            dir_entries.sort_unstable_by(|a, b| sort_order.path_sort_cmp(a.path(), b.path()));
        }

        Ok(dir_entries)
    }

    fn get_relevant_paths<P: AsRef<Path>>(working_dir_path: P, selection: &Selection, opt_sort_order: Option<SortOrder>) -> Result<Vec<PathBuf>> {
        Ok(Metadata::get_relevant_dir_entries(working_dir_path, selection, opt_sort_order)?.iter().map(|e| e.path()).collect())
    }

    fn get_relevant_names<P: AsRef<Path>>(working_dir_path: P, selection: &Selection, opt_sort_order: Option<SortOrder>) -> Result<Vec<String>> {
        Ok(Metadata::get_relevant_paths(working_dir_path, selection, opt_sort_order)?
            .iter()
            .filter_map(|p| p.file_name())
            .map(|o_str| o_str.to_string_lossy().to_string())
            .collect())
    }

    pub fn source_item_names<P: AsRef<Path>>(
        &self,
        working_dir_path: P,
        selection: &Selection,
        sort_order: SortOrder,
        ) -> Result<Vec<String>>
    {
        match *self {
            Metadata::Contains(_) => Ok(vec![]),
            Metadata::SiblingsSeq(_) => Metadata::get_relevant_names(working_dir_path, selection, Some(sort_order)),
            Metadata::SiblingsMap(_) => Metadata::get_relevant_names(working_dir_path, selection, None),
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub enum MetaKey {
    Nil,
    Str(String),
}

impl MetaKey {
    pub fn iter_over<'a>(&'a self) -> impl Iterator<Item = &'a String> {
        let closure = move || {
            match *self {
                MetaKey::Nil => {},
                MetaKey::Str(ref s) => { yield s; },
            }
        };

        GenConverter::gen_to_iter(closure)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum MetaValue {
    Nil,
    Str(String),
    Seq(Vec<MetaValue>),
    Map(BTreeMap<MetaKey, MetaValue>),
}

impl MetaValue {
    pub fn iter_over<'a>(&'a self, mis: MappingIterScheme) -> impl Iterator<Item = &'a String> {
        let closure = move || {
            match *self {
                MetaValue::Nil => {},
                MetaValue::Str(ref s) => { yield s; },
                MetaValue::Seq(ref mvs) => {
                    for mv in mvs {
                        for i in Box::new(mv.iter_over(mis)) {
                            yield i;
                        }
                    }
                },
                MetaValue::Map(ref map) => {
                    for (mk, mv) in map {
                        match mis {
                            MappingIterScheme::Keys | MappingIterScheme::Both => {
                                // This outputs the value of the Nil key first, but only if a BTreeMap is used.
                                for s in Box::new(mk.iter_over()) {
                                    yield s;
                                }
                            },
                            MappingIterScheme::Vals => {},
                        };

                        match mis {
                            MappingIterScheme::Vals | MappingIterScheme::Both => {
                                for s in Box::new(mv.iter_over(mis)) {
                                    yield s;
                                }
                            },
                            MappingIterScheme::Keys => {},
                        };
                    }
                },
            }
        };

        GenConverter::gen_to_iter(closure)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub enum MappingIterScheme {
    Keys,
    Vals,
    Both,
}

#[cfg(test)]
mod tests {
    use super::{
        MetaValue,
        MappingIterScheme,
    };

    #[test]
    fn test_meta_value_flatten() {
        let str_sample_a = "Goldfish".to_string();
        let str_sample_b = "DIMMI".to_string();
        // let str_sample_c = "Pontifexx".to_string();
        let seq_sample = vec![MetaValue::Str(str_sample_a.clone()), MetaValue::Str(str_sample_b.clone())];

        let mis = MappingIterScheme::Both;

        let inputs_and_expected: Vec<(MetaValue, Vec<&String>)> = vec![
            (MetaValue::Nil, vec![]),
            (MetaValue::Str(str_sample_a.clone()), vec![&str_sample_a]),
            (MetaValue::Seq(seq_sample.clone()), vec![&str_sample_a, &str_sample_b]),
        ];

        for (input, expected) in inputs_and_expected {
            let produced: Vec<&String> = input.iter_over(mis).collect();
            assert_eq!(expected, produced);
        }
    }
}
