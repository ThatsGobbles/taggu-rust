use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::fs::DirEntry;
use std::iter::{empty, once};

use helpers::normalize;
use library::sort_order::SortOrder;
use library::selection::Selection;
use error::*;
use generator::gen_to_iter;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub enum MetaAtom {
    Nil,
    Str(String),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub enum MetaKey {
    Nil,
    Str(String),
}

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum MetaValue {
    Nil,
    Str(String),
    Seq(Vec<MetaValue>),
    Map(BTreeMap<MetaKey, MetaValue>),
}

impl MetaValue {
    pub fn collect_data<'a>(&'a self) -> Vec<&'a String> {
        match *self {
            MetaValue::Nil => vec![],
            MetaValue::Str(ref s) => vec![s],
            MetaValue::Seq(ref mvs) => mvs.iter().flat_map(|x| x.collect_data()).collect(),
            MetaValue::Map(ref map) => {
                let mut vals = vec![];

                for (mk, mv) in map {
                    match *mk {
                        MetaKey::Nil => {},
                        MetaKey::Str(ref s) => { vals.push(s); },
                    }

                    for i in mv.collect_data() {
                        vals.push(i);
                    }
                }

                vals
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
    //             MetaValue::Map(ref map) => {
    //                 // TODO: Need to handle null key first.
    //                 for (mk, mv) in map {
    //                     match *mk {
    //                         MetaKey::Nil => {},
    //                         MetaKey::Str(ref s) => { yield s; },
    //                     }

    //                     for i in mv.iter_over() {
    //                         yield i;
    //                     }
    //                 }
    //             },
    //             _ => {},
    //         }
    //     };

    //     gen_to_iter(closure)
    // }
}

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum MetaIterValue {
    Nil,
    Str(String),
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
            MetaTarget::Contains => match abs_meta_path.is_dir() {
                true => Some(abs_meta_path),
                false => None,
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

#[cfg(test)]
mod tests {
    use super::{
        MetaValue,
    };

    #[test]
    fn test_meta_value_collect_data() {
        let str_sample_a = "Goldfish".to_string();
        let str_sample_b = "DIMMI".to_string();
        let seq_sample = vec![MetaValue::Str(str_sample_a.clone()), MetaValue::Str(str_sample_b.clone())];

        let inputs_and_expected: Vec<(MetaValue, Vec<&String>)> = vec![
            (MetaValue::Nil, vec![]),
            (MetaValue::Str(str_sample_a.clone()), vec![&str_sample_a]),
            (MetaValue::Seq(seq_sample.clone()), vec![&str_sample_a, &str_sample_b]),
        ];

        for (input, expected) in inputs_and_expected {
            let produced: Vec<&String> = input.collect_data();
            assert_eq!(expected, produced);
        }
    }
}
