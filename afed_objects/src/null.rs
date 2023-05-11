// Copyright (C) 2022-2023 Tanner Reese
/* This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::fmt::{Display, Error, Formatter};
use std::vec::Vec;

use super::Object;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Null();
name_type! {null: Null}
impl_operable! {Null:
    //! Null value. Exists for compatability with JSON
}

impl Display for Null {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "null")
    }
}

impl From<Null> for Object {
    fn from(n: Null) -> Self {
        Object::new(n)
    }
}
