#![feature(conservative_impl_trait)]
#![feature(generators, generator_trait)]
#![feature(type_ascription)]

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
mod generator;
mod meta_provider;

fn main() {
}
