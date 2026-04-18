/**
 * Lexer — Layer 2 of the computing stack.
 *
 * Tokenizes source code into a stream of tokens. The lexer reads raw source
 * code text character by character and groups those characters into meaningful
 * units called *tokens* — the smallest building blocks a parser can work with.
 *
 * Two lexer implementations are provided:
 *
 * - `tokenize` — the hand-written reference implementation with hardcoded
 *   character-dispatching logic.
 * - `grammarTokenize` — a grammar-driven alternative that reads token
 *   definitions from a `.tokens` file (via `grammar-tools`).
 * - `GrammarLexer` — class-based grammar-driven lexer with support for
 *   pattern groups and on-token callbacks for context-sensitive lexing.
 *
 * Both produce identical `Token` objects and are fully interchangeable.
 *
 * Usage:
 *
 *     import { tokenize } from "@coding-adventures/lexer";
 *
 *     const tokens = tokenize("x = 1 + 2");
 *
 *     // With language-specific keywords:
 *     const tokens = tokenize("if x == 1", { keywords: ["if", "else", "while"] });
 *
 *     // Grammar-driven alternative:
 *     import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
 *     import { grammarTokenize } from "@coding-adventures/lexer";
 *
 *     const grammar = parseTokenGrammar(fs.readFileSync("python.tokens", "utf-8"));
 *     const tokens = grammarTokenize("x = 1 + 2", grammar);
 *
 *     // Class-based with on-token callback:
 *     import { GrammarLexer, LexerContext } from "@coding-adventures/lexer";
 *
 *     const lexer = new GrammarLexer(source, grammar);
 *     lexer.setOnToken((token, ctx) => {
 *       if (token.type === "OPEN_TAG") ctx.pushGroup("tag");
 *       if (token.type === "TAG_CLOSE") ctx.popGroup();
 *     });
 *     const tokens = lexer.tokenize();
 */

export type { Token, Trivia } from "./token.js";
export {
  TOKEN_PRECEDED_BY_NEWLINE,
  TOKEN_CONTEXT_KEYWORD,
} from "./token.js";
export { tokenize, LexerError } from "./tokenizer.js";
export type { LexerConfig } from "./tokenizer.js";
export { grammarTokenize, GrammarLexer, LexerContext } from "./grammar-lexer.js";
export type { OnTokenCallback, GrammarLexerOptions } from "./grammar-lexer.js";
