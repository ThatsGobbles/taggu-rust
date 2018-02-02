use std::path::{Path, PathBuf};
use std::collections::HashMap;

use metadata::{MetaValue, MetaBlock};
use lookup::LookupContext;
use error::*;

pub struct Resolver<'a> {
    lookup_ctx: &'a LookupContext<'a>,
}

impl<'a> Resolver<'a> {
    pub fn new(lookup_ctx: &'a LookupContext<'a>) -> Self {
        Resolver {
            lookup_ctx,
        }
    }
}
