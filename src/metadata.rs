use std::collections::BTreeMap;
use std::path::PathBuf;

use yaml_rust::Yaml;

use path::normalize;

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

pub enum MetaTarget {
    // TODO: Ensure that the file names are simple and do not contain any dot-refs or slashes.
    Alongside(String),
    Container(String),
}

impl MetaTarget {
    pub fn meta_file_name(&self) -> &String {
        match *self {
            MetaTarget::Alongside(ref x) => x,
            MetaTarget::Container(ref x) => x,
        }
    }

    pub fn target_dir_path<P: Into<PathBuf>>(&self, abs_item_path: P) -> Option<PathBuf> {
        let abs_item_path = normalize(&abs_item_path.into());

        if !abs_item_path.exists() {
            return None
        }

        match *self {
            MetaTarget::Alongside(_) => abs_item_path.parent().map(|f| f.to_path_buf()),
            MetaTarget::Container(_) => {
                if abs_item_path.is_dir() { Some(abs_item_path) }
                else { None }
            },
        }
    }

    pub fn meta_file_path<P: Into<PathBuf>>(&self, abs_item_path: P) -> Option<PathBuf> {
        self.target_dir_path(abs_item_path)
            .map(|f| f.join(self.meta_file_name()))
            .and_then(|f| if f.is_file() { Some(f) } else { None })
    }
}

pub type MetaBlock = BTreeMap<String, MetaValue>;
pub type SelfMetadata = MetaBlock;
pub type ItemSeqMetadata = Vec<MetaBlock>;
pub type ItemMapMetadata = BTreeMap<String, MetaBlock>;

fn yaml_as_string(y: &Yaml) -> Option<String> {
    match y {
        &Yaml::Null => None,
        &Yaml::Array(_) => None,
        &Yaml::Hash(_) => None,
        &Yaml::String(ref s) => Some(s.to_string()),

        // TODO: The rest of these need to be revisited.
        // Ideally we would keep them as strings and not convert when parsing.
        &Yaml::Real(ref r) => Some(r.to_string()),
        &Yaml::Integer(i) => Some(i.to_string()),
        &Yaml::Boolean(b) => Some(b.to_string()),
        &Yaml::Alias(_) => None,
        &Yaml::BadValue => None,
    }
}

fn yaml_as_meta_key(y: &Yaml) -> Option<MetaKey> {
    match *y {
        Yaml::Null => Some(MetaKey::Null),
        _ => yaml_as_string(y).map(|s| MetaKey::String(s)),
    }
}

fn yaml_as_meta_value(y: &Yaml) -> Option<MetaValue> {
    match *y {
        Yaml::Null => Some(MetaValue::Null),
        Yaml::Array(ref arr) => {
            let mut seq: Vec<MetaValue> = vec![];

            // Recursively convert each found YAML item into a meta value.
            for val_y in arr {
                if let Some(val) = yaml_as_meta_value(&val_y) {
                    seq.push(val);
                } else {
                    // TODO: Log that an unexpected value was found.
                }
            }

            Some(MetaValue::Sequence(seq))
        },
        Yaml::Hash(ref hsh) => {
            let mut map: BTreeMap<MetaKey, MetaValue> = BTreeMap::new();

            // Recursively convert each found YAML item into a meta value.
            for (key_y, val_y) in hsh {
                let maybe_key = yaml_as_meta_key(&key_y);
                let maybe_val = yaml_as_meta_value(&val_y);

                if let (Some(key), Some(val)) = (maybe_key, maybe_val) {
                    map.insert(key, val);
                } else {
                    // TODO: Log that an unexpected value was found.
                }
            }

            Some(MetaValue::Mapping(map))
        },
        _ => {
            yaml_as_string(&y).map(|s| MetaValue::String(s))
        },
    }
}

fn yaml_as_meta_block(y: &Yaml) -> Option<MetaBlock> {
    // Try to convert to a hash.
    match *y {
        Yaml::Hash(ref hsh) => {
            let mut mb = MetaBlock::new();

            // Keys must be convertible to strings.
            // Values can be any meta value.
            for (key_y, val_y) in hsh {
                let maybe_key = yaml_as_string(&key_y);
                let maybe_val = yaml_as_meta_value(&val_y);

                if let (Some(key), Some(val)) = (maybe_key, maybe_val) {
                    mb.insert(key, val);
                } else {
                    // TODO: Log that an unexpected value was found.
                }
            }

            Some(mb)
        },
        _ => None,
    }
}

pub fn yaml_as_self_metadata(y: &Yaml) -> Option<SelfMetadata> {
    // Try to convert to self-metadata.
    // We expect a meta block.
    yaml_as_meta_block(y)
}

pub fn yaml_as_item_seq_metadata(y: &Yaml) -> Option<ItemSeqMetadata> {
    // Try to convert to sequenced item-metadata.
    // We expect a vector of meta blocks.
    match y {
        &Yaml::Array(ref arr) => {
            let mut item_seq = ItemSeqMetadata::new();

            for val_y in arr {
                if let Some(mb) = yaml_as_meta_block(&val_y) {
                    item_seq.push(mb);
                } else {
                    // TODO: Log that an unexpected value was found.
                }
            }

            Some(item_seq)
        },
        _ => None,
    }
}

pub fn yaml_as_item_map_metadata(y: &Yaml) -> Option<ItemMapMetadata> {
    // Try to convert to mapped item-metadata.
    // We expect a mapping of file names to meta blocks.
    match y {
        &Yaml::Hash(ref hsh) => {
            let mut item_map = ItemMapMetadata::new();

            for (key_y, val_y) in hsh {
                let maybe_key = yaml_as_string(&key_y);
                let maybe_val = yaml_as_meta_block(&val_y);

                if let (Some(key), Some(val)) = (maybe_key, maybe_val) {
                    item_map.insert(key, val);
                } else {
                    // TODO: Log that an unexpected value was found.
                }
            }

            Some(item_map)
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MetaKey,
        MetaValue,
        MetaBlock,
        yaml_as_string,
        yaml_as_meta_key,
        yaml_as_meta_value,
        yaml_as_meta_block,
    };
    use yaml_rust::{YamlLoader};

    #[test]
    fn test_yaml_as_string() {
        let inputs_and_expected = vec![
            // Strings
            ("foo", Some("foo".to_string())),
            (r#""foo""#, Some("foo".to_string())),
            (r#"'foo'"#, Some("foo".to_string())),
            (r#""\"foo\"""#, Some(r#""foo""#.to_string())),
            (r#""[foo, bar]""#, Some("[foo, bar]".to_string())),
            (r#""foo: bar""#, Some("foo: bar".to_string())),
            (r#""foo:    bar""#, Some("foo:    bar".to_string())),

            // Integers
            ("27", Some("27".to_string())),
            ("-27", Some("-27".to_string())),
            // TODO: This does not work, due to it getting parsed as an int and losing the plus.
            // ("+27", Some("+27".to_string())),

            // Floats
            ("3.14", Some("3.14".to_string())),
            ("3.14159265358979323846264338327950288419716939937510582", Some("3.14159265358979323846264338327950288419716939937510582".to_string())),

            // Nulls
            ("~", None),
            ("null", None),

            // Booleans
            ("True", Some("True".to_string())),
            ("true", Some("true".to_string())),
            ("False", Some("False".to_string())),
            ("false", Some("false".to_string())),

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
            let produced = yaml_as_string(yaml);
            assert_eq!(expected, produced);
        }
    }

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

    #[test]
    fn test_yaml_as_meta_block() {
        let inputs_and_expected = vec![
            // Invalid blocks
            ("foo", None),
            ("27", None),
            ("-27", None),
            ("3.14", None),
            ("3.14159265358979323846264338327950288419716939937510582", None),
            ("~", None),
            ("null", None),
            ("true", None),
            ("false", None),
            ("- item_a\n- item_b", None),
            ("[item_a, item_b]", None),

            // Valid blocks
            ("key_a: val_a\nkey_b: val_b", {
                let mut mb = MetaBlock::new();
                mb.insert("key_a".to_string(), MetaValue::String("val_a".to_string()));
                mb.insert("key_b".to_string(), MetaValue::String("val_b".to_string()));
                Some(mb)
            }),
            ("{key_a: val_a, key_b: val_b}", {
                let mut mb = MetaBlock::new();
                mb.insert("key_a".to_string(), MetaValue::String("val_a".to_string()));
                mb.insert("key_b".to_string(), MetaValue::String("val_b".to_string()));
                Some(mb)
            }),
            ("{key_a: [val_a_a, val_a_b, val_a_c], key_b: ~}", {
                let mut mb = MetaBlock::new();
                mb.insert(
                    "key_a".to_string(),
                    MetaValue::Sequence(vec![
                        MetaValue::String("val_a_a".to_string()),
                        MetaValue::String("val_a_b".to_string()),
                        MetaValue::String("val_a_c".to_string()),
                    ])
                );
                mb.insert("key_b".to_string(), MetaValue::Null);
                Some(mb)
            }),
            ("{key_a: {sub_key_a: sub_val_a, sub_key_b: sub_val_b, ~: sub_val_c}, key_b: []}", {
                let mut mb = MetaBlock::new();
                mb.insert(
                    "key_a".to_string(),
                    MetaValue::Mapping(btreemap![
                        MetaKey::String("sub_key_a".to_string()) => MetaValue::String("sub_val_a".to_string()),
                        MetaKey::String("sub_key_b".to_string()) => MetaValue::String("sub_val_b".to_string()),
                        MetaKey::Null => MetaValue::String("sub_val_c".to_string()),
                    ])
                );
                mb.insert("key_b".to_string(), MetaValue::Sequence(vec![]));
                Some(mb)
            }),

            // Skipped entries
            ("{key_a: val_a, [skipped_key, skipped_key]: skipped_val}", {
                let mut mb = MetaBlock::new();
                mb.insert("key_a".to_string(), MetaValue::String("val_a".to_string()));
                Some(mb)
            }),
            ("{key_a: val_a, {skipped_key_key: skipped_key_val}: skipped_val}", {
                let mut mb = MetaBlock::new();
                mb.insert("key_a".to_string(), MetaValue::String("val_a".to_string()));
                Some(mb)
            }),
            ("{key_a: val_a, ~: skipped_val}", {
                let mut mb = MetaBlock::new();
                mb.insert("key_a".to_string(), MetaValue::String("val_a".to_string()));
                Some(mb)
            }),
        ];

        for (input, expected) in inputs_and_expected {
            let yaml = &YamlLoader::load_from_str(input).unwrap()[0];
            let produced = yaml_as_meta_block(yaml);
            assert_eq!(expected, produced);
        }
    }
}
