use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::collections::BTreeMap;

use yaml_rust::{YamlLoader, Yaml};

use metadata::{
    Metadata,
    MetaTarget,
    MetaKey,
    MetaValue,
    MetaBlock,
    MetaBlockSeq,
    MetaBlockMap,
};
use error::*;

pub fn read_yaml_file<P: AsRef<Path>>(yaml_fp: P) -> Result<Yaml> {
    // Opens a YAML file on disk and reads the first document.
    let yaml_fp = yaml_fp.as_ref();
    let mut f = File::open(yaml_fp)?;

    let mut buffer = String::new();
    f.read_to_string(&mut buffer)?;

    let yaml_docs: Vec<Yaml> = YamlLoader::load_from_str(&buffer)?;

    if yaml_docs.len() < 1 {
        Err(ErrorKind::EmptyMetaFile(yaml_fp.to_path_buf()))?
    }

    Ok(yaml_docs[0].clone())
}

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
        Yaml::Null => Some(MetaKey::Nil),
        _ => yaml_as_string(y).map(|s| MetaKey::Str(s)),
    }
}

fn yaml_as_meta_value(y: &Yaml) -> Option<MetaValue> {
    match *y {
        Yaml::Null => Some(MetaValue::Nil),
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

            Some(MetaValue::Seq(seq))
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

            Some(MetaValue::Map(map))
        },
        _ => {
            yaml_as_string(&y).map(|s| MetaValue::Str(s))
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

pub fn yaml_as_meta_block_seq(y: &Yaml) -> Option<MetaBlockSeq> {
    // Try to convert to sequenced item-metadata.
    // We expect a vector of meta blocks.
    match y {
        &Yaml::Array(ref arr) => {
            let mut item_seq = MetaBlockSeq::new();

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

pub fn yaml_as_meta_block_map(y: &Yaml) -> Option<MetaBlockMap> {
    // Try to convert to mapped item-metadata.
    // We expect a mapping of file names to meta blocks.
    match y {
        &Yaml::Hash(ref hsh) => {
            let mut item_map = MetaBlockMap::new();

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

pub fn yaml_as_metadata(y: &Yaml, meta_target: &MetaTarget) -> Option<Metadata> {
    match *meta_target {
        MetaTarget::Contains => {
            yaml_as_meta_block(y).map(|m| Metadata::Contains(m))
        },
        MetaTarget::Siblings => {
            yaml_as_meta_block_seq(y).map(|m| Metadata::SiblingsSeq(m))
                .or(yaml_as_meta_block_map(y).map(|m| Metadata::SiblingsMap(m)))
        },
    }
}

#[cfg(test)]
mod tests {
    use metadata::{
        MetaKey,
        MetaValue,
        MetaBlock,
    };
    use yaml_rust::{YamlLoader};

    use super::{
        yaml_as_string,
        yaml_as_meta_key,
        yaml_as_meta_value,
        yaml_as_meta_block,
    };

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
            ("foo", Some(MetaKey::Str("foo".to_string()))),
            (r#""foo""#, Some(MetaKey::Str("foo".to_string()))),
            (r#"'foo'"#, Some(MetaKey::Str("foo".to_string()))),
            (r#""\"foo\"""#, Some(MetaKey::Str(r#""foo""#.to_string()))),
            (r#""[foo, bar]""#, Some(MetaKey::Str("[foo, bar]".to_string()))),
            (r#""foo: bar""#, Some(MetaKey::Str("foo: bar".to_string()))),
            (r#""foo:    bar""#, Some(MetaKey::Str("foo:    bar".to_string()))),

            // Integers
            ("27", Some(MetaKey::Str("27".to_string()))),
            ("-27", Some(MetaKey::Str("-27".to_string()))),
            // TODO: This does not work, due to it getting parsed as an int and losing the plus.
            // ("+27", Some(MetaKey::Str("+27".to_string()))),

            // Floats
            ("3.14", Some(MetaKey::Str("3.14".to_string()))),
            ("3.14159265358979323846264338327950288419716939937510582", Some(MetaKey::Str("3.14159265358979323846264338327950288419716939937510582".to_string()))),

            // Nulls
            ("~", Some(MetaKey::Nil)),
            ("null", Some(MetaKey::Nil)),

            // Booleans
            ("True", Some(MetaKey::Str("True".to_string()))),
            ("true", Some(MetaKey::Str("true".to_string()))),
            ("False", Some(MetaKey::Str("False".to_string()))),
            ("false", Some(MetaKey::Str("false".to_string()))),

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
            ("foo", Some(MetaValue::Str("foo".to_string()))),
            (r#""foo""#, Some(MetaValue::Str("foo".to_string()))),
            (r#"'foo'"#, Some(MetaValue::Str("foo".to_string()))),
            (r#""\"foo\"""#, Some(MetaValue::Str(r#""foo""#.to_string()))),
            (r#""[foo, bar]""#, Some(MetaValue::Str("[foo, bar]".to_string()))),
            (r#""foo: bar""#, Some(MetaValue::Str("foo: bar".to_string()))),
            (r#""foo:    bar""#, Some(MetaValue::Str("foo:    bar".to_string()))),

            // Integers
            ("27", Some(MetaValue::Str("27".to_string()))),
            ("-27", Some(MetaValue::Str("-27".to_string()))),
            // TODO: This does not work, due to it getting parsed as an int and losing the plus.
            // ("+27", Some(MetaValue::Str("+27".to_string()))),

            // Floats
            ("3.14", Some(MetaValue::Str("3.14".to_string()))),
            ("3.14159265358979323846264338327950288419716939937510582", Some(MetaValue::Str("3.14159265358979323846264338327950288419716939937510582".to_string()))),

            // Nulls
            ("~", Some(MetaValue::Nil)),
            ("null", Some(MetaValue::Nil)),

            // Booleans
            ("True", Some(MetaValue::Str("True".to_string()))),
            ("true", Some(MetaValue::Str("true".to_string()))),
            ("False", Some(MetaValue::Str("False".to_string()))),
            ("false", Some(MetaValue::Str("false".to_string()))),

            // Sequences
            ("- item_a\n- item_b", Some(MetaValue::Seq(vec![
                MetaValue::Str("item_a".to_string()),
                MetaValue::Str("item_b".to_string()),
            ]))),
            ("- item_a", Some(MetaValue::Seq(vec![
                MetaValue::Str("item_a".to_string()),
            ]))),
            ("[item_a, item_b]", Some(MetaValue::Seq(vec![
                MetaValue::Str("item_a".to_string()),
                MetaValue::Str("item_b".to_string()),
            ]))),
            ("[item_a]", Some(MetaValue::Seq(vec![
                MetaValue::Str("item_a".to_string()),
            ]))),
            ("- 27\n- 42", Some(MetaValue::Seq(vec![
                MetaValue::Str("27".to_string()),
                MetaValue::Str("42".to_string()),
            ]))),
            ("- 27\n- null", Some(MetaValue::Seq(vec![
                MetaValue::Str("27".to_string()),
                MetaValue::Nil,
            ]))),

            // Mappings
            ("key_a: val_a\nkey_b: val_b", Some(MetaValue::Map(btreemap![
                MetaKey::Str("key_a".to_string()) => MetaValue::Str("val_a".to_string()),
                MetaKey::Str("key_b".to_string()) => MetaValue::Str("val_b".to_string()),
            ]))),
            ("key_a: val_a", Some(MetaValue::Map(btreemap![
                MetaKey::Str("key_a".to_string()) => MetaValue::Str("val_a".to_string()),
            ]))),
            ("{key_a: val_a, key_b: val_b}", Some(MetaValue::Map(btreemap![
                MetaKey::Str("key_a".to_string()) => MetaValue::Str("val_a".to_string()),
                MetaKey::Str("key_b".to_string()) => MetaValue::Str("val_b".to_string()),
            ]))),
            ("{key_a: val_a}", Some(MetaValue::Map(btreemap![
                MetaKey::Str("key_a".to_string()) => MetaValue::Str("val_a".to_string()),
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
                mb.insert("key_a".to_string(), MetaValue::Str("val_a".to_string()));
                mb.insert("key_b".to_string(), MetaValue::Str("val_b".to_string()));
                Some(mb)
            }),
            ("{key_a: val_a, key_b: val_b}", {
                let mut mb = MetaBlock::new();
                mb.insert("key_a".to_string(), MetaValue::Str("val_a".to_string()));
                mb.insert("key_b".to_string(), MetaValue::Str("val_b".to_string()));
                Some(mb)
            }),
            ("{key_a: [val_a_a, val_a_b, val_a_c], key_b: ~}", {
                let mut mb = MetaBlock::new();
                mb.insert(
                    "key_a".to_string(),
                    MetaValue::Seq(vec![
                        MetaValue::Str("val_a_a".to_string()),
                        MetaValue::Str("val_a_b".to_string()),
                        MetaValue::Str("val_a_c".to_string()),
                    ])
                );
                mb.insert("key_b".to_string(), MetaValue::Nil);
                Some(mb)
            }),
            ("{key_a: {sub_key_a: sub_val_a, sub_key_b: sub_val_b, ~: sub_val_c}, key_b: []}", {
                let mut mb = MetaBlock::new();
                mb.insert(
                    "key_a".to_string(),
                    MetaValue::Map(btreemap![
                        MetaKey::Str("sub_key_a".to_string()) => MetaValue::Str("sub_val_a".to_string()),
                        MetaKey::Str("sub_key_b".to_string()) => MetaValue::Str("sub_val_b".to_string()),
                        MetaKey::Nil => MetaValue::Str("sub_val_c".to_string()),
                    ])
                );
                mb.insert("key_b".to_string(), MetaValue::Seq(vec![]));
                Some(mb)
            }),

            // Skipped entries
            ("{key_a: val_a, [skipped_key, skipped_key]: skipped_val}", {
                let mut mb = MetaBlock::new();
                mb.insert("key_a".to_string(), MetaValue::Str("val_a".to_string()));
                Some(mb)
            }),
            ("{key_a: val_a, {skipped_key_key: skipped_key_val}: skipped_val}", {
                let mut mb = MetaBlock::new();
                mb.insert("key_a".to_string(), MetaValue::Str("val_a".to_string()));
                Some(mb)
            }),
            ("{key_a: val_a, ~: skipped_val}", {
                let mut mb = MetaBlock::new();
                mb.insert("key_a".to_string(), MetaValue::Str("val_a".to_string()));
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
