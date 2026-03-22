package clibuilder

// =========================================================================
// Token Classifier — categorizes argv tokens into typed token events.
// =========================================================================
//
// # What this file does
//
// Before the parser can process argv, each token must be classified into
// one of nine token types (§5.1 of the spec). The TokenClassifier reads
// an argv token and emits a TokenEvent describing what kind of thing it is.
//
// # Why classification matters
//
// The parsing algorithm in Phase 2 is driven by the modal state machine.
// That machine can only react to well-defined events — not raw strings.
// The TokenClassifier bridges the gap: it turns raw strings into typed
// events that the state machine understands.
//
// # Longest-match-first disambiguation (§5.2)
//
// When a token begins with a single `-` followed by two or more characters,
// multiple interpretations could be valid:
//
//   - "-cp"  might be SHORT_FLAG('c') + SHORT_FLAG('p') (stacked)
//   - "-cp"  might be SINGLE_DASH_LONG("cp") if "-cp" is a declared SDL flag
//   - "-classpath" should be SINGLE_DASH_LONG("classpath") not 9 stacked chars
//
// The rule is: try single_dash_long match first (longest match wins). If
// that fails, try short flag interpretation. If that fails, stacked flags.
//
// # The nine token types
//
//   END_OF_FLAGS           "--"
//   LONG_FLAG              "--name"
//   LONG_FLAG_WITH_VALUE   "--name=value"
//   SINGLE_DASH_LONG       "-classpath" (declared single_dash_long flag)
//   SHORT_FLAG             "-x"
//   SHORT_FLAG_WITH_VALUE  "-xVALUE"
//   STACKED_FLAGS          "-lah"
//   POSITIONAL             any non-flag token
//   UNKNOWN_FLAG           "-x" where x is not a known flag

import "strings"

// TokenKind is the type of a classified token.
type TokenKind string

const (
	// TokenEndOfFlags: the "--" sentinel that ends flag scanning.
	TokenEndOfFlags TokenKind = "end_of_flags"

	// TokenLongFlag: "--name" — a long flag without an inline value.
	// The flag may be boolean (no value follows) or value-taking (the
	// next argv token is the value).
	TokenLongFlag TokenKind = "long_flag"

	// TokenLongFlagWithValue: "--name=value" — flag and value in one token.
	TokenLongFlagWithValue TokenKind = "long_flag_with_value"

	// TokenSingleDashLong: "-classpath" — a multi-char single-dash flag
	// declared via the `single_dash_long` field.
	TokenSingleDashLong TokenKind = "single_dash_long"

	// TokenShortFlag: "-x" — a single short flag character with no inline value.
	TokenShortFlag TokenKind = "short_flag"

	// TokenShortFlagWithValue: "-xVALUE" — short flag with inline value.
	// Used when a non-boolean short flag is followed immediately by its value.
	TokenShortFlagWithValue TokenKind = "short_flag_with_value"

	// TokenStackedFlags: "-lah" — multiple boolean short flags in one token.
	// The last may optionally be non-boolean (its value is the next token).
	TokenStackedFlags TokenKind = "stacked_flags"

	// TokenPositional: any token that is not a flag — subcommand names,
	// file paths, URLs, bare words.
	TokenPositional TokenKind = "positional"

	// TokenUnknownFlag: "-x" or "--xyz" where x/xyz matches no known flag.
	// Triggers an unknown_flag parse error with a fuzzy suggestion.
	TokenUnknownFlag TokenKind = "unknown_flag"
)

// TokenEvent is the result of classifying one argv token.
//
// The Kind field identifies which of the nine token types this is.
// The other fields carry the parsed data for that kind:
//
//   - Name:   flag name (long, single_dash_long, or short char)
//   - Value:  inline value (for WITH_VALUE kinds)
//   - Chars:  list of short flag characters (for STACKED_FLAGS)
//   - Raw:    the original token string (always set)
type TokenEvent struct {
	Kind  TokenKind
	Name  string   // flag name or short char
	Value string   // inline value (for WITH_VALUE kinds)
	Chars []string // stacked flag characters
	Raw   string   // the original token, for error messages
}

// TokenClassifier classifies argv tokens into typed events.
//
// It is constructed with the active flag set for the current command scope.
// The active flags determine which short flags, long flags, and
// single_dash_long flags are recognized.
//
// Construct with NewTokenClassifier. The zero value is not usable.
type TokenClassifier struct {
	// Indexes for O(1) lookups during classification.
	longFlags       map[string]map[string]any // long name → flag def
	shortFlags      map[string]map[string]any // single char → flag def
	singleDashLongs map[string]map[string]any // sdl name → flag def

	// All SDL names sorted longest-first for greedy prefix matching.
	// (Not needed with exact match, but kept for correctness.)
	sdlNames []string
}

// NewTokenClassifier constructs a TokenClassifier from a list of active
// flag definitions (already combined: local + global + builtins).
//
// Each flag map must be the raw map[string]any from the JSON spec.
func NewTokenClassifier(activeFlags []map[string]any) *TokenClassifier {
	tc := &TokenClassifier{
		longFlags:       make(map[string]map[string]any),
		shortFlags:      make(map[string]map[string]any),
		singleDashLongs: make(map[string]map[string]any),
	}

	// First-write-wins: user-defined flags take precedence over any builtin
	// flags that share the same name (e.g. a user's -h should not be treated
	// as the builtin --help even when the builtin is also in activeFlags).
	for _, f := range activeFlags {
		if long := stringField(f, "long"); long != "" {
			if _, exists := tc.longFlags[long]; !exists {
				tc.longFlags[long] = f
			}
		}
		if short := stringField(f, "short"); short != "" {
			if _, exists := tc.shortFlags[short]; !exists {
				tc.shortFlags[short] = f
			}
		}
		if sdl := stringField(f, "single_dash_long"); sdl != "" {
			if _, exists := tc.singleDashLongs[sdl]; !exists {
				tc.singleDashLongs[sdl] = f
				tc.sdlNames = append(tc.sdlNames, sdl)
			}
		}
	}

	// Sort SDL names longest-first so greedy matching works if we ever
	// need prefix matching (currently we use exact match per §5.2 Rule 1).
	for i := 0; i < len(tc.sdlNames)-1; i++ {
		for j := i + 1; j < len(tc.sdlNames); j++ {
			if len(tc.sdlNames[j]) > len(tc.sdlNames[i]) {
				tc.sdlNames[i], tc.sdlNames[j] = tc.sdlNames[j], tc.sdlNames[i]
			}
		}
	}

	return tc
}

// Classify classifies a single argv token into a TokenEvent.
//
// The classification algorithm follows §5.1 and §5.2 of the spec.
// Call this for every token during Phase 2 scanning.
func (tc *TokenClassifier) Classify(token string) TokenEvent {
	raw := token

	// --- Special case: single dash "-" is always POSITIONAL ---
	// By convention "-" represents stdin/stdout in Unix tools.
	if token == "-" {
		return TokenEvent{Kind: TokenPositional, Name: token, Raw: raw}
	}

	// --- END_OF_FLAGS: exactly "--" ---
	if token == "--" {
		return TokenEvent{Kind: TokenEndOfFlags, Raw: raw}
	}

	// --- Long flags: token begins with "--" ---
	if strings.HasPrefix(token, "--") {
		rest := token[2:] // strip "--"
		if idx := strings.Index(rest, "="); idx >= 0 {
			// LONG_FLAG_WITH_VALUE: "--name=value"
			name := rest[:idx]
			value := rest[idx+1:]
			return TokenEvent{Kind: TokenLongFlagWithValue, Name: name, Value: value, Raw: raw}
		}
		// LONG_FLAG: "--name"
		// If the name is not in our long flag index, it is unknown.
		if _, known := tc.longFlags[rest]; known {
			return TokenEvent{Kind: TokenLongFlag, Name: rest, Raw: raw}
		}
		// Not a known long flag → unknown
		return TokenEvent{Kind: TokenUnknownFlag, Name: rest, Raw: raw}
	}

	// --- Single-dash tokens: token begins with "-" but not "--" ---
	if strings.HasPrefix(token, "-") && len(token) >= 2 {
		rest := token[1:] // strip leading "-"

		// Rule 1: try single_dash_long (longest-match-first, exact match on rest)
		if _, ok := tc.singleDashLongs[rest]; ok {
			return TokenEvent{Kind: TokenSingleDashLong, Name: rest, Raw: raw}
		}

		// Rule 2: single-character short flag
		if len(rest) >= 1 {
			firstChar := string(rest[0])
			if flagDef, ok := tc.shortFlags[firstChar]; ok {
				flagType := stringField(flagDef, "type")
				if flagType == "boolean" {
					if len(rest) == 1 {
						// Just "-x" for a boolean flag
						return TokenEvent{Kind: TokenShortFlag, Name: firstChar, Raw: raw}
					}
					// More characters follow — try stacking
					return tc.classifyStacked(rest, raw)
				}
				// Non-boolean short flag with additional characters following.
				if len(rest) == 1 {
					// "-x" alone — value is the next token
					return TokenEvent{Kind: TokenShortFlag, Name: firstChar, Raw: raw}
				}
				// "-xSTUFF": check if all remaining chars are known flags.
				// If yes, this is an invalid stacking attempt (non-boolean not last)
				// and should error. If no, the remainder is the inline value.
				allKnownFlags := true
				for _, r := range rest[1:] {
					if _, ok := tc.shortFlags[string(r)]; !ok {
						allKnownFlags = false
						break
					}
				}
				if allKnownFlags {
					return TokenEvent{Kind: TokenUnknownFlag, Name: firstChar, Raw: raw}
				}
				// "-xVALUE" — remainder is the inline value
				return TokenEvent{Kind: TokenShortFlagWithValue, Name: firstChar, Value: rest[1:], Raw: raw}
			}
		}

		// Rule 3: try stacked flags
		if len(rest) > 1 {
			return tc.classifyStacked(rest, raw)
		}

		// Rule 4: no match → unknown flag
		return TokenEvent{Kind: TokenUnknownFlag, Name: rest, Raw: raw}
	}

	// Not a flag at all → POSITIONAL
	return TokenEvent{Kind: TokenPositional, Name: token, Raw: raw}
}

// classifyStacked attempts to classify a multi-character short-flag sequence
// (e.g. "lah" from "-lah") as STACKED_FLAGS.
//
// Rules (§5.2 Rule 3):
//   - Walk each character left to right.
//   - Each character except possibly the last must be a boolean short flag.
//   - The last character may be non-boolean; its value is the next token.
//   - If any character is unknown, emit UNKNOWN_FLAG.
func (tc *TokenClassifier) classifyStacked(chars string, raw string) TokenEvent {
	runes := []rune(chars)
	result := make([]string, 0, len(runes))

	for i, r := range runes {
		ch := string(r)
		flagDef, ok := tc.shortFlags[ch]
		if !ok {
			// Unknown character in stack
			return TokenEvent{Kind: TokenUnknownFlag, Name: ch, Raw: raw}
		}
		flagType := stringField(flagDef, "type")
		if flagType != "boolean" && i < len(runes)-1 {
			// Non-boolean flag in non-last position — invalid stack.
			// Per spec: "All characters in the stack except possibly the
			// last must be boolean flags."
			return TokenEvent{Kind: TokenUnknownFlag, Name: ch, Raw: raw}
		}
		result = append(result, ch)
	}

	return TokenEvent{Kind: TokenStackedFlags, Chars: result, Raw: raw}
}

// ClassifyTraditional applies the "traditional mode" rule (§5.3):
// if the token does not start with "-" and is not a known subcommand,
// treat it as stacked short flags without a leading dash.
//
// knownSubcommands is the set of valid next-command names at this point.
// Returns TokenStackedFlags if the interpretation succeeds, or
// TokenPositional if it fails (fall back to positional).
func (tc *TokenClassifier) ClassifyTraditional(token string, knownSubcommands map[string]bool) TokenEvent {
	if strings.HasPrefix(token, "-") || knownSubcommands[token] {
		// Normal classification for flag tokens or known subcommands
		return tc.Classify(token)
	}

	// Try treating as stacked flags
	ev := tc.classifyStacked(token, token)
	if ev.Kind == TokenStackedFlags {
		return ev
	}
	// Fall back to positional
	return TokenEvent{Kind: TokenPositional, Name: token, Raw: token}
}

// KnownLongNames returns all long flag names known to this classifier.
// Used for fuzzy matching suggestions on unknown_flag errors.
func (tc *TokenClassifier) KnownLongNames() []string {
	names := make([]string, 0, len(tc.longFlags))
	for n := range tc.longFlags {
		names = append(names, "--"+n)
	}
	return names
}

// KnownShortNames returns all short flag names (with "-" prefix).
func (tc *TokenClassifier) KnownShortNames() []string {
	names := make([]string, 0, len(tc.shortFlags))
	for n := range tc.shortFlags {
		names = append(names, "-"+n)
	}
	return names
}

// LookupByLong returns the flag definition for a long flag name, or nil.
func (tc *TokenClassifier) LookupByLong(name string) map[string]any {
	return tc.longFlags[name]
}

// LookupByShort returns the flag definition for a short flag char, or nil.
func (tc *TokenClassifier) LookupByShort(char string) map[string]any {
	return tc.shortFlags[char]
}

// LookupBySDL returns the flag definition for a single_dash_long name, or nil.
func (tc *TokenClassifier) LookupBySDL(name string) map[string]any {
	return tc.singleDashLongs[name]
}
