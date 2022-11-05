use std::io::Write;
use std::fmt::{Display, Formatter, Error};
use std::collections::HashMap;
use std::path::PathBuf;
use std::ffi::OsString;

use super::expr::{ExprId, ExprArena};
use super::object::Object;

use parser::{parse, ParseError};

mod parser;


struct Subst {
    start: usize,
    end: usize,
    filename: Option<OsString>,

    lineno: usize,
    column: usize,
    target: ExprId,
    value: Option<Object>,
}

pub struct Docmt {
    src: String,
    len: usize,
    pub paths: Vec<PathBuf>,

    arena: ExprArena,
    is_parsed: bool,
    pub only_clear: bool,
    errors: Vec<ParseError>,
    substs: Vec<Subst>,
    pub ignore_substs: bool,
}

impl Docmt {
    pub fn new(src: String, path: Option<PathBuf>) -> Docmt {
        let paths = path.and_then(|p|
            p.canonicalize().ok()
        ).into_iter().collect();

        Docmt {
            len: src.len(), src, paths,
            arena: ExprArena::new(),
            is_parsed: false, only_clear: false,
            errors: Vec::new(),
            substs: Vec::new(),
            ignore_substs: false,
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
            write!(err_out, "{}\n", err)
            .expect("IO Error while writing parse error");
        }

        let count = self.errors.len();
        if count == 0 { Ok(()) } else { Err(count) }
    }

    pub fn eval<W: Write>(
        &mut self, err_out: &mut W
    ) -> Result<(), usize> {
        if self.only_clear { return Ok(()) }

        let mut err_count = 0;
        for Subst {
            filename, target, value, lineno, column, ..
        } in self.substs.iter_mut() {
            if value.is_some() { continue; }
            let res = self.arena.eval(*target);
            if res.is_err() {
                write!(err_out, "line {}, column {}", lineno, column)
                .and_then(|_| if let Some(name) = filename {
                    write!(err_out, " of {:?}", name)
                } else { Ok(()) })
                .and_then(|_| write!(err_out, " {}\n", res))
                .expect("IO Error while writing eval error");
                err_count += 1;
            }
            *value = Some(res);
        }
        if err_count > 0 { Err(err_count) } else { Ok(()) }
    }


    fn add_error(&mut self, mut err: ParseError) {
        if let Some(name) = self.paths.last()
        .and_then(|file| file.file_name()) {
            err.set_filename(name.to_owned());
        }
        self.errors.push(err);
    }

    fn push(&mut self, mut new: Subst) -> bool {
        if self.ignore_substs { return false }
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
        if let Some(name) = self.paths.last()
        .and_then(|file| file.file_name()) {
            new.filename = Some(name.to_owned());
        }
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

