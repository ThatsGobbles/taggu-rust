pub mod reader;
pub mod target;

use std::path::{Path, PathBuf};
use std::collections::BTreeMap;
use std::fs::DirEntry;

use library::sort_order::SortOrder;
use library::selection::Selection;
use error::*;

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
    pub fn flatten<'a>(&'a self) -> Vec<&'a String> {
        match *self {
            MetaKey::Nil => vec![],
            MetaKey::Str(ref s) => vec![s],
        }
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
    pub fn flatten<'a>(&'a self, mis: MappingIterScheme) -> Vec<&'a String> {
        match *self {
            MetaValue::Nil => vec![],
            MetaValue::Str(ref s) => vec![s],
            MetaValue::Seq(ref mvs) => mvs.iter().flat_map(|mv| mv.flatten(mis)).collect(),
            MetaValue::Map(ref map) => {
                map.iter().flat_map(|(k, v)| {
                    // This yields nothing for null keys.
                    // Takes advantage of the fact that due to our definition of the enum, null values are first in the btree map.
                    let mut res = vec![];
                    match mis {
                        MappingIterScheme::Keys | MappingIterScheme::Both => { res.extend(k.flatten()); },
                        MappingIterScheme::Vals => {},
                    };

                    match mis {
                        MappingIterScheme::Vals | MappingIterScheme::Both => { res.extend(v.flatten(mis)); },
                        MappingIterScheme::Keys => {},
                    };

                    res
                }).collect()
            },
        }
    }

    // LEARN: Generators use stackless coroutines, so they can't be recursive. :(
    // pub fn iter_over<'a>(&'a self) -> impl Iterator<Item = &String> + 'a {
    //     let closure = move || {
    //         match *self {
    //             MetaValue::Nil => {},
    //             MetaValue::Str(ref s) => { yield s; },
    //             MetaValue::Seq(ref mvs) => {
    //                 for mv in mvs {
    //                     for i in mv.iter_over() {
    //                         yield i;
    //                     }
    //                 }
    //             },
    //             _ => {},

    //     gen_to_iter(closure)
    // }
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
        let str_sample_c = "Pontifexx".to_string();
        let seq_sample = vec![MetaValue::Str(str_sample_a.clone()), MetaValue::Str(str_sample_b.clone())];

        let mis = MappingIterScheme::Both;

        let inputs_and_expected: Vec<(MetaValue, Vec<&String>)> = vec![
            (MetaValue::Nil, vec![]),
            (MetaValue::Str(str_sample_a.clone()), vec![&str_sample_a]),
            (MetaValue::Seq(seq_sample.clone()), vec![&str_sample_a, &str_sample_b]),
        ];

        for (input, expected) in inputs_and_expected {
            let produced: Vec<&String> = input.flatten(mis);
            assert_eq!(expected, produced);
        }
    }
}
