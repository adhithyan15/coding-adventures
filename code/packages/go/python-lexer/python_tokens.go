// AUTO-GENERATED FILE - DO NOT EDIT
package pythonlexer

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var PythonTokens = &grammartools.TokenGrammar{
	Version: 1,
	CaseInsensitive: false,
	CaseSensitive: true,
	Definitions: []grammartools.TokenDefinition{
		grammartools.TokenDefinition{Name: "NAME", Pattern: "[a-zA-Z_][a-zA-Z0-9_]*", IsRegex: true, LineNumber: 13, Alias: ""},
		grammartools.TokenDefinition{Name: "NUMBER", Pattern: "[0-9]+", IsRegex: true, LineNumber: 14, Alias: ""},
		grammartools.TokenDefinition{Name: "STRING", Pattern: "\"([^\"\\\\]|\\\\.)*\"", IsRegex: true, LineNumber: 15, Alias: ""},
		grammartools.TokenDefinition{Name: "EQUALS_EQUALS", Pattern: "==", IsRegex: false, LineNumber: 18, Alias: ""},
		grammartools.TokenDefinition{Name: "EQUALS", Pattern: "=", IsRegex: false, LineNumber: 21, Alias: ""},
		grammartools.TokenDefinition{Name: "PLUS", Pattern: "+", IsRegex: false, LineNumber: 22, Alias: ""},
		grammartools.TokenDefinition{Name: "MINUS", Pattern: "-", IsRegex: false, LineNumber: 23, Alias: ""},
		grammartools.TokenDefinition{Name: "STAR", Pattern: "*", IsRegex: false, LineNumber: 24, Alias: ""},
		grammartools.TokenDefinition{Name: "SLASH", Pattern: "/", IsRegex: false, LineNumber: 25, Alias: ""},
		grammartools.TokenDefinition{Name: "LPAREN", Pattern: "(", IsRegex: false, LineNumber: 28, Alias: ""},
		grammartools.TokenDefinition{Name: "RPAREN", Pattern: ")", IsRegex: false, LineNumber: 29, Alias: ""},
		grammartools.TokenDefinition{Name: "COMMA", Pattern: ",", IsRegex: false, LineNumber: 30, Alias: ""},
		grammartools.TokenDefinition{Name: "COLON", Pattern: ":", IsRegex: false, LineNumber: 31, Alias: ""},
	},
	Keywords: []string{"if", "else", "elif", "while", "for", "def", "return", "class", "import", "from", "as", "True", "False", "None", },
}
