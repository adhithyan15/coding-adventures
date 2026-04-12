package ls00

// capabilities.go — BuildCapabilities, SemanticTokenLegend, and encodeSemanticTokens
//
// # What Are Capabilities?
//
// During the LSP initialize handshake, the server sends back a "capabilities"
// object telling the editor which LSP features it supports. The editor uses this
// to decide which requests to send. If a capability is absent, the editor won't
// send the corresponding requests — so no "Go to Definition" button appears
// unless definitionProvider is true.
//
// Building capabilities dynamically (based on the bridge's interface
// implementations) means the server is always honest about what it can do.
// A bridge that only has a lexer and parser gets capabilities like:
//
//   {
//     "textDocumentSync": 2,
//     "semanticTokensProvider": {...},
//     "documentSymbolProvider": true,
//     "foldingRangeProvider": true
//   }
//
// A bridge with a full symbol table also gets:
//
//   {
//     "hoverProvider": true,
//     "definitionProvider": true,
//     "referencesProvider": true,
//     ...
//   }
//
// # Semantic Token Legend
//
// Semantic tokens use a compact binary encoding. Instead of sending {"type":"keyword"}
// per token, LSP sends an integer index into a legend. The legend must be
// declared in the capabilities so the editor knows what each index means.
//
// Example legend:
//   tokenTypes:     ["namespace","type","class","enum",...,"keyword","string","number",...]
//   tokenModifiers: ["declaration","definition","readonly","static",...]
//
// A token with type index 14 and modifiers bitmask 0b0001 means:
//   type = tokenTypes[14] = "keyword", modifiers = [tokenModifiers[0]] = ["declaration"]

import "sort"

// BuildCapabilities inspects the bridge at runtime and returns the LSP
// capabilities object to include in the initialize response.
//
// Uses Go type assertions (bridge.(HoverProvider)) to check which optional
// provider interfaces the bridge implements. Only advertises capabilities
// for features the bridge actually supports.
func BuildCapabilities(bridge LanguageBridge) map[string]interface{} {
	// textDocumentSync=2 means "incremental": the editor sends only changed
	// ranges, not the full file, on every keystroke. This is far more efficient
	// for large files. We always advertise this.
	caps := map[string]interface{}{
		"textDocumentSync": 2,
	}

	// Check each optional provider interface via type assertion.
	// The syntax "bridge.(HoverProvider)" is a Go type assertion that asks:
	// "does the bridge value also implement HoverProvider?"
	// The second return value (ok) is false if the assertion fails.

	if _, ok := bridge.(HoverProvider); ok {
		caps["hoverProvider"] = true
	}

	if _, ok := bridge.(DefinitionProvider); ok {
		caps["definitionProvider"] = true
	}

	if _, ok := bridge.(ReferencesProvider); ok {
		caps["referencesProvider"] = true
	}

	if _, ok := bridge.(CompletionProvider); ok {
		// completionProvider is an object, not a boolean, because it can include
		// triggerCharacters: which chars auto-trigger completions (e.g. "." for members).
		caps["completionProvider"] = map[string]interface{}{
			"triggerCharacters": []string{" ", "."},
		}
	}

	if _, ok := bridge.(RenameProvider); ok {
		caps["renameProvider"] = true
	}

	if _, ok := bridge.(DocumentSymbolsProvider); ok {
		caps["documentSymbolProvider"] = true
	}

	if _, ok := bridge.(FoldingRangesProvider); ok {
		caps["foldingRangeProvider"] = true
	}

	if _, ok := bridge.(SignatureHelpProvider); ok {
		// signatureHelpProvider includes triggerCharacters: "(" starts a call,
		// "," moves to the next parameter.
		caps["signatureHelpProvider"] = map[string]interface{}{
			"triggerCharacters": []string{"(", ","},
		}
	}

	if _, ok := bridge.(FormatProvider); ok {
		caps["documentFormattingProvider"] = true
	}

	if _, ok := bridge.(SemanticTokensProvider); ok {
		// Semantic tokens require a legend so the editor knows what each integer
		// index in the token data maps to.
		caps["semanticTokensProvider"] = map[string]interface{}{
			"legend": SemanticTokenLegend(),
			"full":   true, // we support full-document token requests
		}
	}

	return caps
}

// SemanticTokenLegendData holds the legend arrays for semantic tokens.
// The editor uses these to decode the compact integer encoding.
type SemanticTokenLegendData struct {
	TokenTypes     []string `json:"tokenTypes"`
	TokenModifiers []string `json:"tokenModifiers"`
}

// SemanticTokenLegend returns the full legend for all supported semantic token
// types and modifiers.
//
// # Why a Fixed Legend?
//
// The legend is sent once in the capabilities response. Afterwards, each
// semantic token is encoded as an integer index into this legend rather than
// a string. This makes the per-token encoding much smaller.
//
// The ordering matters: index 0 in TokenTypes corresponds to "namespace",
// index 1 to "type", etc. These match the standard LSP token types.
func SemanticTokenLegend() SemanticTokenLegendData {
	return SemanticTokenLegendData{
		// Standard LSP token types (in the order VS Code expects them).
		// Source: https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide
		TokenTypes: []string{
			"namespace",     // 0
			"type",          // 1
			"class",         // 2
			"enum",          // 3
			"interface",     // 4
			"struct",        // 5
			"typeParameter", // 6
			"parameter",     // 7
			"variable",      // 8
			"property",      // 9
			"enumMember",    // 10
			"event",         // 11
			"function",      // 12
			"method",        // 13
			"macro",         // 14
			"keyword",       // 15
			"modifier",      // 16
			"comment",       // 17
			"string",        // 18
			"number",        // 19
			"regexp",        // 20
			"operator",      // 21
			"decorator",     // 22
		},
		// Standard LSP token modifiers (bitmask flags).
		// tokenModifier[0] = "declaration" → bit 0 (value 1)
		// tokenModifier[1] = "definition"  → bit 1 (value 2)
		// etc.
		TokenModifiers: []string{
			"declaration",   // bit 0
			"definition",    // bit 1
			"readonly",      // bit 2
			"static",        // bit 3
			"deprecated",    // bit 4
			"abstract",      // bit 5
			"async",         // bit 6
			"modification",  // bit 7
			"documentation", // bit 8
			"defaultLibrary", // bit 9
		},
	}
}

// tokenTypeIndex returns the integer index for a semantic token type string.
// Returns -1 if the type is not in the legend (the caller should skip such tokens).
func tokenTypeIndex(tokenType string) int {
	legend := SemanticTokenLegend()
	for i, t := range legend.TokenTypes {
		if t == tokenType {
			return i
		}
	}
	return -1 // unknown token type
}

// tokenModifierMask returns the bitmask for a list of modifier strings.
//
// The LSP semantic tokens encoding represents modifiers as a bitmask:
//   - "declaration" → bit 0 → value 1
//   - "definition"  → bit 1 → value 2
//   - both           → value 3 (bitwise OR)
//
// Unknown modifiers are silently ignored.
func tokenModifierMask(modifiers []string) int {
	legend := SemanticTokenLegend()
	mask := 0
	for _, mod := range modifiers {
		for i, m := range legend.TokenModifiers {
			if m == mod {
				mask |= (1 << i)
				break
			}
		}
	}
	return mask
}

// EncodeSemanticTokens converts a slice of SemanticToken values to the LSP
// compact integer encoding.
//
// # The LSP Semantic Token Encoding
//
// LSP encodes semantic tokens as a flat array of integers, grouped in 5-tuples:
//
//   [deltaLine, deltaStartChar, length, tokenTypeIndex, tokenModifierBitmask, ...]
//
// Where "delta" means: the difference from the PREVIOUS token's position.
// This delta encoding makes most values small (often 0 or 1), which compresses
// well and is efficient to parse.
//
// Example: three tokens on different lines:
//
//   Token A: line=0, char=0, len=3, type="keyword",  modifiers=[]
//   Token B: line=0, char=4, len=5, type="function", modifiers=["declaration"]
//   Token C: line=1, char=0, len=8, type="variable", modifiers=[]
//
// Encoded as:
//   [0, 0, 3, 15, 0,   // A: deltaLine=0, deltaChar=0 (first token uses absolute)
//    0, 4, 5, 12, 1,   // B: deltaLine=0, deltaChar=4 (same line, 4 chars later)
//    1, 0, 8,  8, 0]   // C: deltaLine=1, deltaChar=0 (next line, reset char offset)
//
// Note: when deltaLine > 0, deltaStartChar is relative to column 0 of the new line
// (i.e., absolute for that line). When deltaLine == 0, deltaStartChar is relative
// to the previous token's start character.
func EncodeSemanticTokens(tokens []SemanticToken) []int {
	if len(tokens) == 0 {
		return []int{}
	}

	// Sort by (line, character) ascending. The delta encoding requires tokens
	// to be in document order — otherwise the deltas would be negative.
	sorted := make([]SemanticToken, len(tokens))
	copy(sorted, tokens)
	sort.Slice(sorted, func(i, j int) bool {
		if sorted[i].Line != sorted[j].Line {
			return sorted[i].Line < sorted[j].Line
		}
		return sorted[i].Character < sorted[j].Character
	})

	data := make([]int, 0, len(sorted)*5)
	prevLine := 0
	prevChar := 0

	for _, tok := range sorted {
		typeIdx := tokenTypeIndex(tok.TokenType)
		if typeIdx == -1 {
			// Unknown token type — skip it. The client wouldn't know what to do
			// with an index outside the legend anyway.
			continue
		}

		deltaLine := tok.Line - prevLine
		var deltaChar int
		if deltaLine == 0 {
			// Same line: character offset is relative to previous token.
			deltaChar = tok.Character - prevChar
		} else {
			// Different line: character offset is absolute (relative to line start).
			deltaChar = tok.Character
		}

		modMask := tokenModifierMask(tok.Modifiers)

		data = append(data, deltaLine, deltaChar, tok.Length, typeIdx, modMask)

		prevLine = tok.Line
		prevChar = tok.Character
	}

	return data
}
