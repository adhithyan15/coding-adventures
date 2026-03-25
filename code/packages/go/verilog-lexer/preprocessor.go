// Package veriloglexer provides a grammar-driven tokenizer for Verilog HDL source code.
//
// This file implements the Verilog preprocessor, which processes directives
// that start with a backtick (`) and operates on raw source text before the
// lexer sees it.
//
// Supported Directives
// --------------------
//
// `define NAME value
//     Define a text macro. Every subsequent occurrence of `NAME in the source
//     is replaced with value.
//
// `define NAME(a, b) expression
//     Define a parameterized macro. Occurrences of `NAME(x, y) are replaced
//     with expression where a and b are substituted with x and y.
//
// `undef NAME
//     Remove a previously defined macro.
//
// `ifdef NAME / `ifndef NAME
//     Conditional compilation. If NAME is defined (or not defined), include
//     the following lines. Otherwise, skip them.
//
// `else
//     Flip the current conditional — include lines that were being skipped,
//     and vice versa.
//
// `endif
//     End a conditional block.
//
// `include "filename"
//     File inclusion. Currently stubbed — emits a comment and removes the
//     directive. Full file inclusion requires a file resolver callback.
//
// `timescale unit/precision
//     Time unit specification. Stripped from the source (no semantic meaning
//     for synthesis or parsing).
//
// Design as a Stepping Stone
// --------------------------
//
// This preprocessor is intentionally structured as a clean, extractable
// module. When C preprocessor support is added later, the core logic
// (macro table, condition stack, text substitution) can be extracted into
// a shared preprocessor package. The Verilog preprocessor would then
// become a thin configuration of that shared engine.
//
// The key differences between Verilog and C preprocessors:
//
//     +------------------------+-------------------+-------------------+
//     | Feature                | Verilog           | C                 |
//     +------------------------+-------------------+-------------------+
//     | Directive prefix       | ` (backtick)      | # (hash)          |
//     | Macro reference        | `NAME             | NAME              |
//     | Token pasting (##)     | No                | Yes               |
//     | Stringification (#)    | No                | Yes               |
//     | Variadic macros        | No                | Yes (__VA_ARGS__) |
//     | #pragma / #error       | No                | Yes               |
//     +------------------------+-------------------+-------------------+
//
// Verilog is strictly a subset of C's preprocessor capabilities.
package veriloglexer

import (
	"fmt"
	"regexp"
	"strings"
)

// ---------------------------------------------------------------------------
// MacroDef — A macro definition, either simple or parameterized
// ---------------------------------------------------------------------------
//
// Simple macro:
//
//	`define WIDTH 8
//	→ MacroDef{Name: "WIDTH", Body: "8", Params: nil}
//
// Parameterized macro:
//
//	`define MAX(a, b) ((a) > (b) ? (a) : (b))
//	→ MacroDef{Name: "MAX", Body: "((a) > (b) ? (a) : (b))",
//	           Params: []string{"a", "b"}}
type MacroDef struct {
	Name   string
	Body   string
	Params []string // nil for simple macros, non-nil for parameterized
}

// ---------------------------------------------------------------------------
// Regex patterns for directive parsing
// ---------------------------------------------------------------------------
//
// These patterns match the various preprocessor directives at the start
// of a line. Each pattern captures the relevant parts of the directive
// (macro name, parameters, body, etc.).

// defineWithParams matches: `define NAME(params) body
var defineWithParams = regexp.MustCompile(`^\s*` + "`" + `define\s+([a-zA-Z_]\w*)\(([^)]*)\)\s*(.*)`)

// defineSimple matches: `define NAME body
var defineSimple = regexp.MustCompile(`^\s*` + "`" + `define\s+([a-zA-Z_]\w*)\s*(.*)`)

// undefPattern matches: `undef NAME
var undefPattern = regexp.MustCompile(`^\s*` + "`" + `undef\s+([a-zA-Z_]\w*)`)

// ifdefPattern matches: `ifdef NAME
var ifdefPattern = regexp.MustCompile(`^\s*` + "`" + `ifdef\s+([a-zA-Z_]\w*)`)

// ifndefPattern matches: `ifndef NAME
var ifndefPattern = regexp.MustCompile(`^\s*` + "`" + `ifndef\s+([a-zA-Z_]\w*)`)

// elsePattern matches: `else
var elsePattern = regexp.MustCompile(`^\s*` + "`" + `else\b`)

// endifPattern matches: `endif
var endifPattern = regexp.MustCompile(`^\s*` + "`" + `endif\b`)

// includePattern matches: `include "filename"
var includePattern = regexp.MustCompile(`^\s*` + "`" + `include\s+"([^"]*)"`)

// timescalePattern matches: `timescale ...
var timescalePattern = regexp.MustCompile(`^\s*` + "`" + `timescale\b.*`)

// macroRef matches: `NAME (backtick followed by identifier)
var macroRef = regexp.MustCompile("`" + `([a-zA-Z_]\w*)`)

// macroCall matches: `NAME( (backtick identifier followed by open paren)
var macroCall = regexp.MustCompile("`" + `([a-zA-Z_]\w*)\(`)

// ---------------------------------------------------------------------------
// VerilogPreprocess — The main preprocessor entry point
// ---------------------------------------------------------------------------

// VerilogPreprocess processes Verilog preprocessor directives in source text.
//
// It takes raw Verilog source code and returns preprocessed text with macros
// expanded and conditionals resolved. This function is designed to be called
// before the lexer processes the source.
//
// The predefined parameter allows passing in predefined macros (name → value),
// useful for "+define+" command-line flags.
//
// Line Number Preservation: When lines are excluded by conditionals or
// stripped (timescale, include), they are replaced with empty strings.
// This preserves line numbers so that error messages from the lexer/parser
// point to the correct location in the original source.
//
// Example:
//
//	source := "`define WIDTH 8\nwire [`WIDTH-1:0] data;"
//	result := VerilogPreprocess(source)
//	// result contains: "\nwire [8-1:0] data;"
func VerilogPreprocess(source string) string {
	return VerilogPreprocessWithDefines(source, nil)
}

// VerilogPreprocessWithDefines processes Verilog preprocessor directives
// with an optional set of predefined macros.
//
// The predefined map allows external macro definitions (e.g., from command-
// line +define+ flags) to be injected before processing begins.
func VerilogPreprocessWithDefines(source string, predefined map[string]string) string {
	macros := make(map[string]*MacroDef)
	if predefined != nil {
		for name, value := range predefined {
			macros[name] = &MacroDef{Name: name, Body: value}
		}
	}

	// Condition stack: true = include current section, false = skip.
	// Starts with true (unconditional inclusion).
	conditionStack := []bool{true}

	lines := strings.Split(source, "\n")
	result := make([]string, 0, len(lines))

	for _, line := range lines {
		// Check if we're in an active (included) section.
		// All entries in the condition stack must be true.
		active := allTrue(conditionStack)

		// --- Conditional directives (always processed, even when inactive) ---
		//
		// These must be processed even in inactive sections to maintain the
		// condition stack correctly. Consider:
		//
		//   `ifdef A        ← push False
		//     `ifdef B      ← push False (nested in inactive)
		//     `endif        ← pop
		//   `endif          ← pop
		//
		// Without processing nested ifdef/endif in inactive sections, the
		// stack would get out of sync.

		if m := ifdefPattern.FindStringSubmatch(line); m != nil {
			name := m[1]
			if active {
				_, defined := macros[name]
				conditionStack = append(conditionStack, defined)
			} else {
				// Nested conditional inside inactive section — push false
				conditionStack = append(conditionStack, false)
			}
			result = append(result, "") // Preserve line number
			continue
		}

		if m := ifndefPattern.FindStringSubmatch(line); m != nil {
			name := m[1]
			if active {
				_, defined := macros[name]
				conditionStack = append(conditionStack, !defined)
			} else {
				conditionStack = append(conditionStack, false)
			}
			result = append(result, "")
			continue
		}

		if elsePattern.MatchString(line) {
			if len(conditionStack) > 1 {
				// Only flip if the parent section is active
				parentActive := allTrue(conditionStack[:len(conditionStack)-1])
				if parentActive {
					conditionStack[len(conditionStack)-1] = !conditionStack[len(conditionStack)-1]
				}
			}
			result = append(result, "")
			continue
		}

		if endifPattern.MatchString(line) {
			if len(conditionStack) > 1 {
				conditionStack = conditionStack[:len(conditionStack)-1]
			}
			result = append(result, "")
			continue
		}

		// --- Skip inactive lines ---

		if !active {
			result = append(result, "")
			continue
		}

		// --- `define ---

		if m := defineWithParams.FindStringSubmatch(line); m != nil {
			name := m[1]
			params := splitAndTrim(m[2], ",")
			body := strings.TrimSpace(m[3])
			macros[name] = &MacroDef{Name: name, Body: body, Params: params}
			result = append(result, "")
			continue
		}

		if m := defineSimple.FindStringSubmatch(line); m != nil {
			name := m[1]
			body := strings.TrimSpace(m[2])
			macros[name] = &MacroDef{Name: name, Body: body}
			result = append(result, "")
			continue
		}

		// --- `undef ---

		if m := undefPattern.FindStringSubmatch(line); m != nil {
			name := m[1]
			delete(macros, name)
			result = append(result, "")
			continue
		}

		// --- `include (stubbed) ---

		if m := includePattern.FindStringSubmatch(line); m != nil {
			filename := m[1]
			result = append(result, fmt.Sprintf("/* `include \"%s\" — not resolved */", filename))
			continue
		}

		// --- `timescale (stripped) ---

		if timescalePattern.MatchString(line) {
			result = append(result, "")
			continue
		}

		// --- Macro expansion ---

		expanded := expandMacros(line, macros)
		result = append(result, expanded)
	}

	return strings.Join(result, "\n")
}

// ---------------------------------------------------------------------------
// expandMacros — Single-pass macro expansion for one line
// ---------------------------------------------------------------------------
//
// Handles both simple macros (`WIDTH → 8) and parameterized macros
// (`MAX(a, b) → ((a) > (b) ? (a) : (b))).
//
// Expansion is single-pass to avoid infinite loops from recursive macros.

func expandMacros(line string, macros map[string]*MacroDef) string {
	pos := 0
	var parts []string

	for pos < len(line) {
		// Look for backtick-prefixed identifier
		loc := macroRef.FindStringIndex(line[pos:])
		if loc == nil {
			parts = append(parts, line[pos:])
			break
		}

		// Add text before the macro reference
		parts = append(parts, line[pos:pos+loc[0]])
		m := macroRef.FindStringSubmatch(line[pos+loc[0]:])
		name := m[1]

		macro, defined := macros[name]
		if !defined {
			// Not a defined macro — keep the reference as-is
			parts = append(parts, m[0])
			pos = pos + loc[0] + len(m[0])
			continue
		}

		if macro.Params != nil {
			// Parameterized macro — look for opening paren
			callLoc := macroCall.FindStringIndex(line[pos+loc[0]:])
			if callLoc != nil && callLoc[0] == 0 {
				callMatch := macroCall.FindStringSubmatch(line[pos+loc[0]:])
				argsStart := pos + loc[0] + len(callMatch[0])
				argsStr, endPos := extractMacroArgs(line, argsStart)
				args := splitAndTrim(argsStr, ",")

				// Substitute parameters in the macro body
				body := macro.Body
				for i, param := range macro.Params {
					if i < len(args) {
						body = strings.ReplaceAll(body, param, args[i])
					}
				}

				parts = append(parts, body)
				pos = endPos
			} else {
				// Parameterized macro referenced without args — keep as-is
				parts = append(parts, m[0])
				pos = pos + loc[0] + len(m[0])
			}
		} else {
			// Simple macro — direct text substitution
			parts = append(parts, macro.Body)
			pos = pos + loc[0] + len(m[0])
		}
	}

	return strings.Join(parts, "")
}

// ---------------------------------------------------------------------------
// extractMacroArgs — Extract macro arguments handling nested parentheses
// ---------------------------------------------------------------------------
//
// Starts after the opening paren and finds the matching closing paren,
// handling nesting so that MAX((a+b), c) correctly extracts "(a+b)" and
// "c" as two arguments.
//
// Returns (argumentString, positionAfterClosingParen).

func extractMacroArgs(text string, start int) (string, int) {
	depth := 1
	pos := start
	for pos < len(text) && depth > 0 {
		ch := text[pos]
		if ch == '(' {
			depth++
		} else if ch == ')' {
			depth--
		}
		pos++
	}
	// pos is now one past the closing paren
	return text[start : pos-1], pos
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

// allTrue returns true if every element in the slice is true.
func allTrue(slice []bool) bool {
	for _, v := range slice {
		if !v {
			return false
		}
	}
	return true
}

// splitAndTrim splits a string by sep and trims whitespace from each part.
func splitAndTrim(s string, sep string) []string {
	parts := strings.Split(s, sep)
	result := make([]string, len(parts))
	for i, p := range parts {
		result[i] = strings.TrimSpace(p)
	}
	return result
}
