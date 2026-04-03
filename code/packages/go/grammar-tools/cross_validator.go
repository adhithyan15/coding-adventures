package grammartools

import "fmt"

// cross_validator.go — Cross-validates a .tokens grammar and a .grammar file.
//
// The whole point of having two separate grammar files is that they reference
// each other: the .grammar file uses UPPERCASE names to refer to tokens
// defined in the .tokens file. This module checks that the two files are
// consistent.
//
// Why cross-validate?
// -------------------
//
// Each file can be valid on its own but broken when used together:
//
//   - A grammar might reference SEMICOLON, but the .tokens file only
//     defines SEMI. Each file is fine individually, but the pair is broken.
//   - A .tokens file might define TILDE = "~" that no grammar rule ever
//     uses. This is not an error — it might be intentional — but it is worth
//     warning about because unused tokens add complexity without value.
//
// What we check
// -------------
//
//  1. Missing token references: Every UPPERCASE name in the grammar must
//     correspond to a token definition. If not, the generated parser will try
//     to match a token type that the lexer never produces.
//
//  2. Unused tokens: Every token defined in the .tokens file should ideally
//     be referenced somewhere in the grammar. Unused tokens suggest either a
//     typo or leftover cruft. We report these as warnings, not errors.
//
// Synthetic tokens (NEWLINE, INDENT, DEDENT, EOF) are always valid — the
// lexer produces these implicitly without needing a .tokens definition.

// CrossValidate checks that a TokenGrammar and ParserGrammar are consistent.
//
// Errors describe broken references (UPPERCASE name in grammar not defined in
// tokens). Warnings describe unused definitions (token defined but never
// referenced in grammar).
//
// Returns a list of error/warning strings. An empty list means the two
// grammars are fully consistent.
func CrossValidate(tokenGrammar *TokenGrammar, parserGrammar *ParserGrammar) []string {
	result, _ := StartNew[[]string]("grammar-tools.CrossValidate", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			var issues []string

			// Build the set of all token names the parser can reference.
			// This includes both definition names and their aliases.
			definedTokens := tokenGrammar.TokenNames()

			// Synthetic tokens are always valid — the lexer produces these
			// implicitly without needing a .tokens definition:
			//   NEWLINE — emitted at bare '\n' when skip pattern excludes newlines
			//   INDENT/DEDENT — emitted in indentation mode
			//   EOF — always emitted at end of input
			definedTokens["NEWLINE"] = true
			definedTokens["EOF"] = true
			if tokenGrammar.Mode == "indentation" {
				definedTokens["INDENT"] = true
				definedTokens["DEDENT"] = true
			}

			referencedTokens := parserGrammar.TokenReferences()

			// --- Missing token references (errors) ---
			// Every UPPERCASE name in the grammar must be defined in the tokens file.
			sortedRefs := sortedKeys(referencedTokens)
			for _, ref := range sortedRefs {
				if !definedTokens[ref] {
					issues = append(issues, fmt.Sprintf(
						"Error: Grammar references token '%s' which is not defined in the tokens file",
						ref,
					))
				}
			}

			// --- Unused tokens (warnings) ---
			// A definition is "used" if its name OR alias is referenced anywhere in
			// the grammar. Aliased definitions are typically referenced by alias
			// (e.g., STRING_DQ with alias=STRING is "used" when STRING appears in
			// the grammar). We warn about definitions that are completely unused.
			for _, defn := range tokenGrammar.Definitions {
				isUsed := referencedTokens[defn.Name]
				if defn.Alias != "" && referencedTokens[defn.Alias] {
					isUsed = true
				}

				if !isUsed {
					issues = append(issues, fmt.Sprintf(
						"Warning: Token '%s' (line %d) is defined but never used in the grammar",
						defn.Name, defn.LineNumber,
					))
				}
			}

			return rf.Generate(true, false, issues)
		}).GetResult()
	return result
}
