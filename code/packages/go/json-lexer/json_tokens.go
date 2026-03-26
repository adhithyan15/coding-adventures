// AUTO-GENERATED FILE - DO NOT EDIT
package jsonlexer

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var JsonTokens = &grammartools.TokenGrammar{
	Version: 1,
	CaseInsensitive: false,
	CaseSensitive: true,
	Definitions: []grammartools.TokenDefinition{
		grammartools.TokenDefinition{Name: "STRING", Pattern: "\"([^\"\\\\]|\\\\[\"\\\\\\x2fbfnrt]|\\\\u[0-9a-fA-F]{4})*\"", IsRegex: true, LineNumber: 25, Alias: ""},
		grammartools.TokenDefinition{Name: "NUMBER", Pattern: "-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?", IsRegex: true, LineNumber: 31, Alias: ""},
		grammartools.TokenDefinition{Name: "TRUE", Pattern: "true", IsRegex: false, LineNumber: 35, Alias: ""},
		grammartools.TokenDefinition{Name: "FALSE", Pattern: "false", IsRegex: false, LineNumber: 36, Alias: ""},
		grammartools.TokenDefinition{Name: "NULL", Pattern: "null", IsRegex: false, LineNumber: 37, Alias: ""},
		grammartools.TokenDefinition{Name: "LBRACE", Pattern: "{", IsRegex: false, LineNumber: 43, Alias: ""},
		grammartools.TokenDefinition{Name: "RBRACE", Pattern: "}", IsRegex: false, LineNumber: 44, Alias: ""},
		grammartools.TokenDefinition{Name: "LBRACKET", Pattern: "[", IsRegex: false, LineNumber: 45, Alias: ""},
		grammartools.TokenDefinition{Name: "RBRACKET", Pattern: "]", IsRegex: false, LineNumber: 46, Alias: ""},
		grammartools.TokenDefinition{Name: "COLON", Pattern: ":", IsRegex: false, LineNumber: 47, Alias: ""},
		grammartools.TokenDefinition{Name: "COMMA", Pattern: ",", IsRegex: false, LineNumber: 48, Alias: ""},
	},
	SkipDefinitions: []grammartools.TokenDefinition{
		grammartools.TokenDefinition{Name: "WHITESPACE", Pattern: "[ \\t\\r\\n]+", IsRegex: true, LineNumber: 59, Alias: ""},
	},
}
