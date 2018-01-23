pub mod yaml;

use std::path::Path;
use std::fs::File;
use std::io::Read;

use metadata::Metadata;
use metadata::target::MetaTarget;
use error::*;

pub trait MetaReader {
    fn from_str<S: AsRef<str>>(s: S, mt: MetaTarget) -> Result<Metadata>;

    fn from_file<P: AsRef<Path>>(p: P, mt: MetaTarget) -> Result<Metadata> {
        let p = p.as_ref();
        let mut f = File::open(p)?;

        let mut buffer = String::new();
        f.read_to_string(&mut buffer)?;

        Self::from_str(buffer, mt).chain_err(|| "umable to parse YAML text")
    }
}
