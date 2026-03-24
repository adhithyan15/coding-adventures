package grammartools

import (
	"fmt"
	"regexp"
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

// PatternGroup represents a named set of token definitions that are active
// together during context-sensitive lexing.
//
// When this group is at the top of the lexer's group stack, only these
// patterns are tried during token matching. Skip patterns are global and
// always tried regardless of the active group.
//
// Pattern groups enable context-sensitive lexing. For example, an XML lexer
// defines a "tag" group with patterns for attribute names, equals signs, and
// attribute values. These patterns are only active inside tags — the callback
// pushes the "tag" group when ``<`` is matched and pops it when ``>`` is
// matched.
//
// Fields:
//   - Name: The group name, e.g. "tag" or "cdata". Must be a lowercase
//     identifier matching [a-z_][a-z0-9_]*.
//   - Definitions: Ordered list of token definitions in this group.
//     Order matters (first-match-wins), just like the top-level
//     definitions list.
type PatternGroup struct {
	Name        string
	Definitions []TokenDefinition
}

// TokenGrammar represents the complete contents of a parsed .tokens file.
type TokenGrammar struct {
	Definitions      []TokenDefinition
	Keywords         []string
	Mode             string                   // Lexer mode (e.g. "indentation")
	EscapeMode       string                   // Escape processing mode (e.g. "none" to skip escape processing)
	SkipDefinitions  []TokenDefinition        // Patterns consumed without producing tokens
	ErrorDefinitions []TokenDefinition        // Error recovery patterns tried when no normal token matches
	ReservedKeywords []string                 // Keywords that cause lex errors
	Groups           map[string]*PatternGroup // Named pattern groups for context-sensitive lexing
}

// TokenNames returns the set of all defined token names (including aliases).
//
// When a definition has an alias, both the original name and the alias are
// included. This includes names from all pattern groups, since group tokens
// can also appear in parser grammars.
//
// This is useful for cross-validation: the parser grammar references tokens
// by name, and we need to check that every referenced token actually exists.
func (g *TokenGrammar) TokenNames() map[string]bool {
	names := make(map[string]bool)

	// Collect all definitions: top-level plus all group definitions.
	allDefs := make([]TokenDefinition, 0, len(g.Definitions))
	allDefs = append(allDefs, g.Definitions...)
	for _, group := range g.Groups {
		allDefs = append(allDefs, group.Definitions...)
	}

	for _, d := range allDefs {
		names[d.Name] = true
		if d.Alias != "" {
			names[d.Alias] = true
		}
	}
	return names
}

// EffectiveTokenNames returns the set of token names as the parser will see
// them.
//
// For definitions with aliases, this returns the alias (not the definition
// name), because that is what the lexer will emit and what the parser grammar
// references. For definitions without aliases, this returns the definition
// name. Includes names from all pattern groups.
func (g *TokenGrammar) EffectiveTokenNames() map[string]bool {
	names := make(map[string]bool)

	// Collect all definitions: top-level plus all group definitions.
	allDefs := make([]TokenDefinition, 0, len(g.Definitions))
	allDefs = append(allDefs, g.Definitions...)
	for _, group := range g.Groups {
		allDefs = append(allDefs, group.Definitions...)
	}

	for _, d := range allDefs {
		if d.Alias != "" {
			names[d.Alias] = true
		} else {
			names[d.Name] = true
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

// groupNameRe matches valid group names: lowercase identifiers like "tag"
// or "cdata_section". Group names must start with a lowercase letter or
// underscore, followed by lowercase letters, digits, or underscores.
var groupNameRe = regexp.MustCompile(`^[a-z_][a-z0-9_]*$`)

// reservedGroupNames is the set of names that cannot be used as pattern
// group names. These names have special meaning in the .tokens format:
//   - "default" — the implicit group for top-level definitions
//   - "skip" — the skip: section
//   - "keywords" — the keywords: section
//   - "reserved" — the reserved: section
//   - "errors" — the errors: section
var reservedGroupNames = map[string]bool{
	"default":  true,
	"skip":     true,
	"keywords": true,
	"reserved": true,
	"errors":   true,
}

// ParseTokenGrammar parses a .tokens file into a TokenGrammar.
//
// Supports mode:, keywords:, reserved:, skip:, and group NAME: sections,
// as well as -> ALIAS syntax on token definitions.
//
// Pattern groups are declared with ``group NAME:`` where NAME is a lowercase
// identifier. All subsequent indented lines belong to that group. Groups
// enable context-sensitive lexing: the lexer maintains a stack of active
// groups and only tries patterns from the group on top of the stack.
func ParseTokenGrammar(source string) (*TokenGrammar, error) {
	grammar := &TokenGrammar{
		Groups: make(map[string]*PatternGroup),
	}
	lines := strings.Split(source, "\n")
	var currentSection string // "keywords", "reserved", "skip", or "group:NAME"

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

		// Group headers — ``group NAME:`` declares a named pattern group.
		// All subsequent indented lines are token definitions belonging to
		// that group. The group name must be a lowercase identifier, must
		// not be a reserved name, and must not duplicate an existing group.
		if strings.HasPrefix(stripped, "group ") && strings.HasSuffix(stripped, ":") {
			groupName := strings.TrimSpace(stripped[6 : len(stripped)-1])
			if groupName == "" {
				return nil, fmt.Errorf("Line %d: Missing group name after 'group'", lineNumber)
			}
			if !groupNameRe.MatchString(groupName) {
				return nil, fmt.Errorf(
					"Line %d: Invalid group name: %q (must be a lowercase identifier like 'tag' or 'cdata')",
					lineNumber, groupName,
				)
			}
			if reservedGroupNames[groupName] {
				return nil, fmt.Errorf(
					"Line %d: Reserved group name: %q (cannot use default, errors, keywords, reserved, skip)",
					lineNumber, groupName,
				)
			}
			if _, exists := grammar.Groups[groupName]; exists {
				return nil, fmt.Errorf("Line %d: Duplicate group name: %q", lineNumber, groupName)
			}
			grammar.Groups[groupName] = &PatternGroup{
				Name:        groupName,
				Definitions: nil,
			}
			currentSection = "group:" + groupName
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
		if stripped == "errors:" || stripped == "errors :" {
			currentSection = "errors"
			continue
		}

		// Inside a section
		if currentSection != "" {
			if len(line) > 0 && (line[0] == ' ' || line[0] == '\t') {
				// Dispatch based on the current section. Group sections use
				// the "group:NAME" format to carry the group name.
				switch {
				case currentSection == "keywords":
					if stripped != "" {
						grammar.Keywords = append(grammar.Keywords, stripped)
					}
				case currentSection == "reserved":
					if stripped != "" {
						grammar.ReservedKeywords = append(grammar.ReservedKeywords, stripped)
					}
				case currentSection == "skip":
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
				case currentSection == "errors":
					// Errors section contains token definitions for error recovery —
					// patterns tried as a fallback when no normal token matches
					// (e.g., BAD_STRING for unclosed strings in CSS).
					eqIdx := strings.Index(stripped, "=")
					if eqIdx == -1 {
						return nil, fmt.Errorf("Line %d: Expected error pattern (NAME = pattern), got: %q", lineNumber, stripped)
					}
					errName := strings.TrimSpace(stripped[:eqIdx])
					errPattern := strings.TrimSpace(stripped[eqIdx+1:])
					if errName == "" || errPattern == "" {
						return nil, fmt.Errorf("Line %d: Incomplete error pattern definition: %q", lineNumber, stripped)
					}
					defn, err := parseDefinition(errPattern, errName, lineNumber)
					if err != nil {
						return nil, err
					}
					grammar.ErrorDefinitions = append(grammar.ErrorDefinitions, defn)
				case strings.HasPrefix(currentSection, "group:"):
					// Group section — parse token definitions just like the
					// skip: section, but append to the named group instead.
					groupName := currentSection[6:]
					eqIdx := strings.Index(stripped, "=")
					if eqIdx == -1 {
						return nil, fmt.Errorf(
							"Line %d: Expected token definition in group '%s' (NAME = pattern), got: %q",
							lineNumber, groupName, stripped,
						)
					}
					gName := strings.TrimSpace(stripped[:eqIdx])
					gPattern := strings.TrimSpace(stripped[eqIdx+1:])
					if gName == "" || gPattern == "" {
						return nil, fmt.Errorf(
							"Line %d: Incomplete definition in group '%s': %q",
							lineNumber, groupName, stripped,
						)
					}
					defn, err := parseDefinition(gPattern, gName, lineNumber)
					if err != nil {
						return nil, err
					}
					group := grammar.Groups[groupName]
					group.Definitions = append(group.Definitions, defn)
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

// ---------------------------------------------------------------------------
// Validator
// ---------------------------------------------------------------------------

// validateDefinitions checks a list of token definitions for common problems.
// The label parameter describes the context (e.g. "token", "skip pattern",
// "group 'tag' token") for error messages.
//
// Checks performed:
//   - Duplicate token names within the list
//   - Empty patterns (should be caught during parsing, but double-checked)
//   - Invalid regex patterns (compiled with Go's regexp package)
//   - Non-UPPER_CASE token names (convention violation)
//   - Non-UPPER_CASE alias names (convention violation)
func validateDefinitions(definitions []TokenDefinition, label string) []string {
	var issues []string
	seenNames := make(map[string]int)

	for _, defn := range definitions {
		// --- Duplicate check ---
		if firstLine, exists := seenNames[defn.Name]; exists {
			issues = append(issues, fmt.Sprintf(
				"Line %d: Duplicate %s name '%s' (first defined on line %d)",
				defn.LineNumber, label, defn.Name, firstLine,
			))
		} else {
			seenNames[defn.Name] = defn.LineNumber
		}

		// --- Empty pattern check ---
		if defn.Pattern == "" {
			issues = append(issues, fmt.Sprintf(
				"Line %d: Empty pattern for %s '%s'",
				defn.LineNumber, label, defn.Name,
			))
		}

		// --- Invalid regex check ---
		if defn.IsRegex {
			if _, err := regexp.Compile(defn.Pattern); err != nil {
				issues = append(issues, fmt.Sprintf(
					"Line %d: Invalid regex for %s '%s': %v",
					defn.LineNumber, label, defn.Name, err,
				))
			}
		}

		// --- Naming convention check ---
		if defn.Name != strings.ToUpper(defn.Name) {
			issues = append(issues, fmt.Sprintf(
				"Line %d: Token name '%s' should be UPPER_CASE",
				defn.LineNumber, defn.Name,
			))
		}

		// --- Alias convention check ---
		if defn.Alias != "" && defn.Alias != strings.ToUpper(defn.Alias) {
			issues = append(issues, fmt.Sprintf(
				"Line %d: Alias '%s' for token '%s' should be UPPER_CASE",
				defn.LineNumber, defn.Alias, defn.Name,
			))
		}
	}

	return issues
}

// ValidateTokenGrammar checks a parsed TokenGrammar for common problems.
//
// This is a *lint* pass, not a parse pass — the grammar has already been
// parsed successfully. We look for semantic issues that would cause problems
// downstream:
//
//   - Duplicate token names within each definition list
//   - Invalid regex patterns (cannot be compiled by Go's regexp)
//   - Empty patterns
//   - Non-UPPER_CASE token names and aliases
//   - Invalid lexer mode (only "indentation" is supported)
//   - Invalid escape mode (only "none" is supported)
//   - Invalid group names
//   - Empty pattern groups (no definitions)
//   - Definition issues within groups (same checks as top-level)
func ValidateTokenGrammar(grammar *TokenGrammar) []string {
	var issues []string

	// Validate regular definitions
	issues = append(issues, validateDefinitions(grammar.Definitions, "token")...)

	// Validate skip definitions
	issues = append(issues, validateDefinitions(grammar.SkipDefinitions, "skip pattern")...)

	// Validate error definitions
	issues = append(issues, validateDefinitions(grammar.ErrorDefinitions, "error pattern")...)

	// Validate mode
	if grammar.Mode != "" && grammar.Mode != "indentation" {
		issues = append(issues, fmt.Sprintf(
			"Unknown lexer mode '%s' (only 'indentation' is supported)",
			grammar.Mode,
		))
	}

	// Validate escape mode
	if grammar.EscapeMode != "" && grammar.EscapeMode != "none" {
		issues = append(issues, fmt.Sprintf(
			"Unknown escape mode '%s' (only 'none' is supported)",
			grammar.EscapeMode,
		))
	}

	// Validate pattern groups
	for groupName, group := range grammar.Groups {
		// Group name format
		if !groupNameRe.MatchString(groupName) {
			issues = append(issues, fmt.Sprintf(
				"Invalid group name '%s' (must be a lowercase identifier)",
				groupName,
			))
		}

		// Empty group warning
		if len(group.Definitions) == 0 {
			issues = append(issues, fmt.Sprintf(
				"Empty pattern group '%s' (has no token definitions)",
				groupName,
			))
		}

		// Validate definitions within the group
		issues = append(issues, validateDefinitions(
			group.Definitions,
			fmt.Sprintf("group '%s' token", groupName),
		)...)
	}

	return issues
}
