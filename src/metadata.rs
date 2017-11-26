use std::path::{Path, PathBuf};
use std::collections::BTreeMap;

use yaml_rust::Yaml;

use generator::gen_to_iter;
use library::{MediaLibrary, SortOrder, Selection};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum MetaKey {
    Null,
    String(String),
}

#[derive(PartialEq, Debug)]
pub enum MetaValue {
    Null,
    String(String),
    Sequence(Vec<MetaValue>),
    Mapping(BTreeMap<MetaKey, MetaValue>),
}

pub type MetaBlock = BTreeMap<String, MetaValue>;
pub type SelfMetadata = MetaBlock;
pub type ItemSeqMetadata = Vec<MetaBlock>;
pub type ItemMapMetadata = BTreeMap<String, MetaBlock>;

fn yaml_as_meta_key(y: &Yaml) -> Option<MetaKey> {
    match y {
        &Yaml::Null => Some(MetaKey::Null),
        &Yaml::Array(_) => None,
        &Yaml::Hash(_) => None,
        &Yaml::String(ref s) => Some(MetaKey::String(s.to_string())),

        // TODO: The rest of these need to be revisited.
        // Ideally we would keep them as strings and not convert when parsing.
        &Yaml::Real(ref r) => Some(MetaKey::String(r.to_string())),
        &Yaml::Integer(i) => Some(MetaKey::String(i.to_string())),
        &Yaml::Boolean(b) => Some(MetaKey::String(b.to_string())),
        &Yaml::Alias(_) => None,
        &Yaml::BadValue => None,
    }
}

fn yaml_as_meta_value(y: &Yaml) -> Option<MetaValue> {
    match y {
        &Yaml::Null => Some(MetaValue::Null),
        &Yaml::Array(ref arr) => {
            let mut seq: Vec<MetaValue> = vec![];

            // Recursively convert each found YAML item into a meta value.
            for sy in arr {
                if let Some(conv_val) = yaml_as_meta_value(&sy) {
                    seq.push(conv_val);
                } else {
                    // TODO: Log that an unexpected value was found.
                }
            }

            Some(MetaValue::Sequence(seq))
        },
        &Yaml::Hash(ref hsh) => {
            let mut map: BTreeMap<MetaKey, MetaValue> = BTreeMap::new();

            // Recursively convert each found YAML item into a meta value.
            for (sk, sv) in hsh {
                let conv_key = yaml_as_meta_key(&sk);
                let conv_val = yaml_as_meta_value(&sv);

                if let (Some(key), Some(val)) = (conv_key, conv_val) {
                    map.insert(key, val);
                } else {
                    // TODO: Log that an unexpected value was found.
                }
            }

            Some(MetaValue::Mapping(map))
        },
        &Yaml::String(ref s) => Some(MetaValue::String(s.to_string())),

        // TODO: The rest of these need to be revisited.
        // Ideally we would keep them as strings and not convert when parsing.
        &Yaml::Real(ref r) => Some(MetaValue::String(r.to_string())),
        &Yaml::Integer(i) => Some(MetaValue::String(i.to_string())),
        &Yaml::Boolean(b) => Some(MetaValue::String(b.to_string())),
        &Yaml::Alias(_) => None,
        &Yaml::BadValue => None,
    }
}

pub fn yaml_as_self_metadata(y: &Yaml) -> SelfMetadata {
    // Try to convert to self-metadata.
    SelfMetadata::new()
}

#[cfg(test)]
mod tests {
    use super::{MetaKey, MetaValue, yaml_as_meta_key, yaml_as_meta_value};
    use yaml_rust::{Yaml, YamlLoader};

    #[test]
    fn test_yaml_as_meta_key() {
        let inputs_and_expected = vec![
            // Strings
            ("foo", Some(MetaKey::String("foo".to_string()))),
            (r#""foo""#, Some(MetaKey::String("foo".to_string()))),
            (r#"'foo'"#, Some(MetaKey::String("foo".to_string()))),
            (r#""\"foo\"""#, Some(MetaKey::String(r#""foo""#.to_string()))),
            (r#""[foo, bar]""#, Some(MetaKey::String("[foo, bar]".to_string()))),
            (r#""foo: bar""#, Some(MetaKey::String("foo: bar".to_string()))),
            (r#""foo:    bar""#, Some(MetaKey::String("foo:    bar".to_string()))),

            // Integers
            ("27", Some(MetaKey::String("27".to_string()))),
            ("-27", Some(MetaKey::String("-27".to_string()))),
            // TODO: This does not work, due to it getting parsed as an int and losing the plus.
            // ("+27", Some(MetaKey::String("+27".to_string()))),

            // Floats
            ("3.14", Some(MetaKey::String("3.14".to_string()))),
            ("3.14159265358979323846264338327950288419716939937510582", Some(MetaKey::String("3.14159265358979323846264338327950288419716939937510582".to_string()))),

            // Nulls
            ("~", Some(MetaKey::Null)),
            ("null", Some(MetaKey::Null)),

            // Booleans
            ("True", Some(MetaKey::String("True".to_string()))),
            ("true", Some(MetaKey::String("true".to_string()))),
            ("False", Some(MetaKey::String("False".to_string()))),
            ("false", Some(MetaKey::String("false".to_string()))),

            // Sequences
            ("- item_a\n- item_b", None),
            ("- item_a", None),
            ("[item_a, item_b]", None),
            ("[item_a]", None),

            // Mappings
            ("key_a: val_a\nkey_b: val_b", None),
            ("key_a: val_a", None),
            ("{key_a: val_a, key_b: val_b}", None),
            ("{key_a: val_a}", None),

            // Aliases
        ];

        for (input, expected) in inputs_and_expected {
            let yaml = &YamlLoader::load_from_str(input).unwrap()[0];
            let produced = yaml_as_meta_key(yaml);
            assert_eq!(expected, produced);
        }
    }

    #[test]
    fn test_yaml_as_meta_value() {
        let inputs_and_expected = vec![
            // Strings
            ("foo", Some(MetaValue::String("foo".to_string()))),
            (r#""foo""#, Some(MetaValue::String("foo".to_string()))),
            (r#"'foo'"#, Some(MetaValue::String("foo".to_string()))),
            (r#""\"foo\"""#, Some(MetaValue::String(r#""foo""#.to_string()))),
            (r#""[foo, bar]""#, Some(MetaValue::String("[foo, bar]".to_string()))),
            (r#""foo: bar""#, Some(MetaValue::String("foo: bar".to_string()))),
            (r#""foo:    bar""#, Some(MetaValue::String("foo:    bar".to_string()))),

            // Integers
            ("27", Some(MetaValue::String("27".to_string()))),
            ("-27", Some(MetaValue::String("-27".to_string()))),
            // TODO: This does not work, due to it getting parsed as an int and losing the plus.
            // ("+27", Some(MetaValue::String("+27".to_string()))),

            // Floats
            ("3.14", Some(MetaValue::String("3.14".to_string()))),
            ("3.14159265358979323846264338327950288419716939937510582", Some(MetaValue::String("3.14159265358979323846264338327950288419716939937510582".to_string()))),

            // Nulls
            ("~", Some(MetaValue::Null)),
            ("null", Some(MetaValue::Null)),

            // Booleans
            ("True", Some(MetaValue::String("True".to_string()))),
            ("true", Some(MetaValue::String("true".to_string()))),
            ("False", Some(MetaValue::String("False".to_string()))),
            ("false", Some(MetaValue::String("false".to_string()))),

            // Sequences
            ("- item_a\n- item_b", Some(MetaValue::Sequence(vec![
                MetaValue::String("item_a".to_string()),
                MetaValue::String("item_b".to_string()),
            ]))),
            ("- item_a", Some(MetaValue::Sequence(vec![
                MetaValue::String("item_a".to_string()),
            ]))),
            ("[item_a, item_b]", Some(MetaValue::Sequence(vec![
                MetaValue::String("item_a".to_string()),
                MetaValue::String("item_b".to_string()),
            ]))),
            ("[item_a]", Some(MetaValue::Sequence(vec![
                MetaValue::String("item_a".to_string()),
            ]))),
            ("- 27\n- 42", Some(MetaValue::Sequence(vec![
                MetaValue::String("27".to_string()),
                MetaValue::String("42".to_string()),
            ]))),
            ("- 27\n- null", Some(MetaValue::Sequence(vec![
                MetaValue::String("27".to_string()),
                MetaValue::Null,
            ]))),

            // Mappings
            ("key_a: val_a\nkey_b: val_b", Some(MetaValue::Mapping(btreemap![
                MetaKey::String("key_a".to_string()) => MetaValue::String("val_a".to_string()),
                MetaKey::String("key_b".to_string()) => MetaValue::String("val_b".to_string()),
            ]))),
            ("key_a: val_a", Some(MetaValue::Mapping(btreemap![
                MetaKey::String("key_a".to_string()) => MetaValue::String("val_a".to_string()),
            ]))),
            ("{key_a: val_a, key_b: val_b}", Some(MetaValue::Mapping(btreemap![
                MetaKey::String("key_a".to_string()) => MetaValue::String("val_a".to_string()),
                MetaKey::String("key_b".to_string()) => MetaValue::String("val_b".to_string()),
            ]))),
            ("{key_a: val_a}", Some(MetaValue::Mapping(btreemap![
                MetaKey::String("key_a".to_string()) => MetaValue::String("val_a".to_string()),
            ]))),

            // Aliases
        ];

        for (input, expected) in inputs_and_expected {
            let yaml = &YamlLoader::load_from_str(input).unwrap()[0];
            let produced = yaml_as_meta_value(yaml);
            assert_eq!(expected, produced);
        }
    }
}

// pub enum Metadata {
//     SelfMeta(MetaBlock),
//     SeqItemMeta(Vec<MetaBlock>),
//     MapItemMeta(BTreeMap<String, MetaBlock>),
// }

// impl Metadata {
//     // LEARN: It seems that using `impl trait` always requires a named lifetime.
//     pub fn discover<'a, P: AsRef<Path> + 'a>(&'a self,
//             rel_target_dir: P,
//             media_lib: &'a MediaLibrary)
//             -> impl Iterator<Item = (PathBuf, MetaBlock)> + 'a
//     {
//         let closure = move || {
//             let normed = media_lib.co_norm(rel_target_dir);

//             if let Ok((rel_target_dir, abs_target_dir)) = normed {
//                 if !abs_target_dir.is_dir() {
//                     return
//                 }

//                 match self {
//                     &Metadata::SelfMeta(ref block) => {
//                         yield (abs_target_dir, block.clone())
//                     },
//                     &Metadata::SeqItemMeta(ref vec_blocks) => {
//                         let items = media_lib.sort_entries(media_lib.filtered_entries_in_dir(&rel_target_dir));

//                         // TODO: Add warning here for mismatched counts.
//                         for pair in items.iter().zip(vec_blocks) {
//                             println!{"{:?}", pair};
//                         }
//                     },
//                     _ => {
//                     },
//                 }
//             }
//             else {
//                 return
//             }
//         };

//         gen_to_iter(closure)
//     }
// }

// pub fn example() {
//     let selection = Selection::Or(
//         Box::new(Selection::IsDir),
//         Box::new(Selection::And(
//             Box::new(Selection::IsFile),
//             Box::new(Selection::Ext("flac".to_string())),
//         )),
//     );

//     let media_lib = MediaLibrary::new("/home/lemoine/Music",
//             "taggu_item.yml",
//             "taggu_self.yml",
//             selection,
//             SortOrder::Name,
//     ).unwrap();
// }
