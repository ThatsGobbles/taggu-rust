use std::path::{Path, PathBuf, Component};

use glob::{glob, Pattern};

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
        return PathBuf::from(Component::CurDir.as_ref());
    }

    let mut norm_path = PathBuf::new();

    for item in &stack {
        norm_path.push(item.as_ref());
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

pub fn fuzzy_name_lookup<P: AsRef<Path>, S: AsRef<str>>(dir_path: P, file_name_stub: S) -> Option<String> {
    let file_name_stub = file_name_stub.as_ref();
    let dir_path = normalize(dir_path.as_ref());

    let opt_pattern = {
        dir_path
            .to_str()
            .map(String::from)
            .map(|mut s| { s.push_str(file_name_stub); s })
            .map(|mut s| { s.push('*'); s })
    };

    match opt_pattern {
        Some(pattern) => {
            match glob(&pattern) {
                Ok(globber) => {
                    let mut matching_paths: Vec<PathBuf> = vec![];

                    for entry in globber {
                        match entry {
                            Ok(p) => { matching_paths.push(p) },
                            Err(e) => {
                                error!("Error when globbing paths: {}", e);
                            },
                        }
                    }

                    if matching_paths.len() < 1 {
                        error!("No matching paths found with glob");
                        None
                    } else if matching_paths.len() > 1 {
                        error!("More than one matching path ({}) found with glob", matching_paths.len());
                        None
                    }
                    else {
                        match matching_paths[0].file_name().and_then(|s| s.to_str()) {
                            Some(fn_str) => Some(fn_str.to_string()),
                            None => None,
                        }
                    }
                },
                Err(e) => {
                    error!("Error when constructing glob: {}", e);
                    None
                },
            }
        },
        None => {
            error!("Directory path not convertible to string");
            None
        }
    }
}

pub fn fuzzy_name_match<'a, N, H, J>(needle: N, haystack: H) -> Option<&'a str>
where N: AsRef<str>,
      H: IntoIterator<Item = &'a J>,
      J: AsRef<str> + 'a,
{
    // Create fnmatch-style pattern.
    let mut pattern_str = Pattern::escape(needle.as_ref());
    pattern_str.push('*');

    match Pattern::new(&pattern_str) {
        Ok(pattern) => {
            let matched_strs: Vec<_> = {
                haystack
                .into_iter()
                .map(AsRef::as_ref)
                .filter(|s| pattern.matches(s))
                .collect()
            };

            if matched_strs.len() < 1 {
                warn!("No matches found");
                None
            }
            else if matched_strs.len() > 1 {
                warn!("Multiple matches found");
                None
            }
            else {
                Some(matched_strs[0])
            }
        },
        Err(e) => {
            warn!("Error when constructing pattern: {}", e);
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{normalize, is_valid_item_name, fuzzy_name_match};

    #[test]
    fn test_normalize() {
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

        let expected = Some("TRACK00.flac");
        let produced = fuzzy_name_match("TRACK00", &haystack);
        assert_eq!(expected, produced);

        let expected = Some("TRACK00.flac");
        let produced = fuzzy_name_match("TRACK00.flac", &haystack);
        assert_eq!(expected, produced);

        let expected = Some("TRACK07.flac");
        let produced = fuzzy_name_match("TRACK07", &haystack);
        assert_eq!(expected, produced);

        let expected = Some("TRACK10.flac");
        let produced = fuzzy_name_match("TRACK1", &haystack);
        assert_eq!(expected, produced);

        let expected: Option<&str> = None;
        let produced = fuzzy_name_match("TRACK01", &haystack);
        assert_eq!(expected, produced);

        let expected: Option<&str> = None;
        let produced = fuzzy_name_match("NOTFOUND", &haystack);
        assert_eq!(expected, produced);

        let expected: Option<&str> = None;
        let produced = fuzzy_name_match("TRACK0", &haystack);
        assert_eq!(expected, produced);
    }
}
