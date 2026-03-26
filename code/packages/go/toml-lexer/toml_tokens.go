// AUTO-GENERATED FILE - DO NOT EDIT
package tomllexer

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var TomlTokens = &grammartools.TokenGrammar{
	Version: 1,
	CaseInsensitive: false,
	CaseSensitive: true,
	EscapeMode: "none",
	Definitions: []grammartools.TokenDefinition{
		grammartools.TokenDefinition{Name: "ML_BASIC_STRING", Pattern: "\"\"\"([^\\\\]|\\\\(.|\\n)|\\n)*?\"\"\"", IsRegex: true, LineNumber: 60, Alias: ""},
		grammartools.TokenDefinition{Name: "ML_LITERAL_STRING", Pattern: "'''[\\s\\S]*?'''", IsRegex: true, LineNumber: 61, Alias: ""},
		grammartools.TokenDefinition{Name: "BASIC_STRING", Pattern: "\"([^\"\\\\\\n]|\\\\.)*\"", IsRegex: true, LineNumber: 70, Alias: ""},
		grammartools.TokenDefinition{Name: "LITERAL_STRING", Pattern: "'[^'\\n]*'", IsRegex: true, LineNumber: 71, Alias: ""},
		grammartools.TokenDefinition{Name: "OFFSET_DATETIME", Pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?(Z|[+-]\\d{2}:\\d{2})", IsRegex: true, LineNumber: 91, Alias: ""},
		grammartools.TokenDefinition{Name: "LOCAL_DATETIME", Pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?", IsRegex: true, LineNumber: 92, Alias: ""},
		grammartools.TokenDefinition{Name: "LOCAL_DATE", Pattern: "\\d{4}-\\d{2}-\\d{2}", IsRegex: true, LineNumber: 93, Alias: ""},
		grammartools.TokenDefinition{Name: "LOCAL_TIME", Pattern: "\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?", IsRegex: true, LineNumber: 94, Alias: ""},
		grammartools.TokenDefinition{Name: "FLOAT_SPECIAL", Pattern: "[+-]?(inf|nan)", IsRegex: true, LineNumber: 109, Alias: "FLOAT"},
		grammartools.TokenDefinition{Name: "FLOAT_EXP", Pattern: "[+-]?([0-9](_?[0-9])*)(\\.[0-9](_?[0-9])*)?[eE][+-]?[0-9](_?[0-9])*", IsRegex: true, LineNumber: 110, Alias: "FLOAT"},
		grammartools.TokenDefinition{Name: "FLOAT_DEC", Pattern: "[+-]?([0-9](_?[0-9])*)\\.([0-9](_?[0-9])*)", IsRegex: true, LineNumber: 111, Alias: "FLOAT"},
		grammartools.TokenDefinition{Name: "HEX_INTEGER", Pattern: "0x[0-9a-fA-F](_?[0-9a-fA-F])*", IsRegex: true, LineNumber: 123, Alias: "INTEGER"},
		grammartools.TokenDefinition{Name: "OCT_INTEGER", Pattern: "0o[0-7](_?[0-7])*", IsRegex: true, LineNumber: 124, Alias: "INTEGER"},
		grammartools.TokenDefinition{Name: "BIN_INTEGER", Pattern: "0b[01](_?[01])*", IsRegex: true, LineNumber: 125, Alias: "INTEGER"},
		grammartools.TokenDefinition{Name: "INTEGER", Pattern: "[+-]?[0-9](_?[0-9])*", IsRegex: true, LineNumber: 126, Alias: ""},
		grammartools.TokenDefinition{Name: "TRUE", Pattern: "true", IsRegex: false, LineNumber: 137, Alias: ""},
		grammartools.TokenDefinition{Name: "FALSE", Pattern: "false", IsRegex: false, LineNumber: 138, Alias: ""},
		grammartools.TokenDefinition{Name: "BARE_KEY", Pattern: "[A-Za-z0-9_-]+", IsRegex: true, LineNumber: 152, Alias: ""},
		grammartools.TokenDefinition{Name: "EQUALS", Pattern: "=", IsRegex: false, LineNumber: 162, Alias: ""},
		grammartools.TokenDefinition{Name: "DOT", Pattern: ".", IsRegex: false, LineNumber: 163, Alias: ""},
		grammartools.TokenDefinition{Name: "COMMA", Pattern: ",", IsRegex: false, LineNumber: 164, Alias: ""},
		grammartools.TokenDefinition{Name: "LBRACKET", Pattern: "[", IsRegex: false, LineNumber: 165, Alias: ""},
		grammartools.TokenDefinition{Name: "RBRACKET", Pattern: "]", IsRegex: false, LineNumber: 166, Alias: ""},
		grammartools.TokenDefinition{Name: "LBRACE", Pattern: "{", IsRegex: false, LineNumber: 167, Alias: ""},
		grammartools.TokenDefinition{Name: "RBRACE", Pattern: "}", IsRegex: false, LineNumber: 168, Alias: ""},
	},
	SkipDefinitions: []grammartools.TokenDefinition{
		grammartools.TokenDefinition{Name: "COMMENT", Pattern: "#[^\\n]*", IsRegex: true, LineNumber: 28, Alias: ""},
		grammartools.TokenDefinition{Name: "WHITESPACE", Pattern: "[ \\t]+", IsRegex: true, LineNumber: 29, Alias: ""},
	},
}
