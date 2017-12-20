// This module provides an interface to "match up" media items with metadata blocks.

use std::path::{Path, PathBuf};
use std::collections::HashSet;

use library::{
    MediaLibrary,
};
use library::sort_order::{
    SortOrder,
};
use library::selection::{
    Selection,
};
use metadata::{
    MetaBlock,
    MetaBlockSeq,
    MetaBlockMap,
    Metadata,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlexTarget {
    WorkingDir,
    SubItem(String),
}

impl PlexTarget {
    pub fn resolve<P: AsRef<Path>>(&self, working_dir_path: P) -> PathBuf {
        let working_dir_path = working_dir_path.as_ref();

        match *self {
            PlexTarget::WorkingDir => working_dir_path.to_path_buf(),
            PlexTarget::SubItem(ref s) => working_dir_path.join(s),
        }
    }

    // LEARN: This would be the way to ideally do it, not needed because a PathBuf is needed later on anyways.
    // pub fn resolve<'a, P: Into<Cow<'a, Path>>>(&self, working_dir_path: P) -> Cow<'a, Path> {
    //     self.resolve_nongeneric(working_dir_path.into())
    // }

    // fn resolve_nongeneric<'a>(&self, working_dir_path: Cow<'a, Path>) -> Cow<'a, Path> {
    //     match *self {
    //         PlexTarget::WorkingDir => working_dir_path,
    //         PlexTarget::SubItem(ref s) => Cow::Owned(working_dir_path.join(s)),
    //     }
    // }
}

pub type PlexRecord<'a> = (PlexTarget, &'a MetaBlock);

pub fn multiplex<'a, P: AsRef<Path>>(metadata: &'a Metadata, working_dir_path: P, selection: &Selection, sort_order: SortOrder) -> Vec<PlexRecord<'a>> {
    let item_names: Vec<_> = metadata.source_item_names(working_dir_path, selection, sort_order);

    plex(metadata, &item_names)
}

fn plex<'a, I, J>(metadata: &Metadata, item_names: I) -> Vec<PlexRecord>
where I: IntoIterator<Item = &'a J>,
      J: AsRef<str> + 'a
{
    match *metadata {
        Metadata::Contains(ref mb) => plex_singular(&mb),
        Metadata::SiblingsSeq(ref mb_seq) => plex_multiple_seq(mb_seq, item_names),
        Metadata::SiblingsMap(ref mb_map) => plex_multiple_map(mb_map, item_names),
    }
}

fn plex_singular(meta_block: &MetaBlock) -> Vec<PlexRecord> {
    vec![(PlexTarget::WorkingDir, meta_block)]
}

fn plex_multiple_seq<'a, I, J>(meta_block_seq: &MetaBlockSeq, item_names: I) -> Vec<PlexRecord>
where I: IntoIterator<Item = &'a J>,
      J: AsRef<str> + 'a
{
    // Growable vector of results.
    let mut results: Vec<PlexRecord> = vec![];

    // Metadata is a sequence of meta blocks.
    // Each should correspond one-to-one with a valid item in the working dir.
    let item_names: Vec<_> = item_names.into_iter().collect();

    if meta_block_seq.len() > item_names.len() {
        warn!("Excess metadata definitions found ({})", meta_block_seq.len() - item_names.len());
    }
    else if meta_block_seq.len() < item_names.len() {
        warn!("Excess item entries found ({})", item_names.len() - meta_block_seq.len());
    }

    for (item_file_name, mb) in item_names.iter().zip(meta_block_seq) {
        results.push((PlexTarget::SubItem(item_file_name.as_ref().to_string()), mb));
    }

    results
}

fn plex_multiple_map<'a, I, J>(meta_block_map: &MetaBlockMap, item_names: I) -> Vec<PlexRecord>
where I: IntoIterator<Item = &'a J>,
      J: AsRef<str> + 'a
{
    // Growable vector of results.
    let mut results: Vec<PlexRecord> = vec![];

    // Metadata is a mapping of item file names to meta blocks.
    // Collect a mutable set of the expected item names.
    let mut remaining_item_names: HashSet<_> = item_names.into_iter().map(AsRef::as_ref).collect();

    for (item_file_name, mb) in meta_block_map {
        // Check if the item name is valid.
        if !MediaLibrary::is_valid_item_name(&item_file_name) {
            warn!(r#"Item name "{}" is invalid"#, item_file_name);
            continue;
        }

        // Check if the item name from metadata is found in the set.
        if !remaining_item_names.remove(item_file_name.as_str()) {
            warn!(r#"Item name "{}" was not found in expected item names"#, item_file_name);
            continue;
        }

        results.push((PlexTarget::SubItem(item_file_name.to_string()), mb));
    }

    // Warn if any names remain in the set.
    if remaining_item_names.len() > 0 {
        warn!(r#"Excess item entries found ({})"#, remaining_item_names.len());
    }

    results
}


#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        plex_singular,
        plex_multiple_seq,
        plex_multiple_map,
        PlexTarget,
    };
    use metadata::{
        MetaBlock,
        MetaBlockSeq,
        MetaBlockMap,
        MetaValue,
    };

    #[test]
    fn test_plex_singular() {
        let mb: MetaBlock = btreemap![
            String::from("artist") => MetaValue::String(String::from("lapix")),
            String::from("title") => MetaValue::String(String::from("Core Signal")),
        ];

        let expected = vec![
            (PlexTarget::WorkingDir, &mb),
        ];
        let produced = plex_singular(&mb);

        assert_eq!(expected, produced);
    }

    #[test]
    fn test_plex_multiple_seq() {
        let mb_seq: MetaBlockSeq = vec![
            btreemap![
                String::from("artist") => MetaValue::Sequence(vec![
                    MetaValue::String(String::from("MK")),
                    MetaValue::String(String::from("Kanae Asaba")),
                ]),
                String::from("title") => MetaValue::String(String::from("I'm Falling Love With You")),
            ],
            btreemap![
                String::from("artist") => MetaValue::String(String::from("Taishi")),
                String::from("title") => MetaValue::String(String::from("Floating Disk")),
            ],
            btreemap![
                String::from("artist") => MetaValue::String(String::from("Nhato")),
                String::from("title") => MetaValue::String(String::from("Jupiter Junction")),
            ],
        ];

        let names: Vec<&str> = vec!["TRACK01.flac", "TRACK02.flac", "TRACK03.flac"];

        let expected = vec![
            (PlexTarget::SubItem(names[0].to_string()), &mb_seq[0]),
            (PlexTarget::SubItem(names[1].to_string()), &mb_seq[1]),
            (PlexTarget::SubItem(names[2].to_string()), &mb_seq[2]),
        ];
        let produced = plex_multiple_seq(&mb_seq, &names);

        assert_eq!(expected, produced);
    }

    #[test]
    fn test_plex_multiple_map() {
        let mb_map: MetaBlockMap = btreemap![
            String::from("TRACK01.flac") => btreemap![
                String::from("artist") => MetaValue::Sequence(vec![
                    MetaValue::String(String::from("MK")),
                    MetaValue::String(String::from("Kanae Asaba")),
                ]),
                String::from("title") => MetaValue::String(String::from("I'm Falling Love With You")),
            ],
            String::from("TRACK02.flac") => btreemap![
                String::from("artist") => MetaValue::String(String::from("Taishi")),
                String::from("title") => MetaValue::String(String::from("Floating Disk")),
            ],
            String::from("TRACK03.flac") => btreemap![
                String::from("artist") => MetaValue::String(String::from("Nhato")),
                String::from("title") => MetaValue::String(String::from("Jupiter Junction")),
            ],
        ];

        let names: Vec<&str> = vec!["TRACK01.flac", "TRACK02.flac", "TRACK03.flac"];

        let expected = hashset![
            (PlexTarget::SubItem(names[1].to_string()), &mb_map["TRACK02.flac"]),
            (PlexTarget::SubItem(names[0].to_string()), &mb_map["TRACK01.flac"]),
            (PlexTarget::SubItem(names[2].to_string()), &mb_map["TRACK03.flac"]),
        ];
        let produced: HashSet<_> = plex_multiple_map(&mb_map, &names).into_iter().collect();

        assert_eq!(expected, produced);
    }
}
