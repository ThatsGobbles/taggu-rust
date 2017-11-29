#![feature(conservative_impl_trait)]
#![feature(generators, generator_trait)]

extern crate tempdir;
extern crate regex;
extern crate yaml_rust;
#[macro_use] extern crate maplit;

mod library;
mod error;
mod path;
mod generator;
mod yaml;
mod metadata;
mod plexer;
mod discovery;

fn main() {
    library::example();
    // metadata::example();
}
