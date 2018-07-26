#![feature(generators, generator_trait)]
#![feature(type_ascription)]
#![feature(entry_or_default)]

extern crate tempdir;
extern crate regex;
extern crate yaml_rust;
#[macro_use] extern crate maplit;
#[macro_use] extern crate log;
extern crate glob;
#[macro_use] extern crate error_chain;

mod library;
mod helpers;
mod yaml;
mod metadata;
mod plexer;
mod lookup;
mod error;
mod test_helpers;
// mod resolver;
mod generator;

fn main() {
}
