use std::collections::BTreeMap;

use yaml_rust::{Yaml, YamlLoader};

use metadata::reader::MetaReader;
use metadata::target::MetaTarget;
use metadata::{
    Metadata,
    MetaBlock,
    MetaBlockSeq,
    MetaBlockMap,
    MetaKey,
    MetaValue,
};
use error::*;

pub struct YamlMetaReader;

impl MetaReader for YamlMetaReader {
    fn from_str<S: AsRef<str>>(s: S, mt: MetaTarget) -> Result<Metadata> {
        let s = s.as_ref();
        let yaml_docs: Vec<Yaml> = YamlLoader::load_from_str(s)?;

        ensure!(yaml_docs.len() >= 1, "empty YAML document");
        // if yaml_docs.len() < 1 {
        //     Err(ErrorKind::EmptyMetaFile(yaml_fp.to_path_buf()))?
        // }

        let yaml_doc = &yaml_docs[0];

        yaml_as_metadata(yaml_doc, mt)
    }
}

fn yaml_as_string(y: &Yaml) -> Result<String> {
    match y {
        &Yaml::Null => bail!("cannot convert null to string"),
        &Yaml::Array(_) => bail!("cannot convert sequence to string"),
        &Yaml::Hash(_) => bail!("cannot convert mapping to string"),
        &Yaml::String(ref s) => Ok(s.to_string()),

        // TODO: The rest of these need to be revisited.
        // Ideally we would keep them as strings and not convert when parsing.
        &Yaml::Real(ref r) => Ok(r.to_string()),
        &Yaml::Integer(i) => Ok(i.to_string()),
        &Yaml::Boolean(b) => Ok(b.to_string()),
        &Yaml::Alias(_) => bail!("cannot convert alias to string"),
        &Yaml::BadValue => bail!("cannot convert bad value to string"),
    }
}

fn yaml_as_meta_key(y: &Yaml) -> Result<MetaKey> {
    match *y {
        Yaml::Null => Ok(MetaKey::Nil),
        _ => yaml_as_string(y).map(|s| MetaKey::Str(s)).chain_err(|| "cannot convert YAML to meta key"),
    }
}

fn yaml_as_meta_value(y: &Yaml) -> Result<MetaValue> {
    match *y {
        Yaml::Null => Ok(MetaValue::Nil),
        Yaml::Array(ref arr) => {
            let mut seq: Vec<MetaValue> = vec![];

            // Recursively convert each found YAML item into a meta value.
            for val_y in arr {
                seq.push(yaml_as_meta_value(&val_y)?);
            }

            Ok(MetaValue::Seq(seq))
        },
        Yaml::Hash(ref hsh) => {
            let mut map: BTreeMap<MetaKey, MetaValue> = BTreeMap::new();

            // Recursively convert each found YAML item into a meta value.
            for (key_y, val_y) in hsh {
                let key = yaml_as_meta_key(&key_y)?;
                let val = yaml_as_meta_value(&val_y)?;

                map.insert(key, val);
            }

            Ok(MetaValue::Map(map))
        },
        _ => {
            yaml_as_string(&y).map(|s| MetaValue::Str(s)).chain_err(|| "cannot convert YAML to meta value")
        },
    }
}

fn yaml_as_meta_block(y: &Yaml) -> Result<MetaBlock> {
    // Try to convert to a hash.
    match *y {
        Yaml::Hash(ref hsh) => {
            let mut mb = MetaBlock::new();

            // Keys must be convertible to strings.
            // Values can be any meta value.
            for (key_y, val_y) in hsh {
                let key = yaml_as_string(&key_y)?;
                let val = yaml_as_meta_value(&val_y)?;

                mb.insert(key, val);
            }

            Ok(mb)
        },
        _ => bail!("cannot convert YAML to meta block"),
    }
}

pub fn yaml_as_meta_block_seq(y: &Yaml) -> Result<MetaBlockSeq> {
    // Try to convert to sequenced item-metadata.
    // We expect a vector of meta blocks.
    match y {
        &Yaml::Array(ref arr) => {
            let mut item_seq = MetaBlockSeq::new();

            for val_y in arr {
                item_seq.push(yaml_as_meta_block(&val_y)?);
            }

            Ok(item_seq)
        },
        _ => bail!("cannot convert YAML to meta block sequence"),
    }
}

pub fn yaml_as_meta_block_map(y: &Yaml) -> Result<MetaBlockMap> {
    // Try to convert to mapped item-metadata.
    // We expect a mapping of file names to meta blocks.
    match y {
        &Yaml::Hash(ref hsh) => {
            let mut item_map = MetaBlockMap::new();

            for (key_y, val_y) in hsh {
                let key = yaml_as_string(&key_y)?;
                let val = yaml_as_meta_block(&val_y)?;

                item_map.insert(key, val);
            }

            Ok(item_map)
        },
        _ => bail!("cannot convert YAML to meta block mapping"),
    }
}

pub fn yaml_as_metadata(y: &Yaml, meta_target: MetaTarget) -> Result<Metadata> {
    match meta_target {
        MetaTarget::Contains => {
            yaml_as_meta_block(y).map(|m| Metadata::Contains(m))
        },
        MetaTarget::Siblings => {
            yaml_as_meta_block_seq(y).map(|m| Metadata::SiblingsSeq(m))
                .or(yaml_as_meta_block_map(y).map(|m| Metadata::SiblingsMap(m)))
        },
    }
}
