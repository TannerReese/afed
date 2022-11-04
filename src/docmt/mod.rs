use std::io::Write;
use std::fmt::{Display, Formatter, Error};
use std::collections::HashMap;

use super::expr::{ExprId, ExprArena};
use super::object::Object;

use parser::{parse, ParseError};

mod parser;


struct Subst {
    start: usize,
    end: usize,

    lineno: usize,
    column: usize,
    target: ExprId,
    value: Option<Object>,
}

pub struct Docmt {
    src: String,
    len: usize,

    arena: ExprArena,
    is_parsed: bool,
    pub only_clear: bool,
    errors: Vec<ParseError>,
    substs: Vec<Subst>,
}

impl Docmt {
    pub fn new(src: String) -> Docmt {
        Docmt {
            len: src.len(), src,
            arena: ExprArena::new(),
            is_parsed: false, only_clear: false,
            errors: Vec::new(),
            substs: Vec::new(),
        }
    }

    pub fn parse<W: Write>(
        &mut self, err_out: &mut W, bltns: HashMap<String, Object>
    ) -> Result<(), usize> {
        if !self.is_parsed {
            let src = std::mem::take(&mut self.src);
            parse(self, &src, bltns);
            self.src = src;
            self.is_parsed = true;
        }

        for err in self.errors.iter() {
            if let Err(_) = write!(err_out, "{}\n", err) {
                panic!("IO Error while writing parse error");
            }
        }

        let count = self.errors.len();
        if count == 0 { Ok(()) } else { Err(count) }
    }

    pub fn eval<W: Write>(
        &mut self, err_out: &mut W
    ) -> Result<(), usize> {
        if self.only_clear { return Ok(()) }

        let mut err_count = 0;
        for Subst {target, value, lineno, column, ..} in self.substs.iter_mut() {
            if value.is_some() { continue; }
            let res = self.arena.eval(*target);
            if res.is_err() {
                if let Err(_) = write!(err_out,
                    "line {}, column {} {}\n",
                    lineno, column, res
                ) { panic!("IO Error while writing eval error"); }
                err_count += 1;
            }
            *value = Some(res);
        }
        if err_count > 0 { Err(err_count) } else { Ok(()) }
    }


    fn add_error(&mut self, err: ParseError) { self.errors.push(err) }

    fn push(&mut self, new: Subst) -> bool {
        if new.start > new.end || new.end > self.len { return false }

        let i = self.substs.iter()
            .enumerate().rev()
            .filter(|(_, sbs)| sbs.start < new.start)
            .next().map_or(0, |(i, _)| i + 1);

        if i > 0 { if let Some(before) = self.substs.get(i - 1) {
            if before.end > new.start { return false }
        }}

        if let Some(after) = self.substs.get(i) {
            if new.end > after.start { return false }
        }

        self.arena.set_saved(new.target);
        self.substs.insert(i, new);
        return true;
    }
}

impl Display for Docmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let mut last = 0;
        for Subst {start, end, value, ..} in self.substs.iter() {
            if last >= self.len { break; }
            f.write_str(&self.src[last..*start])?;

            if self.only_clear {
                last = *end;
                continue;
            }

            if let Some(obj) = value {
                let sub = obj.to_string();
                let sub = sub.chars().flat_map(|c|
                    if c == '`' { vec!['\\', '`'] }
                    else { vec![c] }
                ).collect::<String>();
                write!(f, "{}", sub)?;
            }
            last = *end;
        }

        if last < self.src.len() {
            f.write_str(&self.src[last..])
        } else { Ok(()) }
    }
}



