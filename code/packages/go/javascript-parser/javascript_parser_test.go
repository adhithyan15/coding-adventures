package javascriptparser

import (
	"testing"
)

// ─────────────────────────────────────────────────────────────────────────────
// Generic-grammar tests (version = "")
//
// These tests use the default grammar, which is the superset of all supported
// ECMAScript versions. They confirm that the version-agnostic code path still
// works exactly as it did in v0.1.0.
// ─────────────────────────────────────────────────────────────────────────────

func TestParseJavascript(t *testing.T) {
	source := "let x = 1 + 2;"
	program, err := ParseJavascript(source, "")
	if err != nil {
		t.Fatalf("Failed to parse JavaScript code: %v", err)
	}

	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Versioned-grammar tests
//
// Each test uses a specific ECMAScript version grammar. We only assert that
// the parse does not error and produces a "program" root node, because the
// exact grammar rules differ per version. The goal is to confirm that the
// file-routing logic reaches the correct grammar file and that the lexer and
// parser grammars stay in sync for each version.
// ─────────────────────────────────────────────────────────────────────────────

// TestParseJavascriptVersion_es1 verifies ECMAScript 1 grammar loading.
// ES1 (June 1997): the first standard; `var` was the only declaration keyword.
func TestParseJavascriptVersion_es1(t *testing.T) {
	program, err := ParseJavascript("var x = 1;", "es1")
	if err != nil {
		t.Fatalf("Failed to parse with es1: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavascriptVersion_es3 verifies ECMAScript 3 grammar loading.
// ES3 (December 1999): added try/catch, regular expressions, and do-while.
func TestParseJavascriptVersion_es3(t *testing.T) {
	program, err := ParseJavascript("var x = 1;", "es3")
	if err != nil {
		t.Fatalf("Failed to parse with es3: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavascriptVersion_es5 verifies ECMAScript 5 grammar loading.
// ES5 (December 2009): added strict mode and JSON built-ins.
func TestParseJavascriptVersion_es5(t *testing.T) {
	program, err := ParseJavascript("var x = 1;", "es5")
	if err != nil {
		t.Fatalf("Failed to parse with es5: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavascriptVersion_es2015 verifies ECMAScript 2015 grammar loading.
// ES2015 (June 2015): classes, modules, arrow functions, let/const.
func TestParseJavascriptVersion_es2015(t *testing.T) {
	program, err := ParseJavascript("const x = 1 + 2;", "es2015")
	if err != nil {
		t.Fatalf("Failed to parse with es2015: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavascriptVersion_es2016 verifies ECMAScript 2016 grammar loading.
// ES2016 (June 2016): exponentiation operator and Array.includes.
func TestParseJavascriptVersion_es2016(t *testing.T) {
	program, err := ParseJavascript("const x = 1 + 2;", "es2016")
	if err != nil {
		t.Fatalf("Failed to parse with es2016: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavascriptVersion_es2017 verifies ECMAScript 2017 grammar loading.
// ES2017 (June 2017): async/await, Object.entries/values.
func TestParseJavascriptVersion_es2017(t *testing.T) {
	program, err := ParseJavascript("const x = 1 + 2;", "es2017")
	if err != nil {
		t.Fatalf("Failed to parse with es2017: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavascriptVersion_es2018 verifies ECMAScript 2018 grammar loading.
// ES2018 (June 2018): rest/spread for objects, async iteration.
func TestParseJavascriptVersion_es2018(t *testing.T) {
	program, err := ParseJavascript("const x = 1 + 2;", "es2018")
	if err != nil {
		t.Fatalf("Failed to parse with es2018: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavascriptVersion_es2019 verifies ECMAScript 2019 grammar loading.
// ES2019 (June 2019): Array.flat/flatMap, optional catch binding.
func TestParseJavascriptVersion_es2019(t *testing.T) {
	program, err := ParseJavascript("const x = 1 + 2;", "es2019")
	if err != nil {
		t.Fatalf("Failed to parse with es2019: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavascriptVersion_es2020 verifies ECMAScript 2020 grammar loading.
// ES2020 (June 2020): BigInt, optional chaining, nullish coalescing.
func TestParseJavascriptVersion_es2020(t *testing.T) {
	program, err := ParseJavascript("const x = 1 + 2;", "es2020")
	if err != nil {
		t.Fatalf("Failed to parse with es2020: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavascriptVersion_es2021 verifies ECMAScript 2021 grammar loading.
// ES2021 (June 2021): logical assignment operators, numeric separators.
func TestParseJavascriptVersion_es2021(t *testing.T) {
	program, err := ParseJavascript("const x = 1 + 2;", "es2021")
	if err != nil {
		t.Fatalf("Failed to parse with es2021: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavascriptVersion_es2022 verifies ECMAScript 2022 grammar loading.
// ES2022 (June 2022): class fields, static blocks, Array.at(), Object.hasOwn.
func TestParseJavascriptVersion_es2022(t *testing.T) {
	program, err := ParseJavascript("const x = 1 + 2;", "es2022")
	if err != nil {
		t.Fatalf("Failed to parse with es2022: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavascriptVersion_es2023 verifies ECMAScript 2023 grammar loading.
// ES2023 (June 2023): Array.findLast, Symbols as WeakMap keys.
func TestParseJavascriptVersion_es2023(t *testing.T) {
	program, err := ParseJavascript("const x = 1 + 2;", "es2023")
	if err != nil {
		t.Fatalf("Failed to parse with es2023: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavascriptVersion_es2024 verifies ECMAScript 2024 grammar loading.
// ES2024 (June 2024): Promise.withResolvers, Object.groupBy.
func TestParseJavascriptVersion_es2024(t *testing.T) {
	program, err := ParseJavascript("const x = 1 + 2;", "es2024")
	if err != nil {
		t.Fatalf("Failed to parse with es2024: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// TestParseJavascriptVersion_es2025 verifies ECMAScript 2025 grammar loading.
// ES2025 (June 2025): the latest stable ECMAScript standard.
func TestParseJavascriptVersion_es2025(t *testing.T) {
	program, err := ParseJavascript("const x = 1 + 2;", "es2025")
	if err != nil {
		t.Fatalf("Failed to parse with es2025: %v", err)
	}
	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Error-path tests
// ─────────────────────────────────────────────────────────────────────────────

// TestParseJavascriptUnknownVersion confirms that an unrecognised version
// string returns an error rather than silently falling back to another grammar.
func TestParseJavascriptUnknownVersion(t *testing.T) {
	_, err := ParseJavascript("let x = 1;", "es99")
	if err == nil {
		t.Fatal("Expected error for unknown version, got nil")
	}
}
