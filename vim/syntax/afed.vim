
if exists("b:current_syntax") && b:current_syntax
	finish
endif
let b:current_syntax = 1

syn sync minlines=300

" Expression operators
syn match afedOper /\v[\:!$%&|*+-/<=>?@^~]+/
syn keyword afedOper if
hi link afedOper Operator

" Syntactic Keywords
syn keyword afedKeyword use help
hi link afedKeyword Keyword

" Variable name
syn match afedVar /\v\a\w*/
hi link afedVar Ignore

" Numeric literals
syn match afedDigit /\v-?\d+(\.\d+)?/
hi link afedDigit Number

" String literals
syn region afedString start=/"/ skip=/\\./ end=/"/ keepend
hi link afedString String

" Named constants
syn keyword afedConstant null true false
hi link afedConstant Constant

" Builtin objects
syn keyword afedBuiltin num arr prs mod vec mat calc plt contained
syn match afedBuiltinWithPeriod /\v\a\w*\./ contains=afedBuiltin,afedOper
hi link afedBuiltin Structure

" Results of Calculation
syn region afedResult start=/`/ skip=/?./ end=/`/ keepend contained
syn region afedEqualsStmt start=/\v\=(\s|\n|#[^\n]*\n|#\{.{-}\}#)*`/ skip=/?./ end=/`/ keepend contains=afedOper,afedResult
hi link afedResult Special

" Identifier in Map
syn match afedLabel /\v(\a\w*|\s|\n|_|\[.{-}\]|\{.{-}\}|#[^\n]*\n|#\{.{-}\}#)*:/ contains=afedName,afedIgnorePattern,afedString,afedOper,afedComment
hi link afedLabel Ignore
syn keyword afedIgnorePattern _
hi link afedIgnorePattern Structure
syn match afedName /\v(\a|_)\w*/ contained
hi link afedName Identifier 

" Single line comments
syn match afedComment "\v#[^{]?.*$"
syn region afedComment start=/\v#\{/ end=/\v\}#/ keepend
hi link afedComment Comment

