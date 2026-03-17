" Vim syntax file
" Language: Diagramy (.dgmy)
" Maintainer: Diagramy Project
" Latest Revision: 2026-03-17

if exists("b:current_syntax")
  finish
endif

" Keywords
syn keyword dgmyKeyword diagram box is at dim grid
syn keyword dgmyKeyword nextgroup=dgmyIdentifier skipwhite

" Properties
syn keyword dgmyProperty version width color text top borderStyle
syn keyword dgmyProperty contained

" Border styles
syn keyword dgmyBorderStyle solid dotted dashed none
syn keyword dgmyBorderStyle contained

" Colors
syn keyword dgmyColor red green blue yellow purple cyan orange pink white grey
syn keyword dgmyColor contained

" Operators
syn match dgmyOperator "="
syn match dgmyOperator ":"
syn match dgmyColon ":" contained

" Numbers (for coordinates and dimensions)
syn match dgmyNumber "\<\d\+\>"
syn match dgmyDimension "\<\d\+x\d\+\>"
syn match dgmyCoordinate "(\s*\d\+\s*,\s*\d\+\s*)"

" Strings (double-quoted)
syn region dgmyString start='"' end='"' skip='\\"'

" Version numbers
syn match dgmyVersion '"\d\+\.\d\+\.\d\+"'

" Identifiers (box names, property values)
syn match dgmyIdentifier "\<[a-zA-Z_][a-zA-Z0-9_]*\>"

" Comments (C++ style)
syn match dgmyComment "//.*$"
syn region dgmyComment start="/\*" end="\*/"

" Braces
syn match dgmyBrace "{"
syn match dgmyBrace "}"

" Special highlighting for property assignments
syn match dgmyPropertyAssignment "\<\(version\|width\|color\|text\|top\|borderStyle\|grid\)\s*:" contains=dgmyProperty,dgmyColon

" Highlight groups
hi def link dgmyKeyword Keyword
hi def link dgmyProperty Identifier
hi def link dgmyBorderStyle Type
hi def link dgmyColor Constant
hi def link dgmyOperator Operator
hi def link dgmyColon Operator
hi def link dgmyNumber Number
hi def link dgmyDimension Number
hi def link dgmyCoordinate Number
hi def link dgmyString String
hi def link dgmyVersion String
hi def link dgmyIdentifier Normal
hi def link dgmyComment Comment
hi def link dgmyBrace Delimiter

let b:current_syntax = "dgmy"

