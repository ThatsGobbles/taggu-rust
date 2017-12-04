use std::fs::File;
use std::io::Read;
use std::path::Path;

use yaml_rust::{YamlLoader, Yaml};

use error::YamlError;

pub fn read_yaml_file<P: AsRef<Path>>(yaml_fp: P) -> Result<Yaml, YamlError> {
    // Opens a YAML file on disk and reads the first document.
    let mut f = File::open(yaml_fp)?;

    let mut buffer = String::new();
    f.read_to_string(&mut buffer)?;

    let yaml_docs: Vec<Yaml> = YamlLoader::load_from_str(&buffer)?;

    if yaml_docs.len() < 1 {
        return Err(YamlError::NoDocuments)
    }

    Ok(yaml_docs[0].clone())
}
