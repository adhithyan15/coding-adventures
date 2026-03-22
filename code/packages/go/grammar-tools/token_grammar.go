package grammartools

import (
	"fmt"
	"strings"
)

// TokenDefinition represents a single token rule from a .tokens file.
type TokenDefinition struct {
	Name       string
	Pattern    string
	IsRegex    bool
	LineNumber int
	Alias      string // Optional type alias (e.g. STRING_DQ -> STRING)
}

// TokenGrammar represents the complete contents of a parsed .tokens file.
type TokenGrammar struct {
	Definitions      []TokenDefinition
	Keywords         []string
	Mode             string            // Lexer mode (e.g. "indentation")
	EscapeMode       string            // Escape processing mode (e.g. "none" to skip escape processing)
	SkipDefinitions  []TokenDefinition // Patterns consumed without producing tokens
	ReservedKeywords []string          // Keywords that cause lex errors
}

// TokenNames returns the set of all defined token names (including aliases).
func (g *TokenGrammar) TokenNames() map[string]bool {
	names := make(map[string]bool)
	for _, d := range g.Definitions {
		names[d.Name] = true
		if d.Alias != "" {
			names[d.Alias] = true
		}
	}
	return names
}

// parseDefinition parses a single pattern with optional -> ALIAS suffix.
func parseDefinition(patternPart, namePart string, lineNumber int) (TokenDefinition, error) {
	defn := TokenDefinition{Name: namePart, LineNumber: lineNumber}

	if strings.HasPrefix(patternPart, "/") {
		// Regex pattern -- find the closing /
		lastSlash := strings.LastIndex(patternPart, "/")
		if lastSlash == 0 {
			return defn, fmt.Errorf("Line %d: Unclosed regex pattern for token %q", lineNumber, namePart)
		}
		defn.Pattern = patternPart[1:lastSlash]
		defn.IsRegex = true
		remainder := strings.TrimSpace(patternPart[lastSlash+1:])

		if defn.Pattern == "" {
			return defn, fmt.Errorf("Line %d: Empty regex pattern for token %q", lineNumber, namePart)
		}

		if strings.HasPrefix(remainder, "->") {
			alias := strings.TrimSpace(remainder[2:])
			if alias == "" {
				return defn, fmt.Errorf("Line %d: Missing alias after '->' for token %q", lineNumber, namePart)
			}
			defn.Alias = alias
		} else if remainder != "" {
			return defn, fmt.Errorf("Line %d: Unexpected text after pattern for token %q: %q", lineNumber, namePart, remainder)
		}

	} else if strings.HasPrefix(patternPart, "\"") {
		// Literal pattern -- find the closing "
		closeQuote := strings.Index(patternPart[1:], "\"")
		if closeQuote == -1 {
			return defn, fmt.Errorf("Line %d: Unclosed literal pattern for token %q", lineNumber, namePart)
		}
		defn.Pattern = patternPart[1 : closeQuote+1]
		defn.IsRegex = false
		remainder := strings.TrimSpace(patternPart[closeQuote+2:])

		if defn.Pattern == "" {
			return defn, fmt.Errorf("Line %d: Empty literal pattern for token %q", lineNumber, namePart)
		}

		if strings.HasPrefix(remainder, "->") {
			alias := strings.TrimSpace(remainder[2:])
			if alias == "" {
				return defn, fmt.Errorf("Line %d: Missing alias after '->' for token %q", lineNumber, namePart)
			}
			defn.Alias = alias
		} else if remainder != "" {
			return defn, fmt.Errorf("Line %d: Unexpected text after pattern for token %q: %q", lineNumber, namePart, remainder)
		}

	} else {
		return defn, fmt.Errorf("Line %d: Pattern must be /regex/ or \"literal\"", lineNumber)
	}

	return defn, nil
}

// ParseTokenGrammar parses a .tokens file into a TokenGrammar.
// Supports mode:, keywords:, reserved:, skip: sections, and -> ALIAS syntax.
func ParseTokenGrammar(source string) (*TokenGrammar, error) {
	grammar := &TokenGrammar{}
	lines := strings.Split(source, "\n")
	var currentSection string // "keywords", "reserved", "skip"

	for i, rawLine := range lines {
		lineNumber := i + 1
		line := strings.TrimRight(rawLine, " \t\r")
		stripped := strings.TrimSpace(line)

		if stripped == "" || strings.HasPrefix(stripped, "#") {
			continue
		}

		// mode: directive
		if strings.HasPrefix(stripped, "mode:") {
			modeValue := strings.TrimSpace(stripped[5:])
			if modeValue == "" {
				return nil, fmt.Errorf("Line %d: Missing value after 'mode:'", lineNumber)
			}
			grammar.Mode = modeValue
			currentSection = ""
			continue
		}

		// escapes: directive — controls how escape sequences in STRING tokens
		// are handled. "none" means the lexer strips quotes but leaves escape
		// sequences as raw text (useful for languages like TOML and CSS where
		// different string types have different escape semantics).
		if strings.HasPrefix(stripped, "escapes:") {
			escapeValue := strings.TrimSpace(stripped[8:])
			if escapeValue == "" {
				return nil, fmt.Errorf("Line %d: Missing value after 'escapes:'", lineNumber)
			}
			grammar.EscapeMode = escapeValue
			currentSection = ""
			continue
		}

		// Section headers
		if stripped == "keywords:" || stripped == "keywords :" {
			currentSection = "keywords"
			continue
		}
		if stripped == "reserved:" || stripped == "reserved :" {
			currentSection = "reserved"
			continue
		}
		if stripped == "skip:" || stripped == "skip :" {
			currentSection = "skip"
			continue
		}

		// Inside a section
		if currentSection != "" {
			if len(line) > 0 && (line[0] == ' ' || line[0] == '\t') {
				switch currentSection {
				case "keywords":
					if stripped != "" {
						grammar.Keywords = append(grammar.Keywords, stripped)
					}
				case "reserved":
					if stripped != "" {
						grammar.ReservedKeywords = append(grammar.ReservedKeywords, stripped)
					}
				case "skip":
					eqIdx := strings.Index(stripped, "=")
					if eqIdx == -1 {
						return nil, fmt.Errorf("Line %d: Expected skip pattern (NAME = pattern), got: %q", lineNumber, stripped)
					}
					skipName := strings.TrimSpace(stripped[:eqIdx])
					skipPattern := strings.TrimSpace(stripped[eqIdx+1:])
					if skipName == "" || skipPattern == "" {
						return nil, fmt.Errorf("Line %d: Incomplete skip pattern definition: %q", lineNumber, stripped)
					}
					defn, err := parseDefinition(skipPattern, skipName, lineNumber)
					if err != nil {
						return nil, err
					}
					grammar.SkipDefinitions = append(grammar.SkipDefinitions, defn)
				}
				continue
			}
			// Non-indented line -- exit section
			currentSection = ""
		}

		// Token definition
		eqIndex := strings.Index(line, "=")
		if eqIndex == -1 {
			return nil, fmt.Errorf("Line %d: Expected token definition (NAME = pattern)", lineNumber)
		}

		namePart := strings.TrimSpace(line[:eqIndex])
		patternPart := strings.TrimSpace(line[eqIndex+1:])

		if namePart == "" {
			return nil, fmt.Errorf("Line %d: Missing token name", lineNumber)
		}

		if patternPart == "" {
			return nil, fmt.Errorf("Line %d: Missing pattern after '='", lineNumber)
		}

		defn, err := parseDefinition(patternPart, namePart, lineNumber)
		if err != nil {
			return nil, err
		}
		grammar.Definitions = append(grammar.Definitions, defn)
	}
	return grammar, nil
}
