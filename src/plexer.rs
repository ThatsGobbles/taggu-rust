// This module provides an interface to "match up" media items with metadata blocks.

use std::collections::HashSet;

use library::MediaLibrary;
use metadata::{
    MetaBlock,
    MetaBlockSeq,
    MetaBlockMap,
    MetaTarget,
    Metadata,
    SelfMetaFormat,
    ItemMetaFormat,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlexTarget {
    WorkingDir,
    SubItem(String),
}

pub type PlexRecord<'a> = (PlexTarget, &'a MetaBlock);

pub fn plex<'a, I, J>(metadata: &Metadata, selected_item_names: I) -> Vec<PlexRecord>
where I: IntoIterator<Item = &'a J>,
      J: AsRef<str> + 'a
{
    match *metadata {
        Metadata::SelfMetadata(ref smf) => plex_self_format(smf),
        Metadata::ItemMetadata(ref imf) => plex_item_format(imf, selected_item_names)
    }
}

pub fn plex_self_format(smf: &SelfMetaFormat) -> Vec<PlexRecord> {
    // This will yield only a single item path: the working directory path itself.
    match *smf {
        SelfMetaFormat::Def(ref mb) => vec![(PlexTarget::WorkingDir, mb)],
    }
}

pub fn plex_item_format<'a, I, J>(imf: &ItemMetaFormat, selected_item_names: I) -> Vec<PlexRecord>
where I: IntoIterator<Item = &'a J>,
      J: AsRef<str> + 'a
{
    match *imf {
        ItemMetaFormat::Seq(ref mb_seq) => plex_meta_block_seq(mb_seq, selected_item_names),
        ItemMetaFormat::Map(ref mb_map) => plex_meta_block_map(mb_map, selected_item_names),
    }
}

fn plex_meta_block_seq<'a, I, J>(mb_seq: &MetaBlockSeq, selected_item_names: I) -> Vec<PlexRecord>
where I: IntoIterator<Item = &'a J>,
      J: AsRef<str> + 'a
{
    // Growable vector of results.
    let mut results: Vec<PlexRecord> = vec![];

    // Metadata is a sequence of meta blocks.
    // Each should correspond one-to-one with a valid item in the working dir.
    let sorted_selected_item_names: Vec<_> = selected_item_names.into_iter().collect();

    if mb_seq.len() != sorted_selected_item_names.len() {
        warn!("Lengths do not match!");
    }

    for (item_file_name, mb) in sorted_selected_item_names.iter().zip(mb_seq) {
        results.push((PlexTarget::SubItem(item_file_name.as_ref().to_string()), mb));
    }

    results
}

fn plex_meta_block_map<'a, I, J>(mb_map: &MetaBlockMap, selected_item_names: I) -> Vec<PlexRecord>
where I: IntoIterator<Item = &'a J>,
      J: AsRef<str> + 'a
{
    // Growable vector of results.
    let mut results: Vec<PlexRecord> = vec![];

    // Metadata is a mapping of item file names to meta blocks.
    // Collect a mutable set of the expected item names.
    let mut remaining_expected_item_names: HashSet<_> = selected_item_names.into_iter().map(AsRef::as_ref).collect();

    for (item_file_name, mb) in mb_map {
        // Check if the file name is valid.
        if !MediaLibrary::is_valid_item_name(&item_file_name) {
            warn!(r#"Item name "{}" is invalid"#, item_file_name);
            continue;
        }

        // Check if the item name from metadata is found in the set.
        if !remaining_expected_item_names.remove(item_file_name.as_str()) {
            warn!(r#"Item name "{}" was not found in the directory"#, item_file_name);
            continue;
        }

        results.push((PlexTarget::SubItem(item_file_name.to_string()), mb));
    }

    // Warn if any names remain in the set.
    if remaining_expected_item_names.len() > 0 {
        warn!(r#"There are unaccounted-for item names remaining"#);
    }

    results
}


// =================================================================================================
// TESTS
// =================================================================================================

#[cfg(test)]
mod tests {
    use super::{
        plex_self_format,
        plex_meta_block_seq,
        PlexTarget,
    };
    use metadata::{
        MetaBlock,
        MetaBlockSeq,
        MetaValue,
        SelfMetaFormat,
        ItemMetaFormat,
    };

    #[test]
    fn test_plex_self_format() {
        let mb: MetaBlock = btreemap![
            String::from("artist") => MetaValue::String(String::from("lapix")),
            String::from("title") => MetaValue::String(String::from("Core Signal")),
        ];
        let smf: SelfMetaFormat = SelfMetaFormat::Def(mb.clone());

        let expected = vec![
            (PlexTarget::WorkingDir, &mb),
        ];
        let produced = plex_self_format(&smf);

        assert_eq!(expected, produced);
    }

    #[test]
    fn test_plex_meta_block_seq() {
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

        // let imf: ItemMetaFormat = ItemMetaFormat::Seq(mb_seq.clone());
        let names: Vec<&str> = vec!["TRACK01.flac", "TRACK02.flac", "TRACK03.flac"];

        let expected = vec![
            (PlexTarget::SubItem(names[0].to_string()), &mb_seq[0]),
            (PlexTarget::SubItem(names[1].to_string()), &mb_seq[1]),
            (PlexTarget::SubItem(names[2].to_string()), &mb_seq[2]),
        ];
        let produced = plex_meta_block_seq(&mb_seq, &names);

        assert_eq!(expected, produced);
    }
}
