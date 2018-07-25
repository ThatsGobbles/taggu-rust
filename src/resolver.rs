use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::borrow::Cow;

use metadata::{MetaKey, MetaValue, MetaBlock};
use lookup::LookupContext;
use error::*;

const INDEX_SIGIL: char = '#';
const REFERENCE_SIGIL: char = '@';
const PREPEND_SIGIL: char = '&';
const APPEND_SIGIL: char = '+';

pub struct Resolver<'a> {
    lookup_ctx: &'a LookupContext<'a>,
}

impl<'a> Resolver<'a> {
    pub fn new(lookup_ctx: &'a LookupContext<'a>) -> Self {
        Resolver {
            lookup_ctx,
        }
    }

    pub fn resolve<P: AsRef<Path>, S: AsRef<str>>(&mut self, abs_item_path: P, field_name: S) {
        // There are several rules for resolving metadata fields:
        //   1) FIELD_NAME -> FIELD_VALUE:
        //     Sets the field name on this item, overriding any parent values.
        //   2) FIELD_NAME -> ~:
        //     Unsets the field name on this item, discarding any parent values.
        //   3) +FIELD_NAME -> FIELD_VALUE:
        //     Appends the given value to the inherited parent's values for this field name; if there are none, same effect as #1.
        //   4) #FIELD_NAME -> INT_SEQUENCE:
        //     Selects specific items from the inherited parent's values for this field name.
        //   5) @FIELD_NAME -> STR_SEQUENCE:
        //     Selects another field to copy values from; this will look upwards if needed.
    }
}

pub fn ltr_overwrite_vals<'l, 'r>(lv: &'l MetaValue, rv: &'r MetaValue) -> Cow<'r, MetaValue> {
    Cow::Borrowed(rv)
}

// Every Str element can be coerced to a singleton Seq.
// Every Str or Seq element can be coerced to a Map with the element as the root value.

/// Given two meta values, updates them in left-to-right fashion.
/// Values on the right side win out over left side values (matters mostly for mappings).
pub fn map_update_vals<'l, 'r>(lv: &'l MetaValue, rv: &'r MetaValue) -> Cow<'r, MetaValue> {
    match *lv {
        MetaValue::Nil | MetaValue::Str(_) | MetaValue::Seq(_) => {
            // Treat as an overwrite.
            Cow::Borrowed(rv)
        },
        MetaValue::Map(ref l_m) => {
            match *rv {
                MetaValue::Nil | MetaValue::Str(_) | MetaValue::Seq(_) => {
                    let mut new_map = l_m.clone();
                    new_map.insert(MetaKey::Nil, rv.clone());
                    Cow::Owned(MetaValue::Map(new_map))
                },
                MetaValue::Map(ref r_m) => {
                    // CONTINUE HERE!
                    Cow::Owned(MetaValue::Nil)
                },
            }
        }
    }
}
