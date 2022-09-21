
if exists("b:current_syntax") && b:current_syntax
	finish
endif
let b:current_syntax = 1

" Match multiline regex patterns accurately
" syn sync linecont /\v([{,]\s*$)|(^\s*$)/
syn sync minlines=300

" Expression operators
syn match afedOper /\v[!$%&*+-/<=>?@^~]+/
syn match afedOper /:/
hi link afedOper Operator

" Numeric literals
syn match afedDigit /\v-?\d+\.?\d*([eE][+-]?\d+)?/
syn match afedDigit /\v0[xX]\x+\.?\x*([pP][+-]?\d+)?/
hi link afedDigit Number

" String literals
syn region afedString start=/"/ skip=/\\"/ end=/"/ keepend
hi link afedString String

" Named constants
syn keyword afedConstant true false pi e
hi link afedConstant Structure

" Results of Calculation
syn region afedResult start=/`/ skip=/\\`/ end=/`/ keepend
hi link afedResult Special

" Identifier in Map
syn match afedName /\v\a\w*/ contained
syn match afedLabel /\v(%^|[{,])(\s|\n)*(\a\w*(\s|\n)*)*:/ contains=afedName,afedOper
hi link afedLabel Ignore
hi link afedName Identifier 

" Single line comments
syn match afedComment "\v#[^{]?.*$"
syn region afedComment start=/\v#\{/ end=/\v\}#/ keepend
hi link afedComment Comment

