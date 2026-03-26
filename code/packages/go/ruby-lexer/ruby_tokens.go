// AUTO-GENERATED FILE - DO NOT EDIT
package rubylexer

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var RubyTokens = &grammartools.TokenGrammar{
	Version: 1,
	CaseInsensitive: false,
	CaseSensitive: true,
	Definitions: []grammartools.TokenDefinition{
		grammartools.TokenDefinition{Name: "NAME", Pattern: "[a-zA-Z_][a-zA-Z0-9_]*", IsRegex: true, LineNumber: 23, Alias: ""},
		grammartools.TokenDefinition{Name: "NUMBER", Pattern: "[0-9]+", IsRegex: true, LineNumber: 24, Alias: ""},
		grammartools.TokenDefinition{Name: "STRING", Pattern: "\"([^\"\\\\]|\\\\.)*\"", IsRegex: true, LineNumber: 25, Alias: ""},
		grammartools.TokenDefinition{Name: "EQUALS_EQUALS", Pattern: "==", IsRegex: false, LineNumber: 28, Alias: ""},
		grammartools.TokenDefinition{Name: "DOT_DOT", Pattern: "..", IsRegex: false, LineNumber: 29, Alias: ""},
		grammartools.TokenDefinition{Name: "HASH_ROCKET", Pattern: "=>", IsRegex: false, LineNumber: 30, Alias: ""},
		grammartools.TokenDefinition{Name: "NOT_EQUALS", Pattern: "!=", IsRegex: false, LineNumber: 31, Alias: ""},
		grammartools.TokenDefinition{Name: "LESS_EQUALS", Pattern: "<=", IsRegex: false, LineNumber: 32, Alias: ""},
		grammartools.TokenDefinition{Name: "GREATER_EQUALS", Pattern: ">=", IsRegex: false, LineNumber: 33, Alias: ""},
		grammartools.TokenDefinition{Name: "EQUALS", Pattern: "=", IsRegex: false, LineNumber: 36, Alias: ""},
		grammartools.TokenDefinition{Name: "PLUS", Pattern: "+", IsRegex: false, LineNumber: 37, Alias: ""},
		grammartools.TokenDefinition{Name: "MINUS", Pattern: "-", IsRegex: false, LineNumber: 38, Alias: ""},
		grammartools.TokenDefinition{Name: "STAR", Pattern: "*", IsRegex: false, LineNumber: 39, Alias: ""},
		grammartools.TokenDefinition{Name: "SLASH", Pattern: "/", IsRegex: false, LineNumber: 40, Alias: ""},
		grammartools.TokenDefinition{Name: "LESS_THAN", Pattern: "<", IsRegex: false, LineNumber: 43, Alias: ""},
		grammartools.TokenDefinition{Name: "GREATER_THAN", Pattern: ">", IsRegex: false, LineNumber: 44, Alias: ""},
		grammartools.TokenDefinition{Name: "LPAREN", Pattern: "(", IsRegex: false, LineNumber: 47, Alias: ""},
		grammartools.TokenDefinition{Name: "RPAREN", Pattern: ")", IsRegex: false, LineNumber: 48, Alias: ""},
		grammartools.TokenDefinition{Name: "COMMA", Pattern: ",", IsRegex: false, LineNumber: 49, Alias: ""},
		grammartools.TokenDefinition{Name: "COLON", Pattern: ":", IsRegex: false, LineNumber: 50, Alias: ""},
	},
	Keywords: []string{"if", "else", "elsif", "end", "while", "for", "do", "def", "return", "class", "module", "require", "puts", "true", "false", "nil", "and", "or", "not", "then", "unless", "until", "yield", "begin", "rescue", "ensure", },
}
