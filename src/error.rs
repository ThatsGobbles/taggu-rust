use std::path::PathBuf;

error_chain!{
    errors {
        NotADirectory(p: PathBuf) {
            description("not a directory"),
            display("not a directory: '{}'", p.to_string_lossy()),
        }
        NotAFile(p: PathBuf) {
            description("not a file"),
            display("not a file: '{}'", p.to_string_lossy()),
        }
        DoesNotExist(p: PathBuf) {
            description("path does not exist"),
            display("path does not exist: '{}'", p.to_string_lossy()),
        }
        InvalidSubPath(p: PathBuf, root: PathBuf) {
            description("subpath is not a descendant of root"),
            display("subpath is not a descendant of root: '{}', '{}'", p.to_string_lossy(), root.to_string_lossy()),
        }
        InvalidMetaFileName(s: String) {
            description("meta file name is invalid"),
            display("meta file name is invalid: '{}'", s),
        }
        EmptyMetaFile(p: PathBuf) {
            description("meta file did not contain any data")
            display("meta file did not contain any data: '{}'", p.to_string_lossy())
        }
    }

    foreign_links {
        Io(::std::io::Error);
        Yaml(::yaml_rust::scanner::ScanError);
    }
}
