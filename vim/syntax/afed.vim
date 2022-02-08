
if exists("b:current_syntax") && b:current_syntax
	finish
endif
let b:current_syntax = 1

" Expression operators
syn match afedOper "\v[!$%&*+-/<>?@^~]+[!$%&*+-/<=>?@^~]*"
syn match afedOper "\v\=[!$%&*+-/<=>?@^~]+"

" Numeric literals
syn match afedDigit "\v-?\d+\.?\d*([eE][+-]?\d+)?"
syn match afedDigit "\v0[xX]\x+\.?\x*([pP][+-]?\d+)?"
hi link afedDigit Number

" Results of Calculation
syn match afedResult "\v[^=#]*" contained
hi link afedResult Special

" Region containing Results of calculation
syn region afedLineEnd start=/\v\=/ end=/\v$/ keepend transparent contains=afedSyntaxOper,afedResult,afedComment

" Anything after a # is a comment
syn match afedComment "\v#.*$"
hi link afedComment Comment

" Non-Expression operators
syn match afedSyntaxOper "\v\=" contained
syn match afedSyntaxOper "\v:"
hi link afedSyntaxOper Operator

