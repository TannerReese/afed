use std::collections::HashMap;

use super::object::Object;

pub mod num;

pub fn make_bltns() -> HashMap<String, Object> {[
    ("num", num::make_bltns()),
].map(|(key, obj)| (key.to_owned(), obj)).into()}

