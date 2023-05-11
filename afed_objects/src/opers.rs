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

use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unary {
    Neg,
    Not,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binary {
    Apply,
    And,
    Or,
    Eq,
    Neq,
    Lt,
    Leq,
    Gt,
    Geq,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    FlrDiv,
    Pow,
}

type Prec = usize;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assoc {
    Left,
    Right,
}

impl Unary {
    pub fn prec(&self) -> Prec {
        match self {
            Unary::Not => 90,
            Unary::Neg => 90,
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            Unary::Not => "!",
            Unary::Neg => "-",
        }
    }

    pub const OPERS: [Self; 2] = [Unary::Not, Unary::Neg];
}

impl Binary {
    pub fn prec(&self) -> Prec {
        match self {
            Binary::Apply => 10,
            Binary::Or => 35,
            Binary::And => 36,
            Binary::Eq | Binary::Neq => 40,
            Binary::Lt | Binary::Leq | Binary::Gt | Binary::Geq => 40,
            Binary::Add | Binary::Sub => 50,
            Binary::Mul | Binary::Div | Binary::Mod | Binary::FlrDiv => 75,
            Binary::Pow => 100,
        }
    }

    pub fn assoc(&self) -> Assoc {
        match self {
            Binary::Apply => Assoc::Right,
            Binary::Or | Binary::And => Assoc::Left,
            Binary::Eq | Binary::Neq => Assoc::Left,
            Binary::Lt | Binary::Leq | Binary::Gt | Binary::Geq => Assoc::Left,
            Binary::Add | Binary::Sub => Assoc::Left,
            Binary::Mul | Binary::Div | Binary::Mod | Binary::FlrDiv => Assoc::Left,
            Binary::Pow => Assoc::Right,
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            Binary::Apply => "$",
            Binary::Or => "||",
            Binary::And => "&&",
            Binary::Eq => "==",
            Binary::Neq => "!=",
            Binary::Lt => "<",
            Binary::Leq => "<=",
            Binary::Gt => ">",
            Binary::Geq => ">=",
            Binary::Add => "+",
            Binary::Sub => "-",
            Binary::Mul => "*",
            Binary::Div => "/",
            Binary::Mod => "%",
            Binary::FlrDiv => "//",
            Binary::Pow => "^",
        }
    }

    pub const OPERS: [Self; 16] = [
        Binary::Apply,
        Binary::Or,
        Binary::And,
        Binary::Eq,
        Binary::Neq,
        Binary::Lt,
        Binary::Leq,
        Binary::Gt,
        Binary::Geq,
        Binary::Add,
        Binary::Sub,
        Binary::Mul,
        Binary::Div,
        Binary::Mod,
        Binary::FlrDiv,
        Binary::Pow,
    ];
}

impl FromStr for Unary {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Unary::OPERS
            .iter()
            .copied()
            .filter(|op| s.starts_with(op.symbol()))
            .max_by_key(|op| op.symbol().len())
            .ok_or(())
    }
}

impl FromStr for Binary {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Binary::OPERS
            .iter()
            .copied()
            .filter(|op| s.starts_with(op.symbol()))
            .max_by_key(|op| op.symbol().len())
            .ok_or(())
    }
}
