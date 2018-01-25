use std::path::{Path, PathBuf, Component};
use std::fmt::{Formatter, Result as FmtResult, Display};
use std::error::Error;
use std::ops::Deref;

use glob;

// #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
// pub struct NormedPath {
//     path: PathBuf,
// }

// impl NormedPath {
//     pub fn new<P: AsRef<Path>>(p: P) -> Self {
//         NormedPath {
//             path: normalize(p),
//         }
//     }

//     pub fn path(&self) -> &PathBuf {
//         &self.path
//     }
// }

pub fn normalize<P: AsRef<Path>>(p: P) -> PathBuf {
    let p = p.as_ref();
    let mut stack: Vec<Component> = vec![];

    // We assume .components() removes redundant consecutive path separators.
    // Note that .components() also does some normalization of '.' on its own anyways.
    // This '.' normalization happens to be compatible with the approach below.
    for component in p.components() {
        match component {
            // Drop CurDir components, do not even push onto the stack.
            Component::CurDir => {},

            // For ParentDir components, we need to use the contents of the stack.
            Component::ParentDir => {
                // Look at the top element of stack, if any.
                let top = stack.last().cloned();

                match top {
                    // A component is on the stack, need more pattern matching.
                    Some(c) => {
                        match c {
                            // Push the ParentDir on the stack.
                            Component::Prefix(_) => { stack.push(component); },

                            // The parent of a RootDir is itself, so drop the ParentDir (no-op).
                            Component::RootDir => {},

                            // A CurDir should never be found on the stack, since they are dropped when seen.
                            Component::CurDir => { unreachable!(); },

                            // If a ParentDir is found, it must be due to it piling up at the start of a path.
                            // Push the new ParentDir onto the stack.
                            Component::ParentDir => { stack.push(component); },

                            // If a Normal is found, pop it off.
                            Component::Normal(_) => { let _ = stack.pop(); }
                        }
                    },

                    // Stack is empty, so path is empty, just push.
                    None => { stack.push(component); }
                }
            },

            // All others, simply push onto the stack.
            _ => { stack.push(component); },
        }
    }

    // If an empty PathBuf would be returned, instead return CurDir ('.').
    if stack.is_empty() {
        return PathBuf::from(Component::CurDir.as_os_str());
    }

    let mut norm_path = PathBuf::new();

    for item in &stack {
        norm_path.push(item.as_os_str());
    }

    norm_path
}

pub fn is_valid_item_name<S: AsRef<str>>(file_name: S) -> bool {
    let file_name = file_name.as_ref();
    let normed = normalize(Path::new(file_name));

    // A valid item file name will have the same string repr before and after normalization.
    match normed.to_str() {
        Some(ns) if ns == file_name => {},
        _ => { return false },
    }

    let comps: Vec<_> = normed.components().collect();

    // A valid item file name has only one component, and it must be normal.
    if comps.len() != 1 {
        return false
    }

    match comps[0] {
        Component::Normal(_) => true,
        _ => false
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum FuzzyMatchError {
    InvalidPattern(String),
    ZeroMatches(String),
    MultipleMatches(String, usize),
}

impl Error for FuzzyMatchError {
    fn description(&self) -> &str {
        match *self {
            FuzzyMatchError::InvalidPattern(_) => "Invalid glob pattern",
            FuzzyMatchError::ZeroMatches(_) => "Found zero matches for pattern",
            FuzzyMatchError::MultipleMatches(_, _) => "Found multiple matches for pattern",
        }
    }
}

impl Display for FuzzyMatchError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match *self {
            FuzzyMatchError::InvalidPattern(ref att_pattern) => write!(f, "{}: '{}'", self.description(), att_pattern),
            FuzzyMatchError::ZeroMatches(ref pattern) => write!(f, "{}: '{}'", self.description(), pattern),
            FuzzyMatchError::MultipleMatches(ref pattern, _) => write!(f, "{}: '{}'", self.description(), pattern),
        }
    }
}

pub fn fuzzy_name_match<'a, N, H, J>(needle: N, haystack: H) -> Result<&'a str, FuzzyMatchError>
where N: AsRef<str>,
      H: IntoIterator<Item = &'a J>,
      J: AsRef<str> + 'a,
{
    // Create fnmatch-style pattern.
    let mut pattern_str = needle.as_ref().to_string();
    // let mut pattern_str = glob::Pattern::escape(needle.as_ref());
    pattern_str.push('*');

    match glob::Pattern::new(&pattern_str) {
        Ok(pattern) => {
            let matched_strs: Vec<_> = {
                haystack
                .into_iter()
                .map(AsRef::as_ref)
                .filter(|s| pattern.matches(s))
                .collect()
            };

            if matched_strs.len() < 1 {
                Err(FuzzyMatchError::ZeroMatches(pattern.to_string()))
            }
            else if matched_strs.len() > 1 {
                Err(FuzzyMatchError::MultipleMatches(pattern.to_string(), matched_strs.len()))
            }
            else {
                Ok(matched_strs[0])
            }
        },
        Err(_) => Err(FuzzyMatchError::InvalidPattern(pattern_str.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        normalize,
        is_valid_item_name,
        fuzzy_name_match,
        FuzzyMatchError,
        // NormedPath,
    };

    // #[test]
    // fn test_normed_path() {
    //     let p_and_e_inputs = vec![
    //         (".", "."),
    //         ("a/b/c/./d/e/../f/", "a/b/c/d/f"),
    //     ];

    //     for (p_input, e_input) in p_and_e_inputs {
    //         let expected = PathBuf::from(e_input);
    //         let produced = NormedPath::new(p_input);
    //         assert_eq!(&expected, produced.path());
    //     }
    // }

    #[test]
    fn test_normalize() {
        macro_rules! tn(
            ($path:expr, $expected:expr) => ( {
                let actual = normalize(PathBuf::from($path));
                assert!(actual.to_str() == Some($expected),
                        "normalizing {:?}: Expected {:?}, got {:?}",
                        $path, $expected,
                        actual.to_str().unwrap());
            });
        );

        if cfg!(unix) {
            tn!("", ".");
            tn!("/", "/");
            tn!("//", "/");  /* Double-slash root is a separate entity in POSIX,
                            but in Rust we treat it as a normal root slash. */
            tn!("foo", "foo");
            tn!(".", ".");
            tn!("..", "..");
            tn!(".foo", ".foo");
            tn!("..foo", "..foo");
            tn!("/foo", "/foo");
            tn!("//foo", "/foo");
            tn!("./foo/", "foo");
            tn!("../foo/", "../foo");
            tn!("/foo/bar", "/foo/bar");
            tn!("foo/bar", "foo/bar");
            tn!("foo/.", "foo");
            tn!("foo//bar", "foo/bar");
            tn!("./foo//bar//", "foo/bar");

            tn!("foo/bar/baz/..", "foo/bar");
            tn!("foo/bar/baz/../", "foo/bar");
            tn!("foo/bar/baz/../..", "foo");
            tn!("foo/bar/baz/../../..", ".");
            tn!("foo/bar/baz/../../../..", "..");
            tn!("foo/bar/baz/../../../../..", "../..");
            tn!("/foo/bar/baz/../../../../..", "/");
            tn!("foo/../bar/../baz/../", ".");
            tn!("/.", "/");
            tn!("/..", "/");
            tn!("/../../", "/");
        } else {
            // Drive-absolute paths.
            tn!(r#"X:\ABC\DEF"#, r#"X:\ABC\DEF"#);
            tn!(r#"X:\"#, r#"X:\"#);
            tn!(r#"X:\ABC\"#, r#"X:\ABC"#);
            tn!(r#"X:\ABC\DEF. ."#, r#"X:\ABC\DEF"#);
            tn!(r#"X:/ABC/DEF"#, r#"X:\ABC\DEF"#);
            tn!(r#"X:\ABC\..\XYZ"#, r#"X:\XYZ"#);
            tn!(r#"X:\ABC\..\..\.."#, r#"X:\"#);

            // Drive-relative paths.
            tn!(r#"X:DEF\GHI"#, r#"X:DEF\GHI"#);
            tn!(r#"X:"#, r#"X:"#);
            tn!(r#"X:DEF. ."#, r#"X:DEF"#);
            tn!(r#"Y:"#, r#"Y:"#);
            tn!(r#"Z:"#, r#"Z:"#);
            tn!(r#"X:ABC\..\XYZ"#, r#"X:XYZ"#);
            tn!(r#"X:ABC\..\..\.."#, r#"X:"#);

            // Rooted paths.
            tn!(r#"\ABC\DEF"#, r#"\ABC\DEF"#);
            tn!(r#"\"#, r#"\"#);
            tn!(r#"\ABC\DEF. ."#, r#"\ABC\DEF"#);
            tn!(r#"/ABC/DEF"#, r#"\ABC\DEF"#);
            tn!(r#"\ABC\..\XYZ"#, r#"\XYZ"#);
            tn!(r#"\ABC\..\..\.."#, r#"\"#);

            // Relative paths.
            tn!(r#"ABC\DEF"#, r#"ABC\DEF"#);
            tn!(r#"."#, r#"."#);
            tn!(r#"ABC\DEF. ."#, r#"ABC\DEF"#);
            tn!(r#"ABC/DEF"#, r#"ABC\DEF"#);
            tn!(r#"..\ABC"#, r#"..\ABC"#);
            tn!(r#"ABC\..\..\.."#, r#"..\.."#);

            // UNC absolute paths.
            tn!(r#"\\server\share\ABC\DEF"#, r#"\\server\share\ABC\DEF"#);
            tn!(r#"\\server"#, r#"\\server"#);
            tn!(r#"\\server\share"#, r#"\\server\share"#);
            tn!(r#"\\server\share\ABC. ."#, r#"\\server\share\ABC"#);
            tn!(r#"//server/share/ABC/DEF"#, r#"\\server\share\ABC\DEF"#);
            tn!(r#"\\server\share\ABC\..\XYZ"#, r#"\\server\share\XYZ"#);
            tn!(r#"\\server\share\ABC\..\..\.."#, r#"\\server\share"#);

            // Local device paths.
            tn!(r#"\\.\COM20"#, r#"\\.\COM20"#);
            tn!(r#"\\.\pipe\mypipe"#, r#"\\.\pipe\mypipe"#);
            tn!(r#"\\.\X:\ABC\DEF. ."#, r#"\\.\X:\ABC\DEF"#);
            tn!(r#"\\.\X:/ABC/DEF"#, r#"\\.\X:\ABC\DEF"#);
            tn!(r#"\\.\X:\ABC\..\XYZ"#, r#"\\.\X:\XYZ"#);
            tn!(r#"\\.\X:\ABC\..\..\C:\"#, r#"\\.\C:\"#);
            tn!(r#"\\.\pipe\mypipe\..\notmine"#, r#"\\.\pipe\notmine"#);

            // More local device paths.
            tn!(r#"COM1"#, r#"\\.\COM1"#);
            tn!(r#"X:\COM1"#, r#"\\.\COM1"#);
            tn!(r#"X:COM1"#, r#"\\.\COM1"#);
            tn!(r#"valid\COM1"#, r#"\\.\COM1"#);
            tn!(r#"X:\notvalid\COM1"#, r#"\\.\COM1"#);
            tn!(r#"X:\COM1.blah"#, r#"\\.\COM1"#);
            tn!(r#"X:\COM1:blah"#, r#"\\.\COM1"#);
            tn!(r#"X:\COM1  .blah"#, r#"\\.\COM1"#);
            tn!(r#"\\.\X:\COM1"#, r#"\\.\X:\COM1"#);
            tn!(r#"\\abc\xyz\COM1"#, r#"\\abc\xyz\COM1"#);

            // Root local device paths.
            tn!(r#"\\?\X:\ABC\DEF"#, r#"\\?\X:\ABC\DEF"#);
            tn!(r#"\\?\X:\"#, r#"\\?\X:\"#);
            tn!(r#"\\?\X:"#, r#"\\?\X:"#);
            tn!(r#"\\?\X:\COM1"#, r#"\\?\X:\COM1"#);
            tn!(r#"\\?\X:\ABC\DEF. ."#, r#"\\?\X:\ABC\DEF"#);
            tn!(r#"\\?\X:/ABC/DEF"#, r#"\\?\X:\ABC\DEF"#);
            tn!(r#"\\?\X:\ABC\..\XYZ"#, r#"\\?\X:\XYZ"#);
            tn!(r#"\\?\X:\ABC\..\..\.."#, r#"\\?\"#);

            // More root local device paths.
            tn!(r#"\??\X:\ABC\DEF"#, r#"X:\??\X:\ABC\DEF"#);
            tn!(r#"\??\X:\"#, r#"X:\??\X:\"#);
            tn!(r#"\??\X:"#, r#"X:\??\X:"#);
            tn!(r#"\??\X:\COM1"#, r#"X:\??\X:\COM1"#);
            tn!(r#"\??\X:\ABC\DEF. ."#, r#"X:\??\X:\ABC\DEF"#);
            tn!(r#"\??\X:/ABC/DEF"#, r#"X:\??\X:\ABC\DEF"#);
            tn!(r#"\??\X:\ABC\..\XYZ"#, r#"X:\??\X:\XYZ"#);
            tn!(r#"\??\X:\ABC\..\..\.."#, r#"X:\"#);






            tn!(r#"a\b\c"#, r#"a\b\c"#);
            tn!(r#"a/b\c"#, r#"a\b\c"#);
            tn!(r#"a/b\c\"#, r#"a\b\c"#);
            tn!(r#"a/b\c/"#, r#"a\b\c"#);
            tn!(r#"\"#, r#"\"#);
            tn!(r#"\\"#, r#"\"#);
            tn!(r#"/"#, r#"\"#);
            tn!(r#"//"#, r#"\"#);

            tn!(r#"C:\a\b"#, r#"C:\a\b"#);
            tn!(r#"C:\"#, r#"C:\"#);
            tn!(r#"C:\."#, r#"C:\"#);
            tn!(r#"C:\.."#, r#"C:\"#);
            tn!(r#"C:a"#, r#"C:a"#);
            tn!(r#"C:."#, r#"C:."#);
            tn!(r#"C:.."#, r#"C:.."#);

            // Should these not have a trailing slash?
            tn!(r#"\\server\share"#, r#"\\server\share\"#);
            tn!(r#"\\server\share\a\b"#, r#"\\server\share\a\b"#);
            tn!(r#"\\server\share\a\.\b"#, r#"\\server\share\a\b"#);
            tn!(r#"\\server\share\a\..\b"#, r#"\\server\share\b"#);
            tn!(r#"\\server\share\a\b\"#, r#"\\server\share\a\b"#);

            tn!(r#"\\?\a\b"#, r#"\\?\a\b"#);
            tn!(r#"\\?\a/\\b\"#, r#"\\?\a/\\b"#);
            tn!(r#"\\?\a/\\b/"#, r#"\\?\a/\\b/"#);
            tn!(r#"\\?\a\b"#, r#"\\?\a\b"#);
        }

        assert_eq!(normalize(Path::new("")), PathBuf::from("."));
        assert_eq!(normalize(Path::new("/")), PathBuf::from("/"));
        assert_eq!(normalize(Path::new("//")), PathBuf::from("/"));
        assert_eq!(normalize(Path::new("///")), PathBuf::from("/"));
        assert_eq!(normalize(Path::new(".")), PathBuf::from("."));
        assert_eq!(normalize(Path::new("..")), PathBuf::from(".."));
        assert_eq!(normalize(Path::new("./")), PathBuf::from("."));
        assert_eq!(normalize(Path::new("../")), PathBuf::from(".."));
        assert_eq!(normalize(Path::new("/.")), PathBuf::from("/"));
        assert_eq!(normalize(Path::new("/..")), PathBuf::from("/"));
        assert_eq!(normalize(Path::new("./foo")), PathBuf::from("foo"));
        assert_eq!(normalize(Path::new("foo")), PathBuf::from("foo"));
        assert_eq!(normalize(Path::new(".foo")), PathBuf::from(".foo"));
        assert_eq!(normalize(Path::new("foo.")), PathBuf::from("foo."));
        assert_eq!(normalize(Path::new("foo/bar/")), PathBuf::from("foo/bar"));
        assert_eq!(normalize(Path::new("foo//bar///")), PathBuf::from("foo/bar"));
        assert_eq!(normalize(Path::new("foo/bar/./baz/")), PathBuf::from("foo/bar/baz"));
        assert_eq!(normalize(Path::new("foo/bar/../baz/")), PathBuf::from("foo/baz"));
        assert_eq!(normalize(Path::new("../foo")), PathBuf::from("../foo"));
    }

    #[test]
    fn test_is_valid_item_name() {
        let inputs_and_expected = vec![
            ("simple", true),
            ("simple.ext", true),
            ("spaces ok", true),
            ("questions?", true),
            ("exclamation!", true),
            ("period.", true),
            (".period", true),
            ("", false),
            (".", false),
            ("..", false),
            ("/simple", false),
            ("./simple", false),
            ("simple/", false),
            ("simple/.", false),
            ("/", false),
            ("/simple/more", false),
            ("simple/more", false),
        ];

        for (input, expected) in inputs_and_expected {
            let produced = is_valid_item_name(input);
            assert_eq!(expected, produced);
        }
    }

    #[test]
    fn test_fuzzy_name_match() {
        let haystack = [
            "TRACK00.flac",
            "TRACK01.flac",
            "TRACK01.flac",
            "TRACK02.flac",
            "TRACK03.flac",
            "TRACK04.flac",
            "TRACK05.flac",
            "TRACK06.flac",
            "TRACK07.flac",
            "TRACK08.flac",
            "TRACK09.flac",
            "TRACK10.flac",
        ];

        let inputs_and_expected = vec![
            ("TRACK00", Ok("TRACK00.flac"): Result<&str, FuzzyMatchError>),
            ("TRACK00.flac", Ok("TRACK00.flac")),
            ("TRACK07", Ok("TRACK07.flac")),
            ("TRACK1", Ok("TRACK10.flac")),
            ("TRACK01", Err(FuzzyMatchError::MultipleMatches(String::from("TRACK01*"), 2))),
            ("NOTFOUND", Err(FuzzyMatchError::ZeroMatches(String::from("NOTFOUND*")))),
            ("TRACK0", Err(FuzzyMatchError::MultipleMatches(String::from("TRACK0*"), 11))),
            ("****", Err(FuzzyMatchError::InvalidPattern(String::from("*****")))),
        ];

        for (input, expected) in inputs_and_expected {
            let produced = fuzzy_name_match(input, &haystack);
            assert_eq!(expected, produced);
        }
    }
}
