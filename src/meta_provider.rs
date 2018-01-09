use std::path::Path;
use std::fs::File;
use std::io::Read;

use metadata::Metadata;
use error::*;

pub trait MetaProvider {
    fn from_str<S: AsRef<str>>(s: S) -> Result<Metadata>;

    fn from_file<P: AsRef<Path>>(p: P) -> Result<Metadata> {
        let p = p.as_ref();
        let mut f = File::open(p)?;

        let mut buffer = String::new();
        f.read_to_string(&mut buffer)?;

        Self::from_str(buffer)
    }
}
