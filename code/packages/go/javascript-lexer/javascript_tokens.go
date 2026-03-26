// AUTO-GENERATED FILE - DO NOT EDIT
package javascriptlexer

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var JavascriptTokens = &grammartools.TokenGrammar{
	Version: 1,
	CaseInsensitive: false,
	CaseSensitive: true,
	Definitions: []grammartools.TokenDefinition{
		grammartools.TokenDefinition{Name: "NAME", Pattern: "[a-zA-Z_$][a-zA-Z0-9_$]*", IsRegex: true, LineNumber: 23, Alias: ""},
		grammartools.TokenDefinition{Name: "NUMBER", Pattern: "[0-9]+", IsRegex: true, LineNumber: 24, Alias: ""},
		grammartools.TokenDefinition{Name: "STRING", Pattern: "\"([^\"\\\\]|\\\\.)*\"", IsRegex: true, LineNumber: 25, Alias: ""},
		grammartools.TokenDefinition{Name: "STRICT_EQUALS", Pattern: "===", IsRegex: false, LineNumber: 28, Alias: ""},
		grammartools.TokenDefinition{Name: "STRICT_NOT_EQUALS", Pattern: "!==", IsRegex: false, LineNumber: 29, Alias: ""},
		grammartools.TokenDefinition{Name: "EQUALS_EQUALS", Pattern: "==", IsRegex: false, LineNumber: 30, Alias: ""},
		grammartools.TokenDefinition{Name: "NOT_EQUALS", Pattern: "!=", IsRegex: false, LineNumber: 31, Alias: ""},
		grammartools.TokenDefinition{Name: "LESS_EQUALS", Pattern: "<=", IsRegex: false, LineNumber: 32, Alias: ""},
		grammartools.TokenDefinition{Name: "GREATER_EQUALS", Pattern: ">=", IsRegex: false, LineNumber: 33, Alias: ""},
		grammartools.TokenDefinition{Name: "ARROW", Pattern: "=>", IsRegex: false, LineNumber: 34, Alias: ""},
		grammartools.TokenDefinition{Name: "EQUALS", Pattern: "=", IsRegex: false, LineNumber: 37, Alias: ""},
		grammartools.TokenDefinition{Name: "PLUS", Pattern: "+", IsRegex: false, LineNumber: 38, Alias: ""},
		grammartools.TokenDefinition{Name: "MINUS", Pattern: "-", IsRegex: false, LineNumber: 39, Alias: ""},
		grammartools.TokenDefinition{Name: "STAR", Pattern: "*", IsRegex: false, LineNumber: 40, Alias: ""},
		grammartools.TokenDefinition{Name: "SLASH", Pattern: "/", IsRegex: false, LineNumber: 41, Alias: ""},
		grammartools.TokenDefinition{Name: "LESS_THAN", Pattern: "<", IsRegex: false, LineNumber: 42, Alias: ""},
		grammartools.TokenDefinition{Name: "GREATER_THAN", Pattern: ">", IsRegex: false, LineNumber: 43, Alias: ""},
		grammartools.TokenDefinition{Name: "BANG", Pattern: "!", IsRegex: false, LineNumber: 44, Alias: ""},
		grammartools.TokenDefinition{Name: "LPAREN", Pattern: "(", IsRegex: false, LineNumber: 47, Alias: ""},
		grammartools.TokenDefinition{Name: "RPAREN", Pattern: ")", IsRegex: false, LineNumber: 48, Alias: ""},
		grammartools.TokenDefinition{Name: "LBRACE", Pattern: "{", IsRegex: false, LineNumber: 49, Alias: ""},
		grammartools.TokenDefinition{Name: "RBRACE", Pattern: "}", IsRegex: false, LineNumber: 50, Alias: ""},
		grammartools.TokenDefinition{Name: "LBRACKET", Pattern: "[", IsRegex: false, LineNumber: 51, Alias: ""},
		grammartools.TokenDefinition{Name: "RBRACKET", Pattern: "]", IsRegex: false, LineNumber: 52, Alias: ""},
		grammartools.TokenDefinition{Name: "COMMA", Pattern: ",", IsRegex: false, LineNumber: 53, Alias: ""},
		grammartools.TokenDefinition{Name: "COLON", Pattern: ":", IsRegex: false, LineNumber: 54, Alias: ""},
		grammartools.TokenDefinition{Name: "SEMICOLON", Pattern: ";", IsRegex: false, LineNumber: 55, Alias: ""},
		grammartools.TokenDefinition{Name: "DOT", Pattern: ".", IsRegex: false, LineNumber: 56, Alias: ""},
	},
	Keywords: []string{"let", "const", "var", "if", "else", "while", "for", "do", "function", "return", "class", "import", "export", "from", "as", "new", "this", "typeof", "instanceof", "true", "false", "null", "undefined", },
}
