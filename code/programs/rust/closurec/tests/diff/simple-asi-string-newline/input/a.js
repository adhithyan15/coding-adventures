// CLOC26 — ASI Rule 1 across a statement that ends in a STRING literal.
//
// No semicolons; statements are separated only by newlines. The first
// statement ends in a string literal (`"total"`). Earlier the line-terminator
// rule used start-line arithmetic and conservatively DECLINED when the token
// before the newline could span lines (a string/template/regex), so this case
// degraded to WHITESPACE_ONLY. The lexer now flags a token that is preceded by
// a line terminator directly (TOKEN_PRECEDED_BY_NEWLINE), so the limitation is
// gone: ASI inserts the semicolons, the program parses, and `1 + 2` folds to
// `3`.
//
// At SIMPLE this becomes:
//   var label="total";var n=3;show(label,n);
// while WHITESPACE_ONLY keeps `1+2` verbatim (it runs no passes).
var label = "total"
var n = 1 + 2
show(label, n)
