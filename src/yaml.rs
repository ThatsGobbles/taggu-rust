use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::collections::BTreeMap;

use yaml_rust::{YamlLoader, YamlEmitter, Yaml};

use error::YamlError;

// In metadata, scalar keys and values can be either strings or nulls.
// We can represent this as an Option<String>,
type TagguMetaAtom = Option<String>;

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

// fn create_atom(y: &Yaml) -> Result<Option<String>, YamlError> {
//     if y.is_null() {
//         return Ok(None)
//     }

//     match y.as_str() {
//         Some(s) => Ok(Some(s.to_string())),
//         None => Err(YamlError::InvalidAtom),
//     }
// }

// pub fn construct_self_metadata(y: &Yaml) -> Result<BTreeMap<TagguMetaAtom, Yaml>, YamlError> {
//     let mut map = BTreeMap::new();

//     // We expect a Mapping<Option<String>, V>.
//     let p = y.as_hash().ok_or(YamlError::InvalidMapping)?;

//     for (key, val) in p.iter() {
//         let atom = create_atom(&key)?;
//         map.insert(atom, val.clone());
//     }

//     Ok(map)
// }

pub fn example() {
    let doc = read_yaml_file("/home/lemoine/Music/BASS AVENGERS/taggu_self.yml").unwrap();
    println!("{:?}", doc);
    // construct_self_metadata(&doc);
}

pub fn test() {
    let mut f = File::open("/home/lemoine/Music/BASS AVENGERS/taggu_item.yml").unwrap();

    let mut buffer = String::new();
    f.read_to_string(&mut buffer);

    println!("{}", &buffer);

    let s =
"
foo:
    - list1
    - list2
bar:
    - 1
    - 2.0
";
    let docs = YamlLoader::load_from_str(s).unwrap();

    // Multi document support, doc is a yaml::Yaml
    let doc = &docs[0];

    // Debug support
    println!("{:?}", doc);

    // Index access for map & array
    assert_eq!(doc["foo"][0].as_str().unwrap(), "list1");
    assert_eq!(doc["bar"][1].as_f64().unwrap(), 2.0);

    // Chained key/array access is checked and won't panic,
    // return BadValue if they are not exist.
    assert!(doc["INVALID_KEY"][100].is_badvalue());

    // Dump the YAML object
    let mut out_str = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut out_str);
        emitter.dump(doc).unwrap(); // dump the YAML object to a String
    }
    println!("{}", out_str);
}
