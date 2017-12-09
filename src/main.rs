#![feature(conservative_impl_trait)]
#![feature(generators, generator_trait)]
#![feature(type_ascription)]

extern crate tempdir;
extern crate regex;
extern crate yaml_rust;
#[macro_use] extern crate maplit;
#[macro_use] extern crate log;

mod library;
mod error;
mod path;
mod generator;
mod yaml;
mod metadata;
mod plexer;

fn main() {}
