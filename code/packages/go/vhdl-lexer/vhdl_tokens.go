// AUTO-GENERATED FILE - DO NOT EDIT
package vhdllexer

import (
	grammartools "github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools"
)

var VhdlTokens = &grammartools.TokenGrammar{
	Version: 0,
	CaseInsensitive: false,
	CaseSensitive: false,
	EscapeMode: "none",
	Definitions: []grammartools.TokenDefinition{
		grammartools.TokenDefinition{Name: "STRING", Pattern: "\"([^\"]|\"\")*\"", IsRegex: true, LineNumber: 63, Alias: ""},
		grammartools.TokenDefinition{Name: "BIT_STRING", Pattern: "[bBoOxXdD]\"[0-9a-fA-F_]+\"", IsRegex: true, LineNumber: 82, Alias: ""},
		grammartools.TokenDefinition{Name: "CHAR_LITERAL", Pattern: "'[^']'", IsRegex: true, LineNumber: 100, Alias: ""},
		grammartools.TokenDefinition{Name: "BASED_LITERAL", Pattern: "[0-9]+#[0-9a-fA-F_]+(\\.[0-9a-fA-F_]+)?#([eE][+-]?[0-9_]+)?", IsRegex: true, LineNumber: 116, Alias: ""},
		grammartools.TokenDefinition{Name: "REAL_NUMBER", Pattern: "[0-9][0-9_]*\\.[0-9_]+([eE][+-]?[0-9_]+)?", IsRegex: true, LineNumber: 120, Alias: ""},
		grammartools.TokenDefinition{Name: "NUMBER", Pattern: "[0-9][0-9_]*", IsRegex: true, LineNumber: 124, Alias: ""},
		grammartools.TokenDefinition{Name: "EXTENDED_IDENT", Pattern: "\\\\[^\\\\]+\\\\", IsRegex: true, LineNumber: 143, Alias: ""},
		grammartools.TokenDefinition{Name: "NAME", Pattern: "[a-zA-Z][a-zA-Z0-9_]*", IsRegex: true, LineNumber: 144, Alias: ""},
		grammartools.TokenDefinition{Name: "VAR_ASSIGN", Pattern: ":=", IsRegex: false, LineNumber: 165, Alias: ""},
		grammartools.TokenDefinition{Name: "LESS_EQUALS", Pattern: "<=", IsRegex: false, LineNumber: 166, Alias: ""},
		grammartools.TokenDefinition{Name: "GREATER_EQUALS", Pattern: ">=", IsRegex: false, LineNumber: 167, Alias: ""},
		grammartools.TokenDefinition{Name: "ARROW", Pattern: "=>", IsRegex: false, LineNumber: 168, Alias: ""},
		grammartools.TokenDefinition{Name: "NOT_EQUALS", Pattern: "/=", IsRegex: false, LineNumber: 169, Alias: ""},
		grammartools.TokenDefinition{Name: "POWER", Pattern: "**", IsRegex: false, LineNumber: 170, Alias: ""},
		grammartools.TokenDefinition{Name: "BOX", Pattern: "<>", IsRegex: false, LineNumber: 171, Alias: ""},
		grammartools.TokenDefinition{Name: "PLUS", Pattern: "+", IsRegex: false, LineNumber: 184, Alias: ""},
		grammartools.TokenDefinition{Name: "MINUS", Pattern: "-", IsRegex: false, LineNumber: 185, Alias: ""},
		grammartools.TokenDefinition{Name: "STAR", Pattern: "*", IsRegex: false, LineNumber: 186, Alias: ""},
		grammartools.TokenDefinition{Name: "SLASH", Pattern: "/", IsRegex: false, LineNumber: 187, Alias: ""},
		grammartools.TokenDefinition{Name: "AMPERSAND", Pattern: "&", IsRegex: false, LineNumber: 188, Alias: ""},
		grammartools.TokenDefinition{Name: "LESS_THAN", Pattern: "<", IsRegex: false, LineNumber: 189, Alias: ""},
		grammartools.TokenDefinition{Name: "GREATER_THAN", Pattern: ">", IsRegex: false, LineNumber: 190, Alias: ""},
		grammartools.TokenDefinition{Name: "EQUALS", Pattern: "=", IsRegex: false, LineNumber: 191, Alias: ""},
		grammartools.TokenDefinition{Name: "TICK", Pattern: "'", IsRegex: false, LineNumber: 192, Alias: ""},
		grammartools.TokenDefinition{Name: "PIPE", Pattern: "|", IsRegex: false, LineNumber: 193, Alias: ""},
		grammartools.TokenDefinition{Name: "LPAREN", Pattern: "(", IsRegex: false, LineNumber: 199, Alias: ""},
		grammartools.TokenDefinition{Name: "RPAREN", Pattern: ")", IsRegex: false, LineNumber: 200, Alias: ""},
		grammartools.TokenDefinition{Name: "LBRACKET", Pattern: "[", IsRegex: false, LineNumber: 201, Alias: ""},
		grammartools.TokenDefinition{Name: "RBRACKET", Pattern: "]", IsRegex: false, LineNumber: 202, Alias: ""},
		grammartools.TokenDefinition{Name: "SEMICOLON", Pattern: ";", IsRegex: false, LineNumber: 203, Alias: ""},
		grammartools.TokenDefinition{Name: "COMMA", Pattern: ",", IsRegex: false, LineNumber: 204, Alias: ""},
		grammartools.TokenDefinition{Name: "DOT", Pattern: ".", IsRegex: false, LineNumber: 205, Alias: ""},
		grammartools.TokenDefinition{Name: "COLON", Pattern: ":", IsRegex: false, LineNumber: 206, Alias: ""},
	},
	Keywords: []string{"abs", "access", "after", "alias", "all", "and", "architecture", "array", "assert", "attribute", "begin", "block", "body", "buffer", "bus", "case", "component", "configuration", "constant", "disconnect", "downto", "else", "elsif", "end", "entity", "exit", "file", "for", "function", "generate", "generic", "group", "guarded", "if", "impure", "in", "inout", "is", "label", "library", "linkage", "literal", "loop", "map", "mod", "nand", "new", "next", "nor", "not", "null", "of", "on", "open", "or", "others", "out", "package", "port", "postponed", "procedure", "process", "pure", "range", "record", "register", "reject", "rem", "report", "return", "rol", "ror", "select", "severity", "signal", "shared", "sla", "sll", "sra", "srl", "subtype", "then", "to", "transport", "type", "unaffected", "units", "until", "use", "variable", "wait", "when", "while", "with", "xnor", "xor", },
	SkipDefinitions: []grammartools.TokenDefinition{
		grammartools.TokenDefinition{Name: "COMMENT", Pattern: "--[^\\n]*", IsRegex: true, LineNumber: 50, Alias: ""},
		grammartools.TokenDefinition{Name: "WHITESPACE", Pattern: "[ \\t\\r\\n]+", IsRegex: true, LineNumber: 51, Alias: ""},
	},
}
