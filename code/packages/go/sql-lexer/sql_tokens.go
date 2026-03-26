// AUTO-GENERATED FILE - DO NOT EDIT
package sqllexer

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var SqlTokens = &grammartools.TokenGrammar{
	Version: 1,
	CaseInsensitive: true,
	CaseSensitive: true,
	Definitions: []grammartools.TokenDefinition{
		grammartools.TokenDefinition{Name: "NAME", Pattern: "[a-zA-Z_][a-zA-Z0-9_]*", IsRegex: true, LineNumber: 12, Alias: ""},
		grammartools.TokenDefinition{Name: "NUMBER", Pattern: "[0-9]+(\\.[0-9]+)?", IsRegex: true, LineNumber: 13, Alias: ""},
		grammartools.TokenDefinition{Name: "STRING_SQ", Pattern: "'([^'\\\\]|\\\\.)*'", IsRegex: true, LineNumber: 14, Alias: "STRING"},
		grammartools.TokenDefinition{Name: "QUOTED_ID", Pattern: "`[^`]+`", IsRegex: true, LineNumber: 15, Alias: "NAME"},
		grammartools.TokenDefinition{Name: "LESS_EQUALS", Pattern: "<=", IsRegex: false, LineNumber: 17, Alias: ""},
		grammartools.TokenDefinition{Name: "GREATER_EQUALS", Pattern: ">=", IsRegex: false, LineNumber: 18, Alias: ""},
		grammartools.TokenDefinition{Name: "NOT_EQUALS", Pattern: "!=", IsRegex: false, LineNumber: 19, Alias: ""},
		grammartools.TokenDefinition{Name: "NEQ_ANSI", Pattern: "<>", IsRegex: false, LineNumber: 20, Alias: "NOT_EQUALS"},
		grammartools.TokenDefinition{Name: "EQUALS", Pattern: "=", IsRegex: false, LineNumber: 22, Alias: ""},
		grammartools.TokenDefinition{Name: "LESS_THAN", Pattern: "<", IsRegex: false, LineNumber: 23, Alias: ""},
		grammartools.TokenDefinition{Name: "GREATER_THAN", Pattern: ">", IsRegex: false, LineNumber: 24, Alias: ""},
		grammartools.TokenDefinition{Name: "PLUS", Pattern: "+", IsRegex: false, LineNumber: 25, Alias: ""},
		grammartools.TokenDefinition{Name: "MINUS", Pattern: "-", IsRegex: false, LineNumber: 26, Alias: ""},
		grammartools.TokenDefinition{Name: "STAR", Pattern: "*", IsRegex: false, LineNumber: 27, Alias: ""},
		grammartools.TokenDefinition{Name: "SLASH", Pattern: "/", IsRegex: false, LineNumber: 28, Alias: ""},
		grammartools.TokenDefinition{Name: "PERCENT", Pattern: "%", IsRegex: false, LineNumber: 29, Alias: ""},
		grammartools.TokenDefinition{Name: "LPAREN", Pattern: "(", IsRegex: false, LineNumber: 31, Alias: ""},
		grammartools.TokenDefinition{Name: "RPAREN", Pattern: ")", IsRegex: false, LineNumber: 32, Alias: ""},
		grammartools.TokenDefinition{Name: "COMMA", Pattern: ",", IsRegex: false, LineNumber: 33, Alias: ""},
		grammartools.TokenDefinition{Name: "SEMICOLON", Pattern: ";", IsRegex: false, LineNumber: 34, Alias: ""},
		grammartools.TokenDefinition{Name: "DOT", Pattern: ".", IsRegex: false, LineNumber: 35, Alias: ""},
	},
	Keywords: []string{"SELECT", "FROM", "WHERE", "GROUP", "BY", "HAVING", "ORDER", "LIMIT", "OFFSET", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "DROP", "TABLE", "IF", "EXISTS", "NOT", "AND", "OR", "NULL", "IS", "IN", "BETWEEN", "LIKE", "AS", "DISTINCT", "ALL", "UNION", "INTERSECT", "EXCEPT", "JOIN", "INNER", "LEFT", "RIGHT", "OUTER", "CROSS", "FULL", "ON", "ASC", "DESC", "TRUE", "FALSE", "CASE", "WHEN", "THEN", "ELSE", "END", "PRIMARY", "KEY", "UNIQUE", "DEFAULT", },
	SkipDefinitions: []grammartools.TokenDefinition{
		grammartools.TokenDefinition{Name: "WHITESPACE", Pattern: "[ \\t\\r\\n]+", IsRegex: true, LineNumber: 95, Alias: ""},
		grammartools.TokenDefinition{Name: "LINE_COMMENT", Pattern: "--[^\\n]*", IsRegex: true, LineNumber: 96, Alias: ""},
		grammartools.TokenDefinition{Name: "BLOCK_COMMENT", Pattern: "\\x2f\\*([^*]|\\*[^\\x2f])*\\*\\x2f", IsRegex: true, LineNumber: 97, Alias: ""},
	},
}
