" Vim syntax file
" Language: Diagramy (.dia)
" Maintainer: Diagramy
" Latest Revision: 2026-03-14

if exists("b:current_syntax")
  finish
endif

" Keywords
syn keyword diaKeyword version diagram box port arrow layout
syn keyword diaKeyword size scale fontsize pos interp
syn keyword diaProperty title color vertical stacked side style
syn keyword diaProperty from to

" Color names
syn keyword diaColor red blue green yellow orange purple pink cyan magenta
syn keyword diaColor lime teal indigo brown gray grey black white navy maroon olive

" Style values
syn keyword diaStyle tieoff

" Side values
syn keyword diaSide left right top bottom

" Numbers
syn match diaNumber '\<\d\+\>'
syn match diaNumber '\<\d\+\.\d\+\>'

" Percentages
syn match diaPercent '\<\d\+%'

" Strings
syn region diaString start='"' end='"' contains=diaStringEscape
syn match diaStringEscape '\\.' contained

" Comments
syn match diaComment '//.*$'
syn region diaComment start='/\*' end='\*/'

" Operators and delimiters
syn match diaOperator ':'
syn match diaOperator '='
syn match diaDelimiter '{'
syn match diaDelimiter '}'
syn match diaDelimiter '('
syn match diaDelimiter ')'
syn match diaDelimiter ','

" Identifiers (box names, port names, etc.)
syn match diaIdentifier '\<[a-zA-Z_][a-zA-Z0-9_]*\>' contains=NONE

" Version numbers
syn match diaVersion '\d\+\.\d\+\.\d\+'

" Highlighting
hi def link diaKeyword Keyword
hi def link diaProperty Type
hi def link diaColor Constant
hi def link diaStyle Constant
hi def link diaSide Constant
hi def link diaNumber Number
hi def link diaPercent Number
hi def link diaString String
hi def link diaStringEscape SpecialChar
hi def link diaComment Comment
hi def link diaOperator Operator
hi def link diaDelimiter Delimiter
hi def link diaIdentifier Identifier
hi def link diaVersion Special

let b:current_syntax = "dia"

