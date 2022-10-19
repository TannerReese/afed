use std::io::Write;
use std::fmt::{Display, Formatter, Error};
use std::collections::HashMap;

use super::object::Object;
use super::object::opers;
use super::object::null::Null;
use super::object::bool::{Bool, Ternary};
use super::object::number::Number;
use super::object::string::Str;

use super::expr::{ExprId, ExprArena};

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
    err_count: usize,
    substs: Vec<Subst>,
}

impl Docmt {
    pub fn new(src: String) -> Docmt {
        Docmt {
            len: src.len(), src,
            arena: ExprArena::new(),
            is_parsed: false, err_count: 0,
            substs: Vec::new()
        }
    }

    pub fn parse<W>(&mut self, err_out: &mut W, bltns: HashMap<String, Object>) -> Result<(), usize>
    where W: Write {
        if !self.is_parsed {
            let src: String = std::mem::take(&mut self.src);

            let mut prs = Parser {doc: self, pos: Pos::new(&src), err_out, err_count: 0};
            let root = prs.parse_map(true).map_err(|err| prs.print_err(err)).ok();
            if !prs.pos.is_empty() {
                prs.print_err(prs.error("Extra unparsed content in document"))
            }
            self.err_count = prs.err_count;

            if let Some(root) = root {
                self.arena.resolve_builtins(root, bltns);
            }

            self.src = src;
            self.is_parsed = true;
        }
        if self.err_count > 0 { Err(self.err_count) } else { Ok(()) }
    }

    pub fn eval<W>(&mut self, err_out: &mut W) -> Result<(), usize> where W: Write {
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
            if let Some(obj) = value { write!(f, "{}", obj)?; }
            last = *end;
        }

        if last < self.src.len() {
            f.write_str(&self.src[last..])
        } else { Ok(()) }
    }
}



#[derive(Debug, Clone, Copy)]
struct Pos<'a> {
    ptr: &'a str,
    idx: usize,
    line_begin: &'a str,
    lineno: usize,
    column: usize,
}

impl<'a> Pos<'a> {
    fn new(s: &'a str) -> Pos<'a> { Pos {
        ptr: s, idx: 0, line_begin: s, lineno: 1, column: 1
    }}

    fn is_empty(&self) -> bool { self.ptr.len() == 0 }
    fn peek(&self) -> Option<char> { self.ptr.chars().next() }

    fn next(&mut self) -> Option<char> {
        let mut chs = self.ptr.chars();
        if let Some(c) = chs.next() {
            self.ptr = chs.as_str();
            self.column += 1;
            self.idx += 1;

            if c == '\n' {
                self.line_begin = self.ptr;
                self.lineno += 1;
                self.column = 1;
            }
            Some(c)
        } else { None }
    }

    fn shift(&mut self, mut offset: usize){
        offset = std::cmp::min(offset, self.ptr.len());
        for _ in 0..offset { self.next(); }
    }

    fn skip_while<P>(&mut self, mut predicate: P) -> usize
    where P: FnMut(char) -> bool {
        let mut count = 0;
        while self.peek().map_or(false, &mut predicate) {
            self.next();
            count += 1;
        }
        return count;
    }

    fn skip_until(&mut self, pat: &str){
        while self.ptr.len() > 0 && !self.ptr.starts_with(pat) {
            self.next();
        }
    }
}



#[derive(Debug, Clone)]
struct ParseError {
    msg: String,
    line: String,
    lineno: usize,
    column: usize,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Parse Error in line {}, column {}\n", self.lineno, self.column)?;
        write!(f, "\t{}\n\t{:2$}^\n", self.line, "", self.column - 1)?;
        write!(f, "Error: {}\n", self.msg)
    }
}



struct Parser<'a, 'b, W> {
    doc: &'a mut Docmt,
    pos: Pos<'a>,
    err_out: &'b mut W,
    err_count: usize,
}

impl<'a, 'b, W> Parser<'a, 'b, W> where W: Write {
    fn skip(&mut self){
        loop {
            self.pos.skip_while(|c| c.is_whitespace());
            if self.pos.peek() == Some('#') {
                self.pos.next();
                if self.pos.peek() == Some('{') {
                    self.pos.skip_until("}#");
                    self.pos.next();
                    self.pos.next();
                } else {
                    self.pos.skip_while(|c| c != '\n');
                }
            } else { break; }
        }
    }

    fn skip_to_comma(&mut self){
        let mut braces = Vec::<char>::new();
        while let Some(c) = self.pos.peek() {
            if c == '(' || c == '[' || c == '{' {
                braces.push(c)
            } else if let Some(c) = match c {
                    ')' => Some('('),
                    ']' => Some('['),
                    '}' => Some('{'),
                    _ => None,
            } {
                while let Some(&last) = braces.last() {
                    if last == c { break; }
                    braces.pop();
                }
                if let None = braces.pop() { return }
            } else if c == ',' { return }
            self.pos.next();
        }
    }

    fn expect(&mut self, c: char, err_msg: &str) -> Result<(), ParseError> {
        self.skip();
        if self.pos.peek() != Some(c) {
            Err(self.error(err_msg))
        } else {
            self.pos.next();
            Ok(())
        }
    }

    fn error(&self, msg: &str) -> ParseError {
        ParseError {
            msg: msg.to_owned(),
            line: self.pos.line_begin
                .lines().next()
                .unwrap_or("").to_owned(),
            lineno: self.pos.lineno,
            column: self.pos.column
        }
    }

    fn print_err(&mut self, err: ParseError){
        if let Err(_) = write!(self.err_out, "{}\n", err) {
            panic!("IO Error while writing parse error");
        }
        self.err_count += 1;
    }

    fn parse_char(&mut self, c: char) -> bool {
        self.skip();
        if self.pos.peek() == Some(c) {
            self.pos.next();
            true
        } else { false }
    }

    fn parse_unary(&mut self) -> Option<opers::Unary> {
        self.skip();
        if let Ok(op) = self.pos.ptr.parse::<opers::Unary>() {
            self.pos.shift(op.symbol().len());
            Some(op)
        } else { None }
    }

    fn parse_binary(&mut self) -> Option<opers::Binary> {
        self.skip();
        if let Ok(op) = self.pos.ptr.parse::<opers::Binary>() {
            self.pos.shift(op.symbol().len());
            Some(op)
        } else { None }
    }

    fn parse_equals(&mut self) -> Result<ExprId, ParseError> {
        let body = self.parse_expr(0)?;
        self.skip();
        if self.parse_char('=') {
            self.skip();
            let (lineno, column) = (self.pos.lineno, self.pos.column);
            self.expect('`', "Missing opening grave in substitution expression")?;

            let start = self.pos.idx;
            self.pos.skip_while(|c| c != '`');
            let end = self.pos.idx;

            self.expect('`', "Missing closing grave in substition expression")?;
            self.doc.push(Subst {
                start, end,
                lineno, column,
                target: body, value: None,
            });
        }
        Ok(body)
    }

    fn parse_expr(&mut self, min_prec: usize) -> Result<ExprId, ParseError> {
        let mut value = if let Some(op) = self.parse_unary() {
            let prec = std::cmp::max(op.prec(), min_prec);
            let arg = self.parse_expr(prec + 1)?;
            self.doc.arena.create_unary(op, arg).unwrap()
        } else { self.parse_call()? };

        let mut before_oper = self.pos;
        while let Some(op) = self.parse_binary() {
            let mut prec = op.prec();
            if prec < min_prec { break }
            if op.assoc() == opers::Assoc::Left { prec += 1; }

            let arg = self.parse_expr(prec)?;
            self.skip();
            value = self.doc.arena.create_binary(op, value, arg).unwrap();
            before_oper = self.pos;
        }
        self.pos = before_oper;
        return Ok(value);
    }



    fn parse_num(&mut self) -> Result<ExprId, ParseError> {
        let numstr = self.pos.ptr;
        let len = self.pos.skip_while(|c| c.is_ascii_digit() || c == '.');
        let numstr = &numstr[..len];

        let num = if let Ok(i) = numstr.parse::<i64>() {
            Number::Ratio(i, 1)
        } else if let Ok(f) = numstr.parse::<f64>() {
            Number::Real(f)
        } else { return Err(self.error("Invalid number")); };
        Ok(self.doc.arena.create_obj(num))
    }

    fn parse_name(&mut self) -> Result<String, ParseError> {
        self.skip();
        if !self.pos.peek().map_or(false, |c| c.is_alphabetic()) {
            return Err(self.error("Name must begin with a alphabetic character"));
        }

        let name = self.pos.ptr;
        let len = self.pos.skip_while(|c|
            c.is_alphanumeric() || c == '_' || c == '.'
        );
        Ok(name[..len].to_owned())
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        self.expect('"', "Missing opening quote in string")?;

        let mut s = String::new();
        while let Some(mut c) = self.pos.peek() {
            if c == '"' { break; }
            else if c == '\\' {
                self.pos.next();
                c = match self.pos.peek() {
                    None => break,
                    Some('a') => '\x07',
                    Some('b') => '\x08',
                    Some('e') => '\x1b',
                    Some('f') => '\x0c',
                    Some('n') => '\n',
                    Some('r') => '\r',
                    Some('t') => '\t',
                    Some('v') => '\x0b',
                    Some(c) => c,
                };
            }
            s.push(c);
            self.pos.next();
        }

        self.expect('"', "Missing closing quote in string")?;
        Ok(s)
    }

    fn parse_array(&mut self) -> Result<ExprId, ParseError> {
        self.expect('[', "Missing opening bracket in array")?;

        let mut elems = Vec::new();
        loop {
            elems.push(self.parse_equals()?);
            if !self.parse_char(',') { break; }
        }

        self.expect(']', "Missing closing bracket in array")?;
        Ok(self.doc.arena.create_array(elems).unwrap())
    }

    fn parse_label(&mut self) -> Option<Vec<String>> {
        let before = self.pos;
        let mut labels = Vec::new();
        loop {
            self.skip();
            let lbl = if let Some(c) = self.pos.peek() {
                if c == '"' && labels.len() == 0 {
                    self.parse_string()
                } else if c.is_ascii_alphabetic() {
                    self.parse_name()
                } else { break }.ok()
            } else { break };

            if let Some(lbl) = lbl {
                labels.push(lbl);
            } else { break }
        }

        if labels.len() > 0 && self.parse_char(':') {
            Some(labels)
        } else {
            self.pos = before;
            None
        }
    }

    fn parse_member(&mut self,
        unnamed: &mut Vec<ExprId>, named: &mut HashMap<String, ExprId>
    ) -> Result<(), ParseError> {
        self.skip();
        if let Some(mut labels) = self.parse_label() {
            let key = labels.remove(0);
            let body = self.parse_equals()?;

            if named.contains_key(&key) {
                return Err(self.error("Redefinition of label in map"));
            }

            if labels.len() == 0 {
                named.insert(key, body);
            } else {
                let func = self.doc.arena.create_func(
                    key.clone(), labels, body
                ).unwrap();
                named.insert(key, func);
            }
        } else {
            unnamed.push(self.parse_equals()?);
        }
        Ok(())
    }

    fn parse_map(&mut self, is_root: bool) -> Result<ExprId, ParseError> {
        if !is_root { self.expect('{', "Missing opening brace in map")?; }

        let mut unnamed = Vec::new();
        let mut named = HashMap::new();
        while !self.pos.is_empty() {
            let before = self.pos;
            if let Err(err) = self.parse_member(&mut unnamed, &mut named) {
                self.print_err(err);
                self.pos = before;
                self.skip_to_comma();
            }

            self.skip();
            if !self.parse_char(',') {
                let before = self.pos.idx;
                self.skip_to_comma();
                if self.pos.idx > before {
                    self.print_err(self.error("Extra content in map member"));
                }

                if !self.parse_char(',') { break; }
            }
            self.skip();
        }

        if !is_root { self.expect('}', "Missing closing brace in map")?; }
        Ok(self.doc.arena.create_map(unnamed, named).unwrap())
    }

    fn parse_call(&mut self) -> Result<ExprId, ParseError> {
        if let Some(val) = self.parse_single()? {
            let mut args = Vec::new();
            while let Some(x) = self.parse_single()? { args.push(x); }

            Ok(if args.len() == 0 { val } else {
                self.doc.arena.create_call(val, args).unwrap()
            })
        } else { Err(self.error("Missing value")) }
    }

    fn parse_lambda(&mut self) -> Result<ExprId, ParseError> {
        self.expect('\\', "Missing opening slash in lambda")?;
        let mut args = Vec::new();
        loop {
            self.skip();
            if let Some(c) = self.pos.peek() {
                if c.is_ascii_alphabetic() {
                    args.push(self.parse_name()?);
                } else { break }
            } else { break }
        }

        if args.len() == 0 {
            return Err(self.error("No arguments given for lambda"));
        } else if !self.parse_char(':') {
            return Err(self.error("Incorrect terminator for lambda arguments"));
        }

        let body = self.parse_equals()?;
        Ok(self.doc.arena.create_func("\\".to_owned(), args, body).unwrap())
    }

    fn parse_single(&mut self) -> Result<Option<ExprId>, ParseError> {
        self.skip();
        Ok(if let Some(c) = self.pos.peek() { Some(match c {
            '"' => {
                let s = Str(self.parse_string()?);
                self.doc.arena.create_obj(s)
            },
            '(' => {
                self.pos.next();
                let body = self.parse_equals()?;
                self.expect(')', "Missing close parenthesis")?;
                body
            },
            '[' => self.parse_array()?,
            '{' => self.parse_map(false)?,
            '\\' => self.parse_lambda()?,
            _ => {
                if c.is_ascii_digit() { self.parse_num()? }
                else if c.is_ascii_alphabetic() {
                    let name = self.parse_name()?;
                    if let Some(obj) = Self::parse_constant(name.as_str()) {
                        self.doc.arena.from_obj(obj)
                    } else {
                        self.doc.arena.create_var(name)
                    }
                } else { return Ok(None) }
            },
        })} else { None })
    }

    fn parse_constant(name: &str) -> Option<Object> {
        Some(match name {
            "null" => Object::new(Null()),
            "true" => Object::new(Bool(true)),
            "false" => Object::new(Bool(false)),
            "if" => Object::new(Ternary()),
            _ => return None,
        })
    }
}

