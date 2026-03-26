/**
 * cross-validator.ts — Cross-validates a .tokens file and a .grammar file.
 *
 * The whole point of having two separate grammar files is that they reference
 * each other: the .grammar file uses UPPERCASE names to refer to tokens
 * defined in the .tokens file. This module checks that the two files are
 * consistent.
 *
 * Why cross-validate?
 * -------------------
 *
 * Each file can be valid on its own but broken when used together:
 *
 * - A grammar might reference `SEMICOLON`, but the .tokens file only
 *   defines `SEMI`. Each file is fine individually, but the pair is broken.
 * - A .tokens file might define `TILDE = "~"` that no grammar rule ever
 *   uses. This is not an error — it might be intentional — but it is worth
 *   warning about because unused tokens add complexity without value.
 *
 * This is analogous to how a C compiler checks that every function you call
 * is actually declared (and vice versa, warns about unused functions).
 *
 * What we check
 * -------------
 *
 * 1. **Missing token references**: Every UPPERCASE name in the grammar must
 *    correspond to a token definition. If not, the generated parser will try
 *    to match a token type that the lexer never produces.
 *
 * 2. **Unused tokens**: Every token defined in the .tokens file should ideally
 *    be referenced somewhere in the grammar. Unused tokens suggest either a
 *    typo or leftover cruft. We report these as warnings, not errors.
 *
 * 3. **Usage report**: We list which tokens and rules are actually used, which
 *    helps users understand their grammar.
 */

import { grammarTokenReferences, type ParserGrammar } from "./parser-grammar.js";
import { tokenNames, type TokenGrammar } from "./token-grammar.js";

/**
 * Cross-validate a token grammar and a parser grammar.
 *
 * Checks that every UPPERCASE name referenced in the parser grammar
 * exists in the token grammar, and warns about tokens that are defined
 * but never used.
 *
 * @param tokenGrammar - A parsed .tokens file.
 * @param parserGrammar - A parsed .grammar file.
 * @returns A list of error/warning strings. Errors describe broken references;
 *     warnings describe unused definitions. An empty list means the two
 *     grammars are fully consistent.
 */
export function crossValidate(
  tokenGrammar: TokenGrammar,
  parserGrammar: ParserGrammar
): string[] {
  const issues: string[] = [];

  const definedTokens = tokenNames(tokenGrammar);
  const referencedTokens = grammarTokenReferences(parserGrammar);

  // Implicit tokens that are always available
  const implicitTokens = new Set<string>(["EOF"]);

  // In indentation mode, INDENT/DEDENT/NEWLINE are implicitly available
  if (tokenGrammar.mode === "indentation") {
    implicitTokens.add("INDENT");
    implicitTokens.add("DEDENT");
    implicitTokens.add("NEWLINE");
  }

  // The NEWLINE token is also implicitly available whenever the skip
  // pattern does NOT consume newlines. In that case, the lexer emits
  // NEWLINE tokens at each bare '\n'. Rather than requiring grammars to
  // redundantly define NEWLINE in their .tokens file, we always treat it
  // as a valid synthetic token (like EOF).
  implicitTokens.add("NEWLINE");

  // --- Missing token references (errors) ---
  for (const ref of [...referencedTokens].sort()) {
    if (!definedTokens.has(ref) && !implicitTokens.has(ref)) {
      issues.push(
        `Error: Grammar references token '${ref}' which is not ` +
          `defined in the tokens file`
      );
    }
  }

  // --- Unused tokens (warnings) ---
  // Check if a token (or its alias) is used by the grammar
  for (const defn of tokenGrammar.definitions) {
    const isUsed = referencedTokens.has(defn.name) ||
      (defn.alias !== undefined && referencedTokens.has(defn.alias));
    if (!isUsed) {
      issues.push(
        `Warning: Token '${defn.name}' (line ${defn.lineNumber}) ` +
          `is defined but never used in the grammar`
      );
    }
  }

  return issues;
}
