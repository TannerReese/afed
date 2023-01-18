# Afed
A programming language for ASCII-based interactive computing
with in-document display of computation results and graphs.
Afed is an interpreted functional language inspired by Haskell.

## Usage

### From command line
`afed` takes an input file and an optional output file.
The contents of the input file is parsed and evaluated.
The results of computations are substituted into the document
and the whole contents (with substitutions) is written to the output file.
All of the following commands are equivalent
```
afed input.af output.af
afed input.af -o output.af
afed -o output.af -i input.af
```
If no output file is given then the contents is written back to the input file
so `afed input.af` is the same as `afed input.af -o input.af`.
`-` can be used for either the input or output file
to mean `STDIN` or `STDOUT`, respectively.

Along with the input and output files,
the `afed` command can take the following options
- `-C` suppresses the printing of results so only errors are shown
- `-d` removes the contents of every equals expression
- `-n` ensures that the input file is not modified
- `-e ERRORS` specifies a file to output errors to (use `-e -` to send to `STDOUT`)
- `-E` suppresses any parsing or evaluation errors

More information can be found about the command line options
can be found in the help message `afed -h`
or in the manpages `man afed`.

### From Vim

- `,,`  Evaluate and place results in the document(doesn't print errors)
- `,.`  Print out errors (doesn't place results)
- `,l`  Clear all equals expressions (e.g. ``3 + 4 = `7` `` becomes ```3 + 4 = `` ```)

All key bindings are defined in `vim/ftplugin/afed.vim`.


## Goals
Interactive computing and development environments
are well-suited to quickly mocking up new ideas.
However, many popular applications for this are based on an imperative paradigm.
As demonstrated by the popularity of spreadsheets among non-programmers,
using a declarative paradigm can often be more ergonomic.
The primary design goals for Afed are

- Results of calculations displayed in the document itself
- Declarative style so variables can be declared and used in any order
- Focus on functional means for performing computations
- As fast evaluation as can be expected for an interpreted language

To use Afed as intended, the editing environment must support a shortcut or UI element
that allows the user to easily evaluate the document.
This is implemented by plugins that call the primary Afed interpreter.

## Features

- Arithmetic operations using rationals falling back on floating point when necessary
- Heterogeneous arrays (e.g. `[1, true, "a"]`)
- Maps keyed by strings with typical accessing (e.g. `{a: 5}.a`)
- Maps support unkeyed entries
- Scoped variables using maps (e.g. `{a: 4, {"b": a + 1=}`)
- Arbitrary printing of results using *equals expression* (e.g. ``1 - (3 + 4 = `7`)``)
- Methods for primitive types
- Function declarations in maps (e.g. `f x t: 1 + x ^ t`)
- Lambda expressions (e.g. `\x y a: a * x - y`)
- Argument destructuring of maps and arrays in lambdas and functions
- First-class functions that can be passed to other functions
- Importing of other Afed files using `use`
- Help messages for all builtin types and methods (e.g. `help 5.gcd`)

There is some example code presented in the manual page, `afed.1`.
For a larger source, take a look at the test cases in `tests/examples`.

## Installation

After cloning this repository, you can build the source using `cargo build -r`.
The Afed interpreter executable will be available at `./target/release/afed`.
This executable or a link to it should be placed in `/usr/local/bin`
(or anywhere else in the `$PATH`).
Then, the `afed` command will be callable by the plugin.

To install the manpages, place the manpage file `afed.1` or a link to it into a manpath folder.
This will likely be `/usr/local/share/man/man1` or `/usr/share/man/man1`.
Alternatively, you can use `man --where` to find out all possible locations.
You should create the folder `man1` if it doesn't already exist.

Next, you need to install the appropriate plugin for your editor.

### Vim
The vim plugin files are available in the `vim` folder.
These can be installed manually.
For more information about this call `:help packages`.
Otherwise, you can use your favorite vim plugin manager such as
[vim-pathogen](https://github.com/tpope/vim-pathogen) or
[Vundle](https://github.com/VundleVim/Vundle.vim).

