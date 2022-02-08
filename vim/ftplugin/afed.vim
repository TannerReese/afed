
if exists('b:did_ftplugin') && b:did_ftplugin
	finish
endif
let b:did_ftplugin = 1

function! AfedEval()
	let curs = getcurpos()  " Save cursor position
	%! afed -E -  " Filter entire document
	call setpos('.', curs)
endfunction

" Shortcut to evaluate document
nnoremap <buffer> ,e :call AfedEval()<CR>

" Check for errors in document
nnoremap <buffer> ,r :w ! afed -C -<CR>

