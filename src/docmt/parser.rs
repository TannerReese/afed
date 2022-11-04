use std::cmp::max;
use std::fmt::{Display, Formatter, Error};
use std::collections::HashMap;

use crate::expr::ExprId;

use crate::object::{Object, Unary, Binary, Assoc};
use crate::object::null::Null;
use crate::object::bool::{Bool, Ternary};
use crate::object::number::Number;
use crate::object::string::Str;

use super::{Docmt, Subst};


macro_rules! alt {
    ($($parse:expr),+) => { loop {
        $(match $parse {
            Ok(good) => break Ok(good),
            Err(Some(err)) => break Err(Some(err)),
            Err(None) => {},
        })+
        break Err(None);
    }};
}

macro_rules! recover {
    ($doc:expr, $parse:expr, $recover:expr) => { match $parse {
        Ok(good) => Ok(Some(good)),
        Err(None) => $recover.map(|_| None),
        Err(Some(err)) => {
            $doc.add_error(err);
            $recover.map(|_| None)
        },
    }};
}

macro_rules! revert {
    ($pos:ident : $parse:expr) => {{
        let start = *$pos;
        let res = $parse;
        if res.is_err() { *$pos = start; }
        res
    }};
}

macro_rules! opt {
    ($parse:expr) => { match $parse {
        Ok(good) => Ok(Some(good)),
        Err(None) => Ok(None),
        Err(Some(errs)) => Err(Some(errs)),
    }};
}

macro_rules! ign {
    ($parse:expr) => { $parse.map(|_| ()) };
}

macro_rules! seq {
    ($pos:ident : $($parse:expr),+) => {
        revert!($pos: loop {
            break Ok(($(match $parse {
                Ok(good) => good,
                Err(err) => break Err(err),
            }),+))
        })
    };
}

macro_rules! many0 {
    ($parse:expr) => {{
        let mut results = Vec::new();
        loop { match $parse {
            Ok(val) => results.push(val),
            Err(None) => break Ok(results),
            Err(Some(errs)) => break Err(Some(errs)),
        }}
    }};
}

macro_rules! many1 {
    ($parse:expr) => {
        many0!($parse).and_then(|results|
            if results.len() == 0 { Err(None) }
            else { Ok(results) }
        )
    }
}



#[derive(Debug, Clone)]
pub struct ParseError {
    msg: String,
    line: String,
    lineno: usize,
    column: usize,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Parse Error in line {}, column {}\n", self.lineno, self.column)?;
        write!(f, "|   {}\n|   {:2$}^\n", self.line, "", self.column - 1)?;
        write!(f, "Error: {}\n", self.msg)
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

type ParseResult<T> = Result<T, Option<ParseError>>;


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

    fn tag(&mut self, name: &str) -> ParseResult<&'a str> {
        if self.ptr.starts_with(name) {
            let name = self.ptr.split_at(name.len()).0;
            self.shift(name);
            Ok(name)
        } else { Err(None) }
    }

    fn while_str<P>(&mut self, mut pred: P) -> ParseResult<&'a str>
    where P: FnMut(&str) -> bool {
        let start = self.ptr;
        while !self.is_empty() && pred(self.ptr) {
            self.next();
        }
        let len = start.len() - self.ptr.len();
        if len > 0 {
            Ok(start.split_at(len).0)
        } else { Err(None) }
    }

    fn while_char<P>(&mut self, mut pred: P) -> ParseResult<&'a str>
    where P: FnMut(char) -> bool {
        self.while_str(|s| s.chars().next().map_or(false, &mut pred))
    }

    fn skip(&mut self) -> ParseResult<()> {
        _ = many0!(alt!(
            ign!(self.while_char(|c| c.is_whitespace())),
            ign!(seq!(self:
                self.tag("#{"),
                self.while_str(|s| !s.starts_with("}#")),
                self.tag("}#")
            )),
            ign!(seq!(self:
                self.tag("#"),
                self.while_char(|c| c != '\n')
            ))
        ));
        Ok(())
    }

    fn char(&mut self, c: char) -> ParseResult<char> {
        revert!(self: {
            _ = self.skip();
            if self.peek() == Some(c) {
                self.next();
                Ok(c)
            } else { Err(None) }
        })
    }

    fn shift(&mut self, s: &str) -> usize {
        let count = s.chars().count();
        for _ in 0..count { self.next(); }
        count
    }
}


impl<'a> Pos<'a> {
    fn error(&self, msg: String) -> ParseError {
        ParseError {
            msg,
            line: self.line_begin
                .lines().next()
                .unwrap_or("").to_owned(),
            lineno: self.lineno,
            column: self.column,
        }
    }

    fn expect(&mut self, c: char, msg: &str) -> ParseResult<char> {
        let res = self.char(c);
        if let Err(None) = res {
            Err(Some(self.error(msg.to_owned())))
        } else { res }
    }

    fn constant(&mut self) -> ParseResult<Object> {
        alt!(
            self.tag("null").map(|_| Object::new(Null())),
            self.tag("true").map(|_| Object::new(Bool(true))),
            self.tag("false").map(|_| Object::new(Bool(false))),
            self.tag("if").map(|_| Object::new(Ternary()))
        )
    }

    fn number(&mut self) -> ParseResult<Number> {
        seq!(self:
            self.skip(),
            self.while_char(|c| c.is_ascii_digit() || c == '.')
            .and_then(|numstr|
                if let Ok(n) = numstr.parse::<i64>() {
                    Ok(Number::Ratio(n, 1))
                } else if let Ok(f) = numstr.parse::<f64>() {
                    Ok(Number::Real(f))
                } else { Err(Some(self.error(
                    "Invalid Number".to_owned()
                ))) }
            )
        ).map(|(_, num)| num)
    }

    fn name(&mut self) -> ParseResult<String> {
        _ = self.skip();
        if !self.peek().map_or(false, |c| c.is_alphabetic()) {
            return Err(None)
        }

        self.while_char(|c| c.is_alphanumeric() || c == '_')
        .map(|name| name.to_owned())
    }

    fn string(&mut self) -> ParseResult<String> {
        self.char('"')?;

        let mut s = String::new();
        while let Some(mut c) = self.peek() {
            if c == '"' { break; }
            else if c == '\\' {
                self.next();
                c = match self.peek() {
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
            self.next();
        }
        self.expect('"', "Missing closing quote on string")?;
        Ok(s)
    }

    fn unary(&mut self) -> ParseResult<Unary> {
        _ = self.skip();
        if let Ok(op) = self.ptr.parse::<Unary>() {
            self.shift(op.symbol());
            Ok(op)
        } else { Err(None) }
    }

    fn binary(&mut self) -> ParseResult<Binary> {
        _ = self.skip();
        if let Ok(op) = self.ptr.parse::<Binary>() {
            self.shift(op.symbol());
            Ok(op)
        } else { Err(None) }
    }
}


impl<'a> Pos<'a> {
    fn defn(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let (labels, mut body) = seq!(self:
            opt!(seq!(self:
                alt!(self.string(), self.name()),
                many0!(self.name()),
                self.char(':')
            ).map(|(name, args, _)| (name, args))),
            self.equals(doc)
        )?;

        if let Some((name, args)) = labels {
            if args.len() > 0 {
                body = doc.arena.create_func(
                    Some(name.clone()), args, body
                );
            }
            Ok(doc.arena.create_defn(name, body))
        } else { Ok(body) }
    }

    fn equals(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        revert!(self: self.expr(doc, 0)
        .and_then(|body| {
            let (start, lineno, column);
            opt!(seq!(self:
                self.char('='), self.skip(),
                {
                    lineno = self.lineno;
                    column = self.column;
                    self.expect('`', "Missing opening grave for equals")
                },
                {
                    start = self.idx;
                    while let Some(c) = self.peek() {
                        if c == '\\' { self.next(); }
                        else if c == '`' { break }
                        self.next();
                    }
                    let end = self.idx;
                    doc.push(Subst {
                        start, end,
                        lineno, column,
                        target: body, value: None,
                    });
                    Ok(())
                },
                self.expect('`', "Missing closing grave for equals")
            ))?;
            Ok(body)
        }))
    }
    fn expr(
        &mut self, doc: &mut Docmt, min_prec: usize
    ) -> ParseResult<ExprId> {
        let mut value = alt!(
            revert!(self: self.unary().and_then(|op| {
                let min_prec = max(op.prec(), min_prec) + 1;
                let arg = self.expr(doc, min_prec)?;
                Ok(doc.arena.create_unary(op, arg))
            })),
            self.call(doc)
        )?;

        _ = many0!(revert!(self: self.binary().and_then(|op| {
            let mut prec = op.prec();
            if prec < min_prec { return Err(None) }
            if op.assoc() == Assoc::Left { prec += 1; }

            let arg = self.expr(doc, prec)?;
            value = doc.arena.create_binary(op, value, arg);
            Ok(())
        })));
        Ok(value)
    }

    fn call(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let ((val, attrs), args) = seq!(self:
            self.access(doc),
            many0!(self.access(doc).map(|(arg, arg_attrs)|
                doc.arena.create_access(arg, arg_attrs)
            ))
        )?;
        Ok(doc.arena.create_call(val, attrs, args))
    }

    fn access(
        &mut self, doc: &mut Docmt
    ) -> ParseResult<(ExprId, Vec<String>)> { seq!(self:
        self.single(doc),
        many0!(seq!(self:
            self.char('.'), self.name()
        ).map(|(_, nm)| nm))
    )}

    fn single(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        seq!(self: self.skip(), alt!(
            self.string().map(|s| doc.arena.create_obj(Str(s))),
            seq!(self:
                self.char('('), self.defn(doc), self.char(')')
            ).map(|(_, id, _)| id),
            self.array(doc),
            self.map(doc),
            self.lambda(doc),
            self.number().map(|num| doc.arena.create_obj(num)),
            self.constant().map(|obj| doc.arena.from_obj(obj)),
            self.name().map(|name| doc.arena.create_var(name))
        )).map(|(_, id)| id)
    }



    fn member(
        &mut self, term: char, doc: &mut Docmt
    ) -> ParseResult<Option<ExprId>> {
        recover!(doc,
            seq!(self: self.defn(doc), alt!(self.char(','), {
                _ = self.skip();
                let c = self.peek();
                if c == Some(term) || c == None { Ok(term) }
                else { Err(None) }
            })).map(|(id, _)| id),
            seq!(self:
                self.skip(),
                self.while_char(|c| c != term && c != ',')
                .map(|_| { doc.add_error(self.error(
                    "Ill formed member".to_owned()
                )); }),
                opt!(self.char(','))
            )
        )
    }

    fn array(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let membs = seq!(self: self.char('['),
            many0!(self.member(']', doc)),
            self.expect(']', "Missing closing bracket in array")
        )?.1;
        let membs = membs.into_iter().filter_map(|opt| opt).collect();
        Ok(doc.arena.create_array(membs))
    }

    fn map(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let (_, id, _) = seq!(self:
            self.char('{'),
            self.map_no_braces(doc),
            self.expect('}', "Missing closing brace in map")
        )?;
        Ok(id)
    }

    fn map_no_braces(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let mut keys = Vec::new();
        let mut membs = Vec::new();
        _ = many0!(self.member('}', doc).map(|opt| {
            if let Some(id) = opt {
                let mut has_redef = false;
                let mut defns = doc.arena.get_defns(id);
                for nm in defns.iter() {
                    if keys.contains(nm) {
                        doc.add_error(self.error(format!(
                            "Redefinition of label '{}' in map", nm
                        )));
                        has_redef = true;
                    }
                }

                if !has_redef {
                    keys.append(&mut defns);
                    membs.push(id);
                }
            };
        }))?;
        Ok(doc.arena.create_map(membs))
    }

    fn lambda(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let (_, args, _, body) = seq!(self:
            self.char('\\'),
            many1!(self.name()),
            self.expect(':', "Missing colon in lambda definition"),
            self.expr(doc, 0)
        )?;
        Ok(doc.arena.create_func(None, args, body))
    }
}

pub fn parse(doc: &mut Docmt, src: &str, bltns: HashMap<String, Object>) {
    let mut pos = Pos::new(src);
    match pos.map_no_braces(doc) {
        Ok(root) => {
            _ = pos.skip();
            if !pos.is_empty() {
                doc.add_error(pos.error(
                    "Extra unparsed content in document".to_owned()
                ))
            }

            doc.arena.resolve_builtins(root, bltns);
        }
        Err(Some(err)) => doc.add_error(err),
        Err(None) => {},
    }
}

