
if exists('b:afed_plugin') && b:afed_plugin
	finish
endif
let b:afed_plugin = 1

function! AfedEval()
	let curs = getcurpos()
	" Filter through Afed without printing errors
	:%! afed -E -
	call setpos('.', curs)
endfunction

" Shortcut to evaluate document
nnoremap <buffer> ,e :call AfedEval()<CR>

" Check for errors in document
nnoremap <buffer> ,r :w ! afed -C -<CR>

