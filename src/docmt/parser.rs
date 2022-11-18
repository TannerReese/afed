use std::cmp::max;
use std::fmt::{Display, Formatter, Error};
use std::fs::read_to_string;
use std::path::PathBuf;
use std::ffi::OsString;

use crate::expr::{ExprId, Pattern, Bltn};

use crate::object::{Object, Unary, Binary, Assoc};
use crate::object::null::Null;
use crate::object::bool::{Bool, Ternary};
use crate::object::number::Number;
use crate::object::string::Str;

use super::{Docmt, Subst};



#[derive(Debug, Clone)]
pub struct ParseError {
    msg: String,
    filename: Option<OsString>,
    line: String,
    lineno: usize,
    column: usize,
}

impl ParseError {
    pub fn set_filename(&mut self, name: OsString) -> bool {
        if self.filename.is_some() { false }
        else { self.filename = Some(name); true }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Parse Error in line {}, column {}", self.lineno, self.column)?;
        if let Some(name) = &self.filename {
            write!(f, " of {:?}", name)?;
        }

        write!(f, "\n|   {}\n|   {:2$}^\n", self.line, "", self.column - 1)?;
        write!(f, "Error: {}\n", self.msg)
    }
}

macro_rules! parse_err {
    ($pos:expr, $($arg:tt)+) => { $pos.error(format!($($arg)+)) }
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

    fn tag_raw(&mut self, name: &str) -> ParseResult<&'a str> {
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
            ign!(tuple!(self:
                self.tag_raw("#{"),
                self.while_str(|s| !s.starts_with("}#")),
                self.tag_raw("}#")
            )),
            ign!(tuple!(self:
                self.tag_raw("#"),
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

    fn tag(&mut self, name: &str) -> ParseResult<&'a str> {
        seq!(self: self.skip(), self.tag_raw(name))
    }

    fn eoi(&mut self) -> ParseResult<()> {
        revert!(self: {
            _ = self.skip();
            if let None = self.peek() { Ok(()) }
            else { Err(None) }
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
            msg, filename: None,
            line: self.line_begin
                .lines().next()
                .unwrap_or("").to_owned(),
            lineno: self.lineno,
            column: self.column,
        }
    }

    fn expect(&mut self, c: char, msg: &str) -> ParseResult<char> {
        let res = self.char(c);
        if let Err(None) = res { Err(Some(parse_err!(self, "{}", msg))) }
        else { res }
    }

    fn constant(&mut self) -> ParseResult<Object> {
        alt!(
            self.tag("null").map(|_| Object::new(Null())),
            self.tag("true").map(|_| Object::new(Bool(true))),
            self.tag("false").map(|_| Object::new(Bool(false))),
            self.tag("if").map(|_| Object::new(Ternary())),
            seq!(self: self.tag("use"), Err(Some(parse_err!(self,
                "'use' keyword can only be used for importing"
            ))))
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
                } else { Err(Some(parse_err!(self, "Invalid Number"))) }

            )
        )
    }

    fn name(&mut self) -> ParseResult<String> {
        _ = self.skip();
        if !self.peek().map_or(false, |c|
            c.is_alphabetic() || c == '_'
        ) { return Err(None) }

        self.while_char(|c| c.is_alphanumeric() || c == '_').and_then(|name|
            if name == "_" { Err(None) } else { Ok(name.to_owned()) }
        )
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
        let (labels, mut body) = tuple!(self:
            opt!(alt!(
                self.char(':').map(|_| ("".to_owned(), vec![])),
                seq!(self: ,
                    tuple!(self:
                        alt!(self.string(), self.name()),
                        many0!(self.pattern())
                    ),
                    self.char(':')
                )
            )),
            self.equals(doc)
        )?;

        if let Some((name, pats)) = labels {
            if pats.len() > 0 {
                if let Some(dup) = Pattern::has_dups(&pats) {
                    return Err(Some(parse_err!(self,
                        "Duplicate argument '{}'", dup
                    )))
                }

                body = doc.arena.create_func(
                    Some(name.clone()), pats, body
                );
            }
            Ok(doc.arena.create_defn(name, body))
        } else { Ok(body) }
    }

    fn equals(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        revert!(self: self.expr(doc, 0)
        .and_then(|body| {
            let (start, lineno, column);
            opt!(tuple!(self:
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
                        start, end, filename: None,
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
        let ((val, attrs), args) = tuple!(self:
            self.access(doc),
            many0!(self.access(doc).map(|(arg, arg_attrs)|
                doc.arena.create_access(arg, arg_attrs)
            ))
        )?;
        Ok(doc.arena.create_call(val, attrs, args))
    }

    fn access(
        &mut self, doc: &mut Docmt
    ) -> ParseResult<(ExprId, Vec<String>)> { tuple!(self:
        self.single(doc),
        many0!(seq!(self: self.char('.'), self.name()))
    )}

    fn single(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        seq!(self: self.skip(), alt!(
            self.string().map(|s| doc.arena.create_obj(Str(s))),
            seq!(self:
                self.char('('), self.defn(doc), self.char(')')
            ),
            self.array(doc),
            self.map(doc),
            self.lambda(doc),
            self.number().map(|num| doc.arena.create_obj(num)),
            self.constant().map(|obj| doc.arena.from_obj(obj)),
            self.name().map(|name| doc.arena.create_var(name))
        ))
    }



    fn member(
        &mut self, term: char, allow_use: bool, doc: &mut Docmt
    ) -> ParseResult<Vec<ExprId>> { recover!(doc,
        seq!(self:,
            alt!(
                if allow_use {
                    self.use_stmt(doc)
                } else { Err(None) },
                self.defn(doc).map(|id| vec![id])
            ),
            alt!(ign!(self.char(',')),
                alt!(self.eoi()),
                peek!(self: ign!(self.char(term)))
            ).map_err(|_| Some(parse_err!(self, "Ill formed member")))
        ),
        seq!(self:
            self.skip(),
            self.while_char(|c| c != term && c != ','),
            opt!(self.char(','))
        ).map(|_| vec![])
    )}

    fn array(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let membs = seq!(self: self.char('['),
            many0!(self.member(']', false, doc)),
            self.expect(']', "Missing closing bracket in array")
        )?;
        let membs = membs.into_iter().flatten().collect();
        Ok(doc.arena.create_array(membs))
    }

    fn map(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let membs = seq!(self:
            self.char('{'),
            self.map_members(doc),
            self.expect('}', "Missing closing brace in map")
        )?;
        Ok(doc.arena.create_map(membs))
    }

    fn map_members(
        &mut self, doc: &mut Docmt
    ) -> ParseResult<Vec<ExprId>> {
        let mut keys = Vec::new();
        let mut membs = Vec::new();
        _ = many0!(self.member('}', true, doc).map(|ids|
            for id in ids.into_iter() {
                let mut has_redef = false;
                let mut defns = doc.arena.get_defns(id);
                for nm in defns.iter() {
                    if keys.contains(nm) {
                        doc.add_error(if nm == "" { parse_err!(self,
                            "Redefinition of map target"
                        )} else { parse_err!(self,
                            "Redefinition of label '{}' in map", nm
                        )});
                        has_redef = true;
                    }
                }

                if !has_redef {
                    keys.append(&mut defns);
                    membs.push(id);
                }
            }
        ))?;
        Ok(membs)
    }

    fn use_stmt(
        &mut self, doc: &mut Docmt
    ) -> ParseResult<Vec<ExprId>> { revert!(self:
        seq!(self:
            self.tag("use"), self.string()
        ).and_then(|path| {
            let path = self.check_path(&path, doc)
                .map_err(|err| Some(err))?;
            match read_to_string(&path) {
                Ok(content) => {
                    doc.paths.push(path);
                    let ign_subs = doc.ignore_substs;
                    doc.ignore_substs = true;
                    let membs = Pos::new(&content).root(doc)?;
                    doc.ignore_substs = ign_subs;
                    doc.paths.pop();
                    Ok(membs)
                },
                Err(err) => Err(Some(parse_err!(self, "{}", err))),
            }
        })
    )}

    fn lambda(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let (pats, body) = tuple!(self:
            seq!(self:
                self.char('\\'),
                many1!(self.pattern()),
                self.expect(':', "Missing colon in lambda definition")
            ),
            self.expr(doc, 0)
        )?;

        if let Some(dup) = Pattern::has_dups(&pats) {
            Err(Some(parse_err!(self, "Duplicate argument '{}'", dup)))
        } else {
            Ok(doc.arena.create_func(None, pats, body))
        }
    }

    fn pattern(&mut self) -> ParseResult<Pattern<String>> {
        alt!(
            self.char('_').map(|_| Pattern::Ignore),
            self.name().map(|nm| Pattern::Arg(nm)),
            seq!(self: self.char('['), many0!(
                self.pattern(), self.char(',')
            ), tuple!(self: opt!(self.char(',')), self.char(']')))
            .map(|pats| Pattern::Array(pats)),

            {
                let mut is_fuzzy = false;
                seq!(self: self.char('{'),
                    many0!(alt!(
                        self.tag("..").map(|_| { is_fuzzy = true; None }),
                        tuple!(self:
                            alt!(self.name(), self.string()),
                            seq!(self: self.char(':'), self.pattern())
                        ).map(Some)
                    ), self.char(',')),
                tuple!(self: opt!(self.char(',')), self.char('}')))
                .map(|pats| Pattern::Map(is_fuzzy,
                    pats.into_iter().filter_map(|opt| opt).collect()
                ))
            }
        )
    }

    fn root(&mut self, doc: &mut Docmt) -> ParseResult<Vec<ExprId>> {
        let membs = self.map_members(doc)?;
        _ = self.skip();
        if !self.is_empty() { doc.add_error(self.error(
            "Extra unparsed content in document".to_owned()
        ))}
        Ok(membs)
    }


    fn check_path(&self, path: &str, doc: &mut Docmt) -> Result<PathBuf, ParseError> {
        let path = PathBuf::from(path);
        let canonical = doc.paths.last()
        .and_then(|last_path| last_path.parent())
        .and_then(|last_path| if path.is_relative() {
            last_path.join(&path).canonicalize().ok()
        } else { None })
        .or_else(|| path.canonicalize().ok())
        .ok_or_else(|| parse_err!(self,
            "Cannot find path '{}'", path.display()
        ))?;

        if doc.paths.contains(&canonical) {
            return Err(parse_err!(self,
                "Circular dependence in file imports from '{}'",
                canonical.display()
            ))
        }
        Ok(canonical)
    }
}


pub fn parse(doc: &mut Docmt, src: &str, bltns: Bltn) {
    let mut pos = Pos::new(src);
    match pos.root(doc) {
        Ok(membs) => {
            let root = doc.arena.create_map(membs);
            doc.arena.resolve_builtins(root, bltns);
        }
        Err(Some(err)) => doc.add_error(err),
        Err(None) => {},
    }
}

