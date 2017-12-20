#![feature(conservative_impl_trait)]
#![feature(generators, generator_trait)]
#![feature(type_ascription)]

extern crate tempdir;
extern crate regex;
extern crate yaml_rust;
#[macro_use] extern crate maplit;
#[macro_use] extern crate log;
extern crate glob;

mod library;
mod error;
mod helpers;
mod generator;
mod yaml;
mod metadata;
mod plexer;
mod query;

fn main() {
}
