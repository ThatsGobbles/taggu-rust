extern crate tempdir;

mod library;
mod error;
mod path;

fn main() {
    library::example();

    // let mut paths = vec![];

    // paths.push(Path::new("../../home/thatsgobbles/././music/../code/.."));
    // paths.push(Path::new("/home//thatsgobbles/music/"));
    // paths.push(Path::new("/../../home/thatsgobbles/././code/../music/.."));
    // paths.push(Path::new(".."));
    // paths.push(Path::new("/.."));
    // paths.push(Path::new("../"));
    // paths.push(Path::new("/"));
    // paths.push(Path::new(""));
    // // More tests for Windows (especially with drive letters and UNC paths) needed.

    // for p in &paths {
    //     let np = normalize(&p);
    //     println!("{:?} ==> {:?}", &p, &np);
    // }
}
