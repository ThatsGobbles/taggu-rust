use std::error;
use std::fmt;
use std::path;
use std::io;

#[derive(Debug)]
pub enum MediaLibraryError {
    // NonAbsPath(path::PathBuf),
    NonRelPath(path::PathBuf),
    NotADir(path::PathBuf),
    IoError(io::Error),
    EscapedSubPath(path::PathBuf, path::PathBuf),
}

impl error::Error for MediaLibraryError {
    // This is meant to be a static description of the error, without any dynamic creation.
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
    // This is the place to put dynamically-created error messages.
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
    // This makes it easy to compose other error types into our own error type.
    fn from(err: io::Error) -> MediaLibraryError {
        MediaLibraryError::IoError(err)
    }
}
