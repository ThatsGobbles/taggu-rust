use std::error;
use std::error::Error;
use std::fmt;
use std::path;
use std::io;

use yaml_rust::scanner;

#[derive(Debug)]
pub enum MediaLibraryError {
    // NonAbsPath(path::PathBuf),
    NonRelPath(path::PathBuf),
    NotADir(path::PathBuf),
    IoError(io::Error),
    EscapedSubPath(path::PathBuf, path::PathBuf),
}

impl error::Error for MediaLibraryError {
    // LEARN: This is meant to be a static description of the error, without any dynamic creation.
    fn description(&self) -> &str {
        match self {
            // &MediaLibraryError::NonAbsPath(_) => "File path was expected to be absolute",
            &MediaLibraryError::NonRelPath(_) => "File path was expected to be relative",
            &MediaLibraryError::NotADir(_) => "File path did not point to an existing directory",
            &MediaLibraryError::IoError(ref e) => e.description(),
            &MediaLibraryError::EscapedSubPath(_, _) => "Sub path was not a descendant of root directory",
        }
    }
}

impl fmt::Display for MediaLibraryError {
    // LEARN: This is the place to put dynamically-created error messages.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // &MediaLibraryError::NonAbsPath(ref p) => write!(f, r##"Path {:?} is not absolute"##, p),
            &MediaLibraryError::NonRelPath(ref p) => write!(f, r##"Path {:?} is not relative"##, p),
            &MediaLibraryError::NotADir(ref p) => write!(f, r##"Path {:?} is not an existing directory"##, p),
            &MediaLibraryError::IoError(ref e) => e.fmt(f),
            &MediaLibraryError::EscapedSubPath(ref p, ref r) => write!(f, r##"Sub path {:?} is not a descendant of root directory {:?}"##, p, r),
        }
    }
}

impl From<io::Error> for MediaLibraryError {
    // LEARN: This makes it easy to compose other error types into our own error type.
    fn from(err: io::Error) -> MediaLibraryError {
        MediaLibraryError::IoError(err)
    }
}

#[derive(Debug)]
pub enum YamlError {
    IoError(io::Error),
    YamlScanError(scanner::ScanError),
    NoDocuments,
    ExtraDocuments,
    InvalidMapping,
    InvalidAtom,
}

impl error::Error for YamlError {
    fn description(&self) -> &str {
        match self {
            &YamlError::IoError(ref e) => e.description(),
            &YamlError::YamlScanError(ref y) => y.description(),
            &YamlError::NoDocuments => "No documents found in YAML file",
            &YamlError::ExtraDocuments => "More than one document found in YAML file",
            &YamlError::InvalidMapping => "Expected a YAML mapping with strings/nulls as keys",
            &YamlError::InvalidAtom => "Expected either a string or a null",
        }
    }
}

impl fmt::Display for YamlError {
    // This is the place to put dynamically-created error messages.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &YamlError::IoError(ref e) => e.fmt(f),
            &YamlError::YamlScanError(ref y) => y.fmt(f),
            &YamlError::NoDocuments => self.description().fmt(f),
            &YamlError::ExtraDocuments => self.description().fmt(f),
            &YamlError::InvalidMapping => self.description().fmt(f),
            &YamlError::InvalidAtom => self.description().fmt(f),
        }
    }
}

impl From<io::Error> for YamlError {
    fn from(err: io::Error) -> YamlError {
        YamlError::IoError(err)
    }
}

impl From<scanner::ScanError> for YamlError {
    fn from(err: scanner::ScanError) -> YamlError {
        YamlError::YamlScanError(err)
    }
}
