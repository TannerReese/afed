use std::cmp::max;
use std::ffi::OsString;
use std::fmt::{Display, Error, Formatter};
use std::fs::read_to_string;
use std::path::PathBuf;

use crate::expr::{ExprId, Pattern};

use afed_objects::{
    bool::Ternary, impl_operable, name_type, null::Null, number::Number, pkg::Pkg,
    string::PrintStr, Assoc, Binary, Object, Unary,
};

use super::{Docmt, Subst};

#[derive(Debug, Clone)]
pub struct ParseError {
    msg: String,

    /* If `None` then the error originated in STDIN
     * Otherwise the absolute path of the file the error comes from
     */
    filename: Option<OsString>,

    // Copy of the line to show user the erroneous section
    line: String,
    // Location in file the error happened at
    lineno: usize,
    column: usize,
}

impl ParseError {
    pub fn set_filename(&mut self, name: OsString) -> bool {
        if self.filename.is_some() {
            false
        } else {
            self.filename = Some(name);
            true
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "Parse Error in line {}, column {}",
            self.lineno, self.column
        )?;
        if let Some(name) = &self.filename {
            write!(f, " of {:?}", name)?;
        }

        writeln!(f, "\n|   {}\n|   {:2$}^", self.line, "", self.column - 1)?;
        writeln!(f, "Error: {}", self.msg)
    }
}

macro_rules! parse_err {
    ($ctx:expr, $($arg:tt)+) => { $ctx.error(format!($($arg)+)) }
}

// Generated by `ParsingContext::help`
// Produces help messages when evaluated
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HelpStmt();
name_type! {help: HelpStmt}

impl_operable! {HelpStmt:
    #[call]
    fn __call(&self, attr: String, obj: Object) -> Result<PrintStr, String> {
        obj.help(if attr.is_empty() { None } else { Some(attr.as_str()) })
        .map(PrintStr)
        .ok_or_else(|| format!("No help exists for attribute '{}'", attr))
    }
}

impl Display for HelpStmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "help")
    }
}

impl From<HelpStmt> for Object {
    fn from(h: HelpStmt) -> Self {
        Object::new(h)
    }
}

#[derive(Debug, Clone, Copy)]
struct ParsingContext<'a> {
    // String slice containing program
    ptr: &'a str,
    // Current byte index in string slice
    idx: usize,

    line_begin: &'a str, // Beginning of current line
    lineno: usize,       // Current line number (one indexed)
    column: usize,       // Current column number (one indexed)
}

/* Cases:
 *   Ok(value) => Successfully parsed value
 *   Err(None) => Failed to parse (recoverable)
 *   Ok(Some(err)) => Unrecoverable error
 *
 * Errors are unrecoverable when the characters being parsed
 * should not be meaningfully parsed by another symbol.
 * This is implemented using `Result` instead of as an `enum`
 * to allow for the try syntax `?`.
 */
type ParseResult<T> = Result<T, Option<ParseError>>;

impl<'a> ParsingContext<'a> {
    fn new(s: &'a str) -> ParsingContext<'a> {
        ParsingContext {
            ptr: s,
            idx: 0,
            line_begin: s,
            lineno: 1,
            column: 1,
        }
    }

    fn is_empty(&self) -> bool {
        self.ptr.len() == 0
    }
    fn peek(&self) -> Option<char> {
        self.ptr.chars().next()
    }

    // Consume a single character and update position
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
        } else {
            None
        }
    }

    // Try to consume the given string slice
    fn tag_raw(&mut self, name: &str) -> ParseResult<&'a str> {
        if self.ptr.starts_with(name) {
            let name = self.ptr.split_at(name.len()).0;
            self.shift(name);
            Ok(name)
        } else {
            Err(None)
        }
    }

    /* Consume characters while the remaining slice fulfills `pred`
     * On success, returns the consumed slice
     * Fails to parse (recoverable) when no characters found
     */
    fn while_str<P>(&mut self, mut pred: P) -> ParseResult<&'a str>
    where
        P: FnMut(&str) -> bool,
    {
        let start = self.ptr;
        while !self.is_empty() && pred(self.ptr) {
            self.next();
        }
        let len = start.len() - self.ptr.len();
        if len > 0 {
            Ok(start.split_at(len).0)
        } else {
            Err(None)
        }
    }

    // Consumes characters while they fulfill `pred`
    fn while_char<P>(&mut self, mut pred: P) -> ParseResult<&'a str>
    where
        P: FnMut(char) -> bool,
    {
        self.while_str(|s| s.chars().next().map_or(false, &mut pred))
    }

    // Try to parse as much whitespace and comments as possible
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

    // Try to parse a single character after skipping
    fn char(&mut self, c: char) -> ParseResult<char> {
        revert!(self: {
            _ = self.skip();
            if self.peek() == Some(c) {
                self.next();
                Ok(c)
            } else { Err(None) }
        })
    }

    // Try to parse a string slice after skipping
    fn tag(&mut self, name: &str) -> ParseResult<&'a str> {
        seq!(self: self.skip(), self.tag_raw(name))
    }

    // Parse end of input
    fn eoi(&mut self) -> ParseResult<()> {
        revert!(self: {
            _ = self.skip();
            if self.peek().is_none() { Ok(()) }
            else { Err(None) }
        })
    }

    fn shift(&mut self, s: &str) -> usize {
        let count = s.chars().count();
        for _ in 0..count {
            self.next();
        }
        count
    }
}

// Each method parses a symbol of the grammar and returns its value
impl<'a> ParsingContext<'a> {
    fn error(&self, msg: String) -> ParseError {
        ParseError {
            msg,
            filename: None,
            line: self.line_begin.lines().next().unwrap_or("").to_owned(),
            lineno: self.lineno,
            column: self.column,
        }
    }

    fn expect(&mut self, c: char, msg: &str) -> ParseResult<char> {
        let res = self.char(c);
        if let Err(None) = res {
            Err(Some(parse_err!(self, "{}", msg)))
        } else {
            res
        }
    }

    fn constant(&mut self) -> ParseResult<Object> {
        alt!(
            self.tag("null").map(|_| Object::new(Null())),
            self.tag("true").map(|_| true.into()),
            self.tag("false").map(|_| false.into()),
            self.tag("if").map(|_| Object::new(Ternary())),
            // Don't allow keywords to be used as variables
            alt!(self.tag("use"), self.tag("help")).and_then(|word| Err(Some(parse_err!(
                self,
                "'{}' keyword cannot be used as a variable",
                word
            ))))
        )
    }

    // First try to parse as integer and then as float
    fn number(&mut self) -> ParseResult<Number> {
        let (intstr, fracstr) = seq!(self: self.skip(),
            tuple!(self:
                self.while_char(|c| c.is_ascii_digit()),
                opt!(seq!(self:
                    self.char('.'), self.while_char(|c| c.is_ascii_digit())
                ))
            )
        )?;

        if let Some(fracstr) = fracstr {
            if let Ok(f) = format!("{}.{}", intstr, fracstr).parse::<f64>() {
                return Ok(Number::Real(f));
            }
        } else if let Ok(n) = intstr.parse::<i64>() {
            return Ok(Number::Ratio(n, 1));
        }
        Err(Some(parse_err!(self, "Invalid Number")))
    }

    // Parse unbroken string of alphanumerics and underscores
    fn name(&mut self) -> ParseResult<String> {
        _ = self.skip();
        if !self.peek().map_or(false, |c| c.is_alphabetic() || c == '_') {
            return Err(None);
        }

        self.while_char(|c| c.is_alphanumeric() || c == '_')
            .and_then(|name| {
                if name == "_" {
                    Err(None)
                } else {
                    Ok(name.to_owned())
                }
            })
    }

    // Parse string delimited by quotes with standard escaping
    fn string(&mut self) -> ParseResult<String> {
        self.char('"')?;

        let mut s = String::new();
        while let Some(mut c) = self.peek() {
            if c == '"' {
                break;
            } else if c == '\\' {
                // Convert escaped characters
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
        } else {
            Err(None)
        }
    }

    fn binary(&mut self) -> ParseResult<Binary> {
        _ = self.skip();
        if let Ok(op) = self.ptr.parse::<Binary>() {
            self.shift(op.symbol());
            Ok(op)
        } else {
            Err(None)
        }
    }
}

impl<'a> ParsingContext<'a> {
    // Parse optional definition (e.g. 'x: 1 + 3' but also '1 + 3')
    fn defn(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let (labels, mut body) = tuple!(
            self: opt!(alt!(
                // Parse empty definition for map target
                self.char(':').map(|_| ("".to_owned(), vec![])),
                seq!(self: ,
                    tuple!(self:
                        // Parse key as "key" or key
                        alt!(self.string(), self.name()),
                        // Parse as many arguments as possible
                        many0!(self.pattern())
                    ),
                    self.char(':')
                ),
                // Try treating the key as a pattern instead
                match self.defn_with_pat(doc) {
                    Ok(id) => return Ok(id),
                    Err(err) => Err(err),
                }
            )),
            self.equals(doc)
        )?;

        // Make sure no arguments are duplicated
        if let Some((name, pats)) = labels {
            if !pats.is_empty() {
                if let Some(dup) = Pattern::has_duplicate_args(&pats) {
                    return Err(Some(parse_err!(self, "Duplicate argument '{}'", dup)));
                }

                body = doc.arena.create_func(Some(name.clone()), pats, body);
            }
            Ok(doc.arena.create_defn(name, body))
        } else {
            Ok(body)
        }
    }

    fn defn_with_pat(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let (pat, _, body) = tuple!(self: self.pattern(), self.char(':'), self.equals(doc))?;
        Ok(doc.arena.create_defn_with_pat(pat, body))
    }

    // Parse optional equals statement (e.g. '3 + 4 = ``' but also '3 + 4')
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
                    // Create new substitution in `doc` for equals statement
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

    // Recursive Pratt parser for infix expressions
    fn expr(&mut self, doc: &mut Docmt, min_prec: usize) -> ParseResult<ExprId> {
        let mut value = alt!(
            revert!(self: self.unary().and_then(|op| {
                let min_prec = max(op.prec(), min_prec) + 1;
                let arg = self.expr(doc, min_prec)?;
                Ok(doc.arena.create_unary(op, arg))
            })),
            alt!(self.help(doc), self.call(doc))
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

    // Parse <caller>(.<attr>)* (<arg>(.<attr>)*)*
    // This encompasses attribute access, method, and function calls
    fn call(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let ((val, attrs), args) = tuple!(self:
            self.access(doc),
            many0!(self.access(doc).map(|(arg, arg_attrs)|
                doc.arena.create_access(arg, arg_attrs)
            ))
        )?;
        Ok(doc.arena.create_call(val, attrs, args))
    }

    // Parse <base>(.<attr>)*
    fn access(&mut self, doc: &mut Docmt) -> ParseResult<(ExprId, Vec<String>)> {
        tuple!(self:
            self.single(doc),
            many0!(seq!(self: self.char('.'), self.name()))
        )
    }

    // Parse help statements (e.g. 'help [1, 2].len')
    fn help(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let (mut tgt, mut attrs) = seq!(self:
            self.tag("help"), self.access(doc)
        )?;
        let last = attrs.pop().unwrap_or_default();
        if !attrs.is_empty() {
            tgt = doc.arena.create_access(tgt, attrs);
        }

        let help = doc.arena.create_obj(HelpStmt());
        let last = doc.arena.create_obj(last);
        Ok(doc.arena.create_call(help, vec![], vec![last, tgt]))
    }

    // Parse atomic value in expression
    fn single(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        seq!(self: self.skip(), alt!(
            self.string().map(|s| doc.arena.create_obj(s)),
            seq!(self:  // Parse parenthetical expression
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

    // Parse array or map member
    fn member(&mut self, term: char, allow_use: bool, doc: &mut Docmt) -> ParseResult<Vec<ExprId>> {
        recover!(
            doc,
            seq!(self:,
                alt!(
                    // Maps allow use statements. Arrays do not
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
            // Recover from error in member by skipping to next comma
            seq!(self:
                self.skip(),
                self.while_char(|c| c != term && c != ','),
                opt!(self.char(','))
            )
            .map(|_| vec![])
        )
    }

    // Parse array (e.g. '[1, 2, "a"]')
    fn array(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let membs = seq!(self: self.char('['),
            many0!(self.member(']', false, doc)),
            self.expect(']', "Missing closing bracket in array")
        )?;
        let membs = membs.into_iter().flatten().collect();
        Ok(doc.arena.create_array(membs))
    }

    // Parse map (e.g. '{a: 4, b: 3}')
    fn map(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let membs = seq!(self:
            self.char('{'),
            self.map_members(doc),
            self.expect('}', "Missing closing brace in map")
        )?;
        Ok(doc.arena.create_map(membs))
    }

    // Parse multiple members while checking for redefinitions
    fn map_members(&mut self, doc: &mut Docmt) -> ParseResult<Vec<ExprId>> {
        let mut keys = Vec::new();
        let mut membs = Vec::new();
        _ = many0!(self
            .member('}', true, doc)
            .map(|ids| for id in ids.into_iter() {
                let mut has_redef = false;
                // Check that none of the definitions in member are redefs
                let mut defns = doc.arena.get_defns(id);
                for nm in defns.iter() {
                    if keys.contains(nm) {
                        doc.add_error(if nm.is_empty() {
                            parse_err!(self, "Redefinition of map target")
                        } else {
                            parse_err!(self, "Redefinition of label '{}' in map", nm)
                        });
                        has_redef = true;
                    }
                }

                if !has_redef {
                    keys.append(&mut defns);
                    membs.push(id);
                }
            }))?;
        Ok(membs)
    }

    // Parse use statement (e.g. 'use "file.af"')
    fn use_stmt(&mut self, doc: &mut Docmt) -> ParseResult<Vec<ExprId>> {
        revert!(self:
            seq!(self:
                self.tag("use"), self.string()
            ).and_then(|path| {
                let path = self.check_path(&path, doc)
                    .map_err(Some)?;
                match read_to_string(&path) {
                    Ok(content) => {
                        doc.paths.push(path);
                        let ign_subs = doc.ignore_substs;
                        doc.ignore_substs = true;
                        let membs = ParsingContext::new(&content).root(doc)?;
                        doc.ignore_substs = ign_subs;
                        doc.paths.pop();
                        Ok(membs)
                    },
                    Err(err) => Err(Some(parse_err!(self, "{}", err))),
                }
            })
        )
    }

    // Parse lambda expression (e.g. '\x: x + 3')
    fn lambda(&mut self, doc: &mut Docmt) -> ParseResult<ExprId> {
        let (pats, body) = tuple!(
            self: seq!(self:
                self.char('\\'),
                many1!(self.pattern()),
                self.expect(':', "Missing colon in lambda definition")
            ),
            self.expr(doc, 0)
        )?;

        if let Some(dup) = Pattern::has_duplicate_args(&pats) {
            Err(Some(parse_err!(self, "Duplicate argument '{}'", dup)))
        } else {
            Ok(doc.arena.create_func(None, pats, body))
        }
    }

    // Parse patterns in the arguments of functions (e.g. '_' in 'f _ x: x')
    fn pattern(&mut self) -> ParseResult<Pattern<String>> {
        alt!(
            self.char('_').map(|_| Pattern::Ignore),
            self.name().map(Pattern::Arg),
            // Array desctructuring
            seq!(self: self.char('['), many0!(
                self.pattern(), self.char(',')
            ), tuple!(self: opt!(self.char(',')), self.char(']')))
            .map(Pattern::Array),
            // Map desctructuring
            {
                let mut is_fuzzy = false;
                seq!(self: self.char('{'),
                    many0!(alt!(
                        // Allow fuzzy arguments in map destruct with '..'
                        self.tag("..").map(|_| { is_fuzzy = true; None }),
                        // Parse key-value entry in map destruct
                        tuple!(self:
                            alt!(self.name(), self.string()),
                            seq!(self: self.char(':'), self.pattern())
                        ).map(Some)
                    ), self.char(',')),
                tuple!(self: opt!(self.char(',')), self.char('}')))
                .map(|pats| Pattern::Map(is_fuzzy, pats.into_iter().flatten().collect()))
            }
        )
    }

    // Parse entire file
    fn root(&mut self, doc: &mut Docmt) -> ParseResult<Vec<ExprId>> {
        let membs = self.map_members(doc)?;
        _ = self.skip();
        if !self.is_empty() {
            doc.add_error(self.error("Extra unparsed content in document".to_owned()))
        }
        Ok(membs)
    }

    // Check that `path` isn't currently being parsed
    fn check_path(&self, path: &str, doc: &mut Docmt) -> Result<PathBuf, ParseError> {
        let path = PathBuf::from(path);
        let canonical = doc
            .paths
            .last()
            .and_then(|last_path| last_path.parent())
            // Try to find `path` relative to the location of the current file
            .and_then(|last_path| {
                if path.is_relative() {
                    last_path.join(&path).canonicalize().ok()
                } else {
                    None
                }
            })
            // Otherwise try to find `path` relative to the processes location
            .or_else(|| path.canonicalize().ok())
            .ok_or_else(|| parse_err!(self, "Cannot find path '{}'", path.display()))?;

        // Check that located file isn't already being parsed
        if doc.paths.contains(&canonical) {
            return Err(parse_err!(
                self,
                "Circular dependence in file imports from '{}'",
                canonical.display()
            ));
        }
        Ok(canonical)
    }
}

// Create and use `ParsingContext` to parse `src`
pub fn parse(doc: &mut Docmt, src: &str, pkgs: Pkg) {
    let mut ctx = ParsingContext::new(src);
    match ctx.root(doc) {
        Ok(membs) => {
            let root = doc.arena.create_map(membs);
            doc.arena.resolve_pkgs(root, pkgs);
        }
        Err(Some(err)) => doc.add_error(err),
        Err(None) => {}
    }
}
