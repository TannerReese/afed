use std::ffi::OsString;
use std::fmt::{Display, Error, Formatter};
use std::io::Write;
use std::path::PathBuf;

use super::expr::{ExprArena, ExprId};
use afed_objects::{bltn::Bltn, Object};

use parser::{parse, ParseError};

#[macro_use]
mod combins;
mod parser;

/* To place the results back into the document, we need to keep track
 * of where the values need to be placed. `Docmt` maintains a list of
 * substitutions for this purpose.  Each substitution represents a location
 * where a result can be placed.  When printing the result, the `Docmt`
 * prints the text outside the substitutions verbatim.
 */
pub struct Docmt {
    /* Length of `src` which is needed since the `Docmt` doesn't have access
     * to `src` while parsing, but needs it to check new `Subst`
     */
    len: usize,
    src: String, // Text to be parsed

    /* List of files in the "parse stack". This is used to prevent circular
     * dependencies from file inclusion. The last element of `paths` will
     * be the current filename unless `paths` is empty. If empty then `src`
     * was acquired from STDIN and so has unknown source.
     */
    pub paths: Vec<PathBuf>,

    is_parsed: bool, // Flag set to true after parsing

    // AST generated during parsing and used during evaluation
    arena: ExprArena,
    // List of parse errors encountered while parsing
    errors: Vec<ParseError>,
    // List of substitutions ordered by location in `src`
    // NOTE: Two `Subst` cannot overlap
    substs: Vec<Subst>,

    /* When true the document is parsed and evaluated,
     * but results are not placed in the `Subst`s
     */
    pub only_clear: bool,

    /* When true the `Docmt` will ignore calls to `push` new `Subst`.
     * It's used when parsing multiple files. The same `Docmt` is used to
     * parse all files.  So this flag is turned on when not parsing the
     * primary program so that the equal statements in the other files
     * don't get recorded.  The flag is then reverted after parsing the
     * included file.
     */
    pub ignore_substs: bool,
}

struct Subst {
    // Location of `Subst` in `src` of `Docmt`
    start: usize,
    end: usize,
    // File from which this substitution was obtained
    filename: Option<OsString>,
    // Line and column number of character before substitution
    lineno: usize,
    column: usize,

    // Expression whose value is printed in the substitution
    target: ExprId,
    // Cached value of substitution
    value: Option<Object>,
}

impl Docmt {
    pub fn new(src: String, path: Option<PathBuf>) -> Docmt {
        // Treat `path` as relative to the current directory
        let paths = path
            .and_then(|p| p.canonicalize().ok())
            .into_iter()
            .collect();

        Docmt {
            len: src.len(),
            src,
            paths,
            arena: ExprArena::new(),
            is_parsed: false,
            only_clear: false,
            errors: Vec::new(),
            substs: Vec::new(),
            ignore_substs: false,
        }
    }

    // Parse document using `parse` method which uses `ParsingContext`
    pub fn parse<W: Write>(&mut self, err_out: &mut W, bltns: Bltn) -> Result<(), usize> {
        if !self.is_parsed {
            let src = std::mem::take(&mut self.src);
            parse(self, &src, bltns);
            self.src = src;
            self.is_parsed = true;
        }

        // Print any parse errors that occur
        for err in self.errors.iter() {
            writeln!(err_out, "{}", err).expect("IO Error while writing parse error");
        }

        let count = self.errors.len();
        if count == 0 {
            Ok(())
        } else {
            Err(count)
        }
    }

    // Evaluate the `ExprArena` and print the results into the substitutions
    pub fn eval<W: Write>(&mut self, err_out: &mut W) -> Result<(), usize> {
        if self.only_clear {
            return Ok(());
        }

        let mut err_count = 0;
        for Subst {
            filename,
            target,
            value,
            lineno,
            column,
            ..
        } in self.substs.iter_mut()
        {
            if value.is_some() {
                continue;
            }
            let res = self.arena.eval(*target);
            // Print EvalError to `err_out`
            if res.is_err() {
                write!(err_out, "line {}, column {}", lineno, column)
                    .and_then(|_| {
                        if let Some(name) = filename {
                            write!(err_out, " of {:?}", name)
                        } else {
                            Ok(())
                        }
                    })
                    .and_then(|_| writeln!(err_out, " {}", res))
                    .expect("IO Error while writing eval error");
                err_count += 1;
            }
            *value = Some(res);
        }
        if err_count > 0 {
            Err(err_count)
        } else {
            Ok(())
        }
    }

    // Add `ParseError` to list of parse errors
    fn add_error(&mut self, mut err: ParseError) {
        if let Some(name) = self.paths.last().and_then(|file| file.file_name()) {
            err.set_filename(name.to_owned());
        }
        self.errors.push(err);
    }

    // Check and add new `Subst` to list of substitions in correct spot
    fn push(&mut self, mut new: Subst) -> bool {
        if self.ignore_substs {
            return false;
        }
        if new.start > new.end || new.end > self.len {
            return false;
        }

        // Find correct location for `new`
        let i = self
            .substs
            .iter()
            .enumerate()
            .rev()
            .find(|(_, sbs)| sbs.start < new.start)
            .map_or(0, |(i, _)| i + 1);

        if i > 0 {
            if let Some(before) = self.substs.get(i - 1) {
                if before.end > new.start {
                    return false;
                }
            }
        }

        if let Some(after) = self.substs.get(i) {
            if new.end > after.start {
                return false;
            }
        }

        // Make sure the `target` expression keeps its cached value
        self.arena.set_saved(new.target);
        if let Some(name) = self.paths.last().and_then(|file| file.file_name()) {
            new.filename = Some(name.to_owned());
        }
        self.substs.insert(i, new);
        true
    }
}

impl Display for Docmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let mut last = 0;
        for Subst {
            start, end, value, ..
        } in self.substs.iter()
        {
            if last >= self.len {
                break;
            }
            f.write_str(&self.src[last..*start])?;

            if self.only_clear {
                last = *end;
                continue;
            }

            if let Some(obj) = value {
                let sub = obj.to_string();
                /* Escape all graves so that repeated
                 * calls of afed don't break things
                 */
                let sub = sub
                    .chars()
                    .flat_map(|c| {
                        if c == '`' || c == '?' {
                            vec!['?', c]
                        } else {
                            vec![c]
                        }
                    })
                    .collect::<String>();
                write!(f, "{}", sub)?;
            }
            last = *end;
        }

        if last < self.src.len() {
            f.write_str(&self.src[last..])
        } else {
            Ok(())
        }
    }
}
