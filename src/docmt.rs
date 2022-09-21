use std::mem;
use std::io::Write;
use std::fmt::{Display, Formatter, Error};
use std::collections::HashMap;

use super::opers;
use super::object::{Object, EvalError};
use super::expr::{Expr, ExprArena};

struct Subst {
    start: usize,
    end: usize,
    
    lineno: usize,
    column: usize,
    target: Expr,
    value: Option<Result<Object, EvalError>>,
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
    
    pub fn parse<W>(&mut self, err_out: &mut W) -> Result<(), usize> where W: Write {
        if !self.is_parsed {
            let src: String = mem::take(&mut self.src);
            let mut prs = Parser {doc: self, pos: Pos::new(&src), err_out, err_count: 0};
            _ = prs.parse_map(true);
            if !prs.pos.is_empty() {
                prs.print_err(prs.error("Extra unparsed content in document"))
            }
            self.err_count = prs.err_count;
            
            self.src = src;
            self.is_parsed = true;
        }
        if self.err_count > 0 { Err(self.err_count) } else { Ok(()) }
    }
     
    pub fn eval<W>(&mut self, err_out: &mut W) -> Result<(), usize> where W: Write {
        let mut substs = mem::replace(&mut self.substs, Vec::new());
        let mut err_count = 0;
        for Subst {target, value, lineno, column, ..} in substs.iter_mut() {
            if value.is_some() { continue; }
            *value = Some(self.arena.eval(*target));
            if let Some(Err(err)) = value {
                if let Err(_) = write!(err_out,
                    "line {}, column {} {}\n",
                    lineno, column, err
                ) { panic!("IO Error while writing eval error"); }
                err_count += 1;
            }
        }
        self.substs = substs;
        if err_count > 0 { Err(err_count) } else { Ok(()) }
    }
    
    
    fn push(&mut self,
        start: usize, end: usize,
        lineno: usize, column: usize,
        target: Expr
    ) -> bool {
        if start > end || end > self.len { return false }
        
        let i = self.substs.iter()
            .enumerate().rev()
            .filter(|(_, sbs)| sbs.start < start)
            .next().map_or(0, |(i, _)| i + 1);
        
        if i > 0 { if let Some(before) = self.substs.get(i - 1) {
            if before.end > start { return false }
        }}
        
        if let Some(after) = self.substs.get(i) {
            if end > after.start { return false }
        }
        
        self.substs.insert(i, Subst {
            start, end,
            lineno, column,
            target, value: None,
        });
        return true;
    }
}

impl Display for Docmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let mut last = 0;
        for Subst {start, end, value, ..} in self.substs.iter() {
            if last >= self.len { break; }
            
            f.write_str(&self.src[last..*start])?;
            if let Some(Ok(obj)) = value { write!(f, "{}", obj)?; }
            else if let Some(Err(err)) = value { write!(f, "{}", err)?; }
            last = *end;
        }
        
        if last < self.src.len() {
            f.write_str(&self.src[last..])
        } else { Ok(()) }
    }
}


pub const CONSTANTS: [(&str, Object); 4] = [
    ("null", Object::Null),
    ("true", Object::Bool(true)),
    ("false", Object::Bool(false)),
    ("pi", Object::Num(std::f64::consts::PI)),
];



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
    
    fn parse_equals(&mut self) -> Result<Expr, ParseError> {
        let body = self.parse_expr(0)?;
        self.skip();
        if self.parse_char('=') {
            self.expect('`', "Missing opening grave in substitution expression")?;
            
            let start = self.pos.idx;
            self.pos.skip_while(|c| c != '`');
            let end = self.pos.idx;
            
            self.expect('`', "Missing closing grave in substition expression")?;
            self.doc.push(
                start, end,
                self.pos.lineno, self.pos.column,
                body
            );
        }
        Ok(body)
    }
    
    fn parse_expr(&mut self, min_prec: usize) -> Result<Expr, ParseError> {
        let mut value = if let Some(op) = self.parse_unary() {
            let prec = op.prec();
            let arg = self.parse_expr(prec)?;
            self.doc.arena.new_unary(op, arg).unwrap()
        } else { self.parse_single()? };
        
        let mut before_oper = self.pos;
        while let Some(op) = self.parse_binary() {
            let mut prec = op.prec();
            if prec < min_prec { break }
            if op.assoc() == opers::Assoc::Left { prec += 1; }
            
            let arg = self.parse_expr(prec)?;
            self.skip();
            value = self.doc.arena.new_binary(op, value, arg).unwrap();
            before_oper = self.pos;
        }
        self.pos = before_oper;
        return Ok(value);
    }
    
    
    
    fn parse_num(&mut self) -> Result<Expr, ParseError> {
        let numstr = self.pos.ptr;
        let len = self.pos.skip_while(|c| c.is_ascii_digit() || c == '.');
        numstr[..len].parse::<f64>().map_or(
            Err(self.error("Invalid number")),
            |num| Ok(self.doc.arena.new_num(num))
        )
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
    
    fn parse_array(&mut self) -> Result<Expr, ParseError> {
        self.expect('[', "Missing opening bracket in array")?;
        
        let mut elems = Vec::new();
        loop {
            elems.push(self.parse_equals()?);
            if !self.parse_char(',') { break; }
        }
        
        self.expect(']', "Missing closing bracket in array")?;
        Ok(self.doc.arena.new_arr(elems).unwrap())
    }
    
    fn parse_member(&mut self) -> Result<(Option<String>, Expr), ParseError> {
        self.skip();
        let before = self.pos;
        let label = if let Some(c) = self.pos.peek() {
            if c == '"' { self.parse_string().ok() }
            else if c.is_ascii_alphabetic() { self.parse_name().ok() }
            else { None }
        } else { None };
        
        if label.is_some() && self.parse_char(':') {
            Ok((label, self.parse_equals()?))
        } else {
            self.pos = before;
            Ok((None, self.parse_equals()?))
        }
    }
    
    fn parse_map(&mut self, is_root: bool) -> Result<Expr, ParseError> {
        if !is_root { self.expect('{', "Missing opening brace in map")?; }
        
        let mut free_elems = Vec::new();
        let mut elems = HashMap::new();
        while !self.pos.is_empty() {
            let before = self.pos;
            match self.parse_member() {
                Ok((Some(label), body)) => if elems.contains_key(&label) {
                    self.print_err(self.error("Redefinition of label in map"));
                } else { elems.insert(label, body); },
                Ok((None, body)) => free_elems.push(body),
                Err(err) => {
                    self.print_err(err);
                    self.pos = before;
                    self.skip_to_comma();
                },
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
        }
        
        if !is_root { self.expect('}', "Missing closing brace in map")?; }
        Ok(self.doc.arena.new_map(free_elems, elems).unwrap())
    }
    
    fn parse_single(&mut self) -> Result<Expr, ParseError> {
        self.skip();
        match self.pos.peek() {
            None => Err(self.error("Unexpected End of Input")),
            Some('"') => {
                let s = self.parse_string()?;
                Ok(self.doc.arena.new_str(s))
            },
            Some('(') => {
                self.pos.next();
                let body = self.parse_equals()?;
                self.expect(')', "Missing close parenthesis")?;
                Ok(body)
            },
            Some('[') => self.parse_array(),
            Some('{') => self.parse_map(false),
            Some(c) => {
                if c.is_ascii_digit() { self.parse_num() }
                else if c.is_alphabetic() {
                    let name = self.parse_name()?;
                    Ok(if let Some((_, obj)) = CONSTANTS.iter()
                    .filter(|(key, _)| key == &name).next() {
                        self.doc.arena.from_obj(obj)
                    } else {
                        self.doc.arena.new_name(name)
                    })
                } else { Err(self.error("Unknown token at beginning of value")) }
            },
        }
    }
}


