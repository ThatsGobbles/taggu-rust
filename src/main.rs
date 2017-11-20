#![feature(conservative_impl_trait)]
#![feature(generators, generator_trait)]

extern crate tempdir;
extern crate regex;

mod library;
mod error;
mod path;
mod generator;

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
