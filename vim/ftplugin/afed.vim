
if exists('b:afed_plugin') && b:afed_plugin
	finish
endif
let b:afed_plugin = 1

function! AfedEval()
	let curs = getcurpos()
	" Filter through Afed without printing errors
	if expand("%") == ""
		:%! afed -E -
	else
		" Pass the filename if available
		:execute "%! afed -f '" . expand("%:p") . "' -E -"
	endif
	call setpos('.', curs)
endfunction

function! AfedCheck()
	if expand("%") == ""
		:w ! afed -C -
	else
		" Pass the filename if available
		:execute "w ! afed -f '" . expand("%:p") . "' -C -"
	endif
endfunction

function! AfedClear()
	let curs = getcurpos()
	:%! afed -E -d -
	call setpos('.', curs)
endfunction

" Shortcut to evaluate document
nnoremap <buffer> ,, :call AfedEval()<CR>

" Check for errors in document
nnoremap <buffer> ,. :call AfedCheck()<CR>

" Clear all substitution expressions
nnoremap <buffer> ,l :call AfedClear()<CR>

