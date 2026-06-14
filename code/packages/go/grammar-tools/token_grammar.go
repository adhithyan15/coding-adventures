package grammartools

import (
	"fmt"
	"regexp"
	"strconv"
	"strings"
)

// MaxTransitions is a safety cap on the number of declarative mode-transition
// rules a grammar may declare (F10). Grammars are trusted build-time
// artifacts, but this bounds the table so a malformed file cannot produce
// an unreasonably large rule list (mirrors MAX_TRANSITIONS in the Rust,
// Python, Ruby, and TypeScript ports).
const MaxTransitions = 4096

// TransitionAction is one action applied to the lexer's mode stack when a
// ModeTransition fires (F10).
//
// Kind is the action; Target is the mode/group name for "set_mode" and
// "push", and "" for "pop"/"enable_skip"/"disable_skip".
//
// Kind strings use UNDERSCORES ("set_mode", "enable_skip") to match the
// Python, Ruby, and TypeScript ports; the surface DSL uses hyphens
// ("set-mode", "enable-skip") and the parser converts.
type TransitionAction struct {
	// Kind is one of "set_mode", "push", "pop", "enable_skip", "disable_skip".
	Kind string
	// Target is the mode/group name for "set_mode" and "push"; empty otherwise.
	Target string
}

// ModeTransition is one declarative mode-transition rule (F10), parsed from a
// "transitions:" line of the form:
//
//	on TOKENS [in MODE] -> ACTION [, ACTION ...]
//
// where TOKENS is a bare name, a parenthesised set "(A | B | C)", or a
// keyword-value guard "KEYWORD=\"value\"", and each ACTION is one of
// set-mode/push/pop/enable-skip/disable-skip.
type ModeTransition struct {
	// OnTokens is the list of token type-names that trigger this rule.
	// "KEYWORD" is the special name for promoted keyword tokens with a value
	// guard stored in OnValue.
	OnTokens []string
	// OnValue is the optional keyword-value guard (e.g. "return").
	OnValue string
	// InMode is the optional active-mode guard; "" means "in any mode".
	InMode string
	// Actions is the ordered list of TransitionActions applied when the rule fires.
	Actions []TransitionAction
	// LineNumber is the 1-based line where the rule appeared.
	LineNumber int
}

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
//
// Magic comments at the top of the file configure the grammar:
//
//	# @version 1           — pins to format version 1 (default: 0, meaning latest)
//	# @case_insensitive true  — keywords matched case-insensitively (default: false)
type TokenGrammar struct {
	Version          int                      // From # @version N magic comment (0 = latest)
	CaseInsensitive  bool                     // From # @case_insensitive true magic comment
	Definitions      []TokenDefinition
	Keywords         []string
	Mode             string                   // Lexer mode (e.g. "indentation")
	EscapeMode       string                   // Escape processing mode (e.g. "none" to skip escape processing)
	SkipDefinitions  []TokenDefinition        // Patterns consumed without producing tokens
	ErrorDefinitions []TokenDefinition        // Error recovery patterns tried when no normal token matches
	ReservedKeywords []string                 // Keywords that cause lex errors
	Groups           map[string]*PatternGroup // Named pattern groups for context-sensitive lexing
	LayoutKeywords   []string                 // Keywords that introduce Haskell-style layout contexts
	CaseSensitive    bool                     // Whether the lexer should match case-sensitively (default true)

	// ContextKeywords are context-sensitive keywords — words that are keywords
	// in some syntactic positions but identifiers in others.
	//
	// These are emitted as NAME tokens with the TokenContextKeyword flag set,
	// leaving the final keyword-vs-identifier decision to the language-specific
	// parser or callback.
	//
	// Examples: JavaScript's `async`, `await`, `yield`, `get`, `set`.
	ContextKeywords []string

	// SoftKeywords are words that act as keywords only in specific syntactic
	// contexts, remaining ordinary identifiers everywhere else.
	//
	// Unlike ContextKeywords (which set a flag on the token), soft keywords
	// produce plain NAME tokens with NO special flag. The lexer is completely
	// unaware of their keyword status — the parser handles disambiguation
	// entirely based on syntactic position.
	//
	// This distinction matters because:
	//   - ContextKeywords: lexer hints to parser ("this NAME might be special")
	//   - SoftKeywords: lexer ignores them completely, parser owns the decision
	//
	// Examples:
	//   Python 3.10+: `match`, `case`, `_` (only keywords inside match statements)
	//   Python 3.12+: `type` (only a keyword in `type X = ...` statements)
	//
	// A `soft_keywords:` section in a .tokens file populates this field.
	SoftKeywords []string

	// StartMode is the name of the initial lexer mode for this grammar (F10).
	// When set, the lexer begins in this mode instead of "default". Corresponds
	// to the `start_mode:` directive. "" means "start in default mode".
	StartMode string

	// Transitions is the ordered list of declarative mode-transition rules (F10).
	// The lexer applies the first matching rule after emitting each token. An
	// empty slice means no declarative transitions are declared (equivalent to
	// the pre-F10 behaviour). Populated by the `transitions:` section.
	Transitions []ModeTransition
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
	result, _ := StartNew[map[string]bool]("grammar-tools.TokenGrammar.TokenNames", nil,
		func(op *Operation[map[string]bool], rf *ResultFactory[map[string]bool]) *OperationResult[map[string]bool] {
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
			return rf.Generate(true, false, names)
		}).GetResult()
	return result
}

// EffectiveTokenNames returns the set of token names as the parser will see
// them.
//
// For definitions with aliases, this returns the alias (not the definition
// name), because that is what the lexer will emit and what the parser grammar
// references. For definitions without aliases, this returns the definition
// name. Includes names from all pattern groups.
func (g *TokenGrammar) EffectiveTokenNames() map[string]bool {
	result, _ := StartNew[map[string]bool]("grammar-tools.TokenGrammar.EffectiveTokenNames", nil,
		func(op *Operation[map[string]bool], rf *ResultFactory[map[string]bool]) *OperationResult[map[string]bool] {
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
			return rf.Generate(true, false, names)
		}).GetResult()
	return result
}

// findClosingSlash scans a /pattern/ string starting at index 1 and returns
// the index of the closing /. It skips escaped characters (\x) and does not
// treat / inside [...] character classes as the closing delimiter.
// Returns -1 if no closing slash is found.
func findClosingSlash(s string) int {
	inBracket := false
	for i := 1; i < len(s); i++ {
		ch := s[i]
		if ch == '\\' {
			i++ // skip escaped character
			continue
		}
		if ch == '[' && !inBracket {
			inBracket = true
		} else if ch == ']' && inBracket {
			inBracket = false
		} else if ch == '/' && !inBracket {
			return i
		}
	}
	// Fallback: if bracket-aware scan found nothing (e.g. unclosed [),
	// try the last / as a best-effort parse.
	if last := strings.LastIndex(s, "/"); last > 0 {
		return last
	}
	return -1
}

// parseDefinition parses a single pattern with optional -> ALIAS suffix.
func parseDefinition(patternPart, namePart string, lineNumber int) (TokenDefinition, error) {
	defn := TokenDefinition{Name: namePart, LineNumber: lineNumber}

	if strings.HasPrefix(patternPart, "/") {
		// Regex pattern — find the closing / by scanning character-by-character.
		// We track bracket depth so that / inside [...] character classes is
		// not mistaken for the closing delimiter. We also skip escaped chars.
		lastSlash := findClosingSlash(patternPart)
		if lastSlash == -1 {
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

// magicCommentRe matches magic comment lines of the form: # @key value
// The first capture group is the key (e.g. "version", "case_insensitive").
// The second capture group is the value (e.g. "1", "true").
// Unknown keys are silently ignored for forward compatibility.
var magicCommentRe = regexp.MustCompile(`^#\s*@(\w+)\s*(.*)$`)

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
	"default":          true,
	"skip":             true,
	"keywords":         true,
	"reserved":         true,
	"errors":           true,
	"layout_keywords":  true,
	"context_keywords": true,
	"soft_keywords":    true,
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
// splitInGuard splits a transition rule head on a top-level " in " mode guard,
// ignoring any " in " that appears inside a parenthesised token set (F10).
//
// For example:
//
//	"(A | B) in div"   → ("(A | B)", "div")
//	"NAME"             → ("NAME",     "")
//	"A | B in default" → ("A | B", "default")   // no parens, splits on first " in "
func splitInGuard(head string) (tokens, inMode string) {
	depth := 0
	for i := 0; i < len(head); i++ {
		switch head[i] {
		case '(':
			depth++
		case ')':
			depth--
		case ' ':
			if depth == 0 && strings.HasPrefix(head[i:], " in ") {
				return head[:i], strings.TrimSpace(head[i+4:])
			}
		}
	}
	return head, ""
}

// parseTransition parses one indented line from the `transitions:` section
// into a ModeTransition (F10).
//
// Expected form:  on TOKENS [in MODE] -> ACTION [, ACTION ...]
//
// TOKENS is either a bare token type-name, a parenthesised list "(A | B | C)",
// or a keyword-value guard "KEYWORD=\"value\"".  Each ACTION is one of:
//
//	set-mode MODE  |  push GROUP  |  pop  |  enable-skip  |  disable-skip
func parseTransition(line string, lineNumber int) (ModeTransition, error) {
	arrowIdx := strings.Index(line, "->")
	if arrowIdx == -1 {
		return ModeTransition{}, fmt.Errorf("Line %d: Transition rule missing '->': %q", lineNumber, line)
	}
	head := strings.TrimSpace(line[:arrowIdx])
	actionStr := strings.TrimSpace(line[arrowIdx+2:])
	if actionStr == "" {
		return ModeTransition{}, fmt.Errorf("Line %d: Transition rule has no actions after '->': %q", lineNumber, line)
	}

	if !strings.HasPrefix(head, "on ") {
		return ModeTransition{}, fmt.Errorf("Line %d: Transition rule must start with 'on ': %q", lineNumber, line)
	}
	head = strings.TrimSpace(head[3:])

	tokensRaw, inMode := splitInGuard(head)
	tokensRaw = strings.TrimSpace(tokensRaw)

	// Unwrap optional parens around the token set.
	inner := tokensRaw
	if strings.HasPrefix(inner, "(") && strings.HasSuffix(inner, ")") {
		inner = inner[1 : len(inner)-1]
	}

	var onTokens []string
	var onValue string
	for _, raw := range strings.Split(inner, "|") {
		item := strings.TrimSpace(raw)
		if item == "" {
			continue
		}
		if eqIdx := strings.Index(item, "="); eqIdx != -1 {
			// KEYWORD="value" guard.
			namePart := strings.TrimSpace(item[:eqIdx])
			valuePart := strings.TrimSpace(item[eqIdx+1:])
			if strings.HasPrefix(valuePart, `"`) && strings.HasSuffix(valuePart, `"`) {
				valuePart = valuePart[1 : len(valuePart)-1]
			}
			onTokens = append(onTokens, namePart)
			onValue = valuePart
		} else {
			onTokens = append(onTokens, item)
		}
	}
	if len(onTokens) == 0 {
		return ModeTransition{}, fmt.Errorf("Line %d: Transition rule has no trigger tokens: %q", lineNumber, line)
	}

	var actions []TransitionAction
	for _, raw := range strings.Split(actionStr, ",") {
		act := strings.TrimSpace(raw)
		if act == "" {
			continue
		}
		switch {
		case strings.HasPrefix(act, "set-mode "):
			actions = append(actions, TransitionAction{Kind: "set_mode", Target: strings.TrimSpace(act[9:])})
		case strings.HasPrefix(act, "push "):
			actions = append(actions, TransitionAction{Kind: "push", Target: strings.TrimSpace(act[5:])})
		case act == "pop":
			actions = append(actions, TransitionAction{Kind: "pop"})
		case act == "enable-skip":
			actions = append(actions, TransitionAction{Kind: "enable_skip"})
		case act == "disable-skip":
			actions = append(actions, TransitionAction{Kind: "disable_skip"})
		default:
			return ModeTransition{}, fmt.Errorf(
				"Line %d: Unknown transition action %q (expected set-mode/push/pop/enable-skip/disable-skip)",
				lineNumber, act,
			)
		}
	}
	if len(actions) == 0 {
		return ModeTransition{}, fmt.Errorf("Line %d: Transition rule has no valid actions: %q", lineNumber, line)
	}

	return ModeTransition{
		OnTokens:   onTokens,
		OnValue:    onValue,
		InMode:     inMode,
		Actions:    actions,
		LineNumber: lineNumber,
	}, nil
}

func ParseTokenGrammar(source string) (*TokenGrammar, error) {
	return StartNew[*TokenGrammar]("grammar-tools.ParseTokenGrammar", nil,
		func(op *Operation[*TokenGrammar], rf *ResultFactory[*TokenGrammar]) *OperationResult[*TokenGrammar] {
			op.AddProperty("sourceLen", len(source))
			result, err := parseTokenGrammarImpl(source)
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, result)
		}).GetResult()
}

func parseTokenGrammarImpl(source string) (*TokenGrammar, error) {
	grammar := &TokenGrammar{
		Groups:        make(map[string]*PatternGroup),
		CaseSensitive: true,
	}
	lines := strings.Split(source, "\n")
	var currentSection string // "keywords", "reserved", "skip", or "group:NAME"

	for i, rawLine := range lines {
		lineNumber := i + 1
		line := strings.TrimRight(rawLine, " \t\r")
		stripped := strings.TrimSpace(line)

		if stripped == "" {
			continue
		}
		if strings.HasPrefix(stripped, "#") {
			// Magic comments: # @key value — configure the grammar.
			// Unknown keys are silently ignored for forward compatibility.
			if m := magicCommentRe.FindStringSubmatch(stripped); m != nil {
				key, value := m[1], strings.TrimSpace(m[2])
				switch key {
				case "version":
					if n, err := strconv.Atoi(value); err == nil {
						grammar.Version = n
					}
				case "case_insensitive":
					grammar.CaseInsensitive = (value == "true")
				}
			}
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

		// case_sensitive: directive — controls whether the lexer should match
		// case-sensitively. When false, the lexer lowercases the source text
		// before matching. Defaults to true when not specified.
		if strings.HasPrefix(stripped, "case_sensitive:") {
			csValue := strings.TrimSpace(stripped[15:])
			csLower := strings.ToLower(csValue)
			if csLower != "true" && csLower != "false" {
				return nil, fmt.Errorf("Line %d: Invalid value for 'case_sensitive:': %q (expected 'true' or 'false')", lineNumber, csValue)
			}
			grammar.CaseSensitive = csLower == "true"
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
		if stripped == "context_keywords:" || stripped == "context_keywords :" {
			currentSection = "context_keywords"
			continue
		}
		if stripped == "layout_keywords:" || stripped == "layout_keywords :" {
			currentSection = "layout_keywords"
			continue
		}
		if stripped == "soft_keywords:" || stripped == "soft_keywords :" {
			currentSection = "soft_keywords"
			continue
		}

		// start_mode: directive (F10) — sets the initial lexer mode.
		// Form: `start_mode: MODE` (a single mode name following the colon).
		if strings.HasPrefix(stripped, "start_mode:") || strings.HasPrefix(stripped, "start_mode :") {
			colonIdx := strings.Index(stripped, ":")
			smValue := strings.TrimSpace(stripped[colonIdx+1:])
			if smValue == "" {
				return nil, fmt.Errorf("Line %d: Missing mode name after 'start_mode:'", lineNumber)
			}
			grammar.StartMode = smValue
			currentSection = ""
			continue
		}

		// transitions: section (F10) — each indented line is a full rule of
		// the form `on TOKENS [in MODE] -> ACTION [, ACTION ...]`.
		if stripped == "transitions:" || stripped == "transitions :" {
			currentSection = "transitions"
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
				case currentSection == "context_keywords":
					if stripped != "" {
						grammar.ContextKeywords = append(grammar.ContextKeywords, stripped)
					}
				case currentSection == "layout_keywords":
					if stripped != "" {
						grammar.LayoutKeywords = append(grammar.LayoutKeywords, stripped)
					}
				case currentSection == "soft_keywords":
					if stripped != "" {
						grammar.SoftKeywords = append(grammar.SoftKeywords, stripped)
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

				case currentSection == "transitions":
					// Each indented line in a transitions: section is a full rule;
					// delegate entirely to parseTransition (no NAME = pattern form).
					rule, err := parseTransition(stripped, lineNumber)
					if err != nil {
						return nil, err
					}
					grammar.Transitions = append(grammar.Transitions, rule)
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
//   - Invalid lexer mode (only "indentation" and "layout" are supported)
//   - Invalid escape mode (only "none" is supported)
//   - Invalid group names
//   - Empty pattern groups (no definitions)
//   - Definition issues within groups (same checks as top-level)
func ValidateTokenGrammar(grammar *TokenGrammar) []string {
	result, _ := StartNew[[]string]("grammar-tools.ValidateTokenGrammar", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			return rf.Generate(true, false, validateTokenGrammarImpl(grammar))
		}).GetResult()
	return result
}

func validateTokenGrammarImpl(grammar *TokenGrammar) []string {
	var issues []string

	// Validate regular definitions
	issues = append(issues, validateDefinitions(grammar.Definitions, "token")...)

	// Validate skip definitions
	issues = append(issues, validateDefinitions(grammar.SkipDefinitions, "skip pattern")...)

	// Validate error definitions
	issues = append(issues, validateDefinitions(grammar.ErrorDefinitions, "error pattern")...)

	// Validate mode
	if grammar.Mode != "" && grammar.Mode != "indentation" && grammar.Mode != "layout" {
		issues = append(issues, fmt.Sprintf(
			"Unknown lexer mode '%s' (only 'indentation' and 'layout' are supported)",
			grammar.Mode,
		))
	}
	if grammar.Mode == "layout" && len(grammar.LayoutKeywords) == 0 {
		issues = append(issues, "Layout mode requires a non-empty layout_keywords section")
	}

	// Validate escape mode
	if grammar.EscapeMode != "" && grammar.EscapeMode != "none" {
		issues = append(issues, fmt.Sprintf(
			"Unknown escape mode '%s' (only 'none' is supported)",
			grammar.EscapeMode,
		))
	}

	// F10: validate declarative lexer modes. A mode is valid if it is
	// "default" or a declared group name. Undefined targets are rejected so
	// the lexer never silently falls back to the wrong group.
	modeExists := func(name string) bool {
		if name == "default" {
			return true
		}
		_, ok := grammar.Groups[name]
		return ok
	}
	if grammar.StartMode != "" && !modeExists(grammar.StartMode) {
		issues = append(issues, fmt.Sprintf(
			"start_mode '%s' is not 'default' or a declared group",
			grammar.StartMode,
		))
	}
	if len(grammar.Transitions) > MaxTransitions {
		issues = append(issues, fmt.Sprintf(
			"Too many transition rules (%d, max %d)",
			len(grammar.Transitions), MaxTransitions,
		))
	}
	for _, rule := range grammar.Transitions {
		if rule.InMode != "" && !modeExists(rule.InMode) {
			issues = append(issues, fmt.Sprintf(
				"Transition guard 'in %s' (line %d) names an undeclared mode",
				rule.InMode, rule.LineNumber,
			))
		}
		for _, action := range rule.Actions {
			if (action.Kind == "set_mode" || action.Kind == "push") && action.Target != "" && !modeExists(action.Target) {
				issues = append(issues, fmt.Sprintf(
					"Transition action targets undeclared mode '%s' (line %d)",
					action.Target, rule.LineNumber,
				))
			}
		}
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
