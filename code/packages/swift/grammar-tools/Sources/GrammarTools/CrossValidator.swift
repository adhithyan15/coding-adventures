// ============================================================================
// CrossValidator.swift — Cross-validates a .tokens file and a .grammar file.
// ============================================================================
//
// The whole point of having two separate grammar files is that they reference
// each other: the .grammar file uses UPPERCASE names to refer to tokens
// defined in the .tokens file. This module checks that the two files are
// consistent.
//
// ============================================================================
// WHY CROSS-VALIDATE?
// ============================================================================
//
// Each file can be valid on its own but broken when used together:
//
// - A grammar might reference `SEMICOLON`, but the .tokens file only
//   defines `SEMI`. Each file is fine individually, but the pair is broken.
//
// - A .tokens file might define `TILDE = "~"` that no grammar rule ever
//   uses. This is not an error -- it might be intentional -- but it's worth
//   warning about because unused tokens add complexity without value.
//
// This is analogous to how a C compiler checks that every function you call
// is actually declared (and vice versa, warns about unused functions).
//
// ============================================================================
// WHAT WE CHECK
// ============================================================================
//
// 1. **Missing token references** (errors): Every UPPERCASE name in the
//    grammar must correspond to a token definition. If not, the generated
//    parser will try to match a token type that the lexer never produces.
//
// 2. **Unused tokens** (warnings): Every token defined in the .tokens file
//    should ideally be referenced somewhere in the grammar. Unused tokens
//    suggest either a typo or leftover cruft.
//
// ============================================================================

import Foundation

/// Cross-validate a token grammar and a parser grammar.
///
/// Checks that every UPPERCASE name referenced in the parser grammar
/// exists in the token grammar, and warns about tokens that are defined
/// but never used.
///
/// - Parameters:
///   - tokenGrammar: A parsed .tokens file.
///   - parserGrammar: A parsed .grammar file.
/// - Returns: A list of error/warning strings. Errors describe broken
///     references; warnings describe unused definitions. Empty means the
///     two grammars are fully consistent.
///
public func crossValidate(
    tokenGrammar: TokenGrammar,
    parserGrammar: ParserGrammar
) -> [String] {
    var issues: [String] = []

    let definedTokens = tokenNames(tokenGrammar)
    let referencedTokens = tokenReferences(parserGrammar)

    // Implicit tokens that are always available
    var implicitTokens: Set<String> = ["EOF", "NEWLINE"]

    // In indentation mode, INDENT/DEDENT are also implicit
    if tokenGrammar.mode == "indentation" {
        implicitTokens.insert("INDENT")
        implicitTokens.insert("DEDENT")
    }

    // --- Missing token references (errors) ---
    for ref in referencedTokens.sorted() {
        if !definedTokens.contains(ref) && !implicitTokens.contains(ref) {
            issues.append(
                "Error: Grammar references token '\(ref)' which is not " +
                "defined in the tokens file"
            )
        }
    }

    // --- Unused tokens (warnings) ---
    for defn in tokenGrammar.definitions {
        let isUsed = referencedTokens.contains(defn.name) ||
            (defn.alias != nil && referencedTokens.contains(defn.alias!))
        if !isUsed {
            issues.append(
                "Warning: Token '\(defn.name)' (line \(defn.lineNumber)) " +
                "is defined but never used in the grammar"
            )
        }
    }

    return issues
}
