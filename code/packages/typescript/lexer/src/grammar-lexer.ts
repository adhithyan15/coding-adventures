/**
 * Grammar-Driven Lexer — Tokenization from .tokens Files
 * =======================================================
 *
 * The hand-written `tokenize` function in `tokenizer.ts` hardcodes which
 * characters map to which tokens. That works well for a single language,
 * but what if you want to tokenize Python *and* Ruby *and* JavaScript
 * with the same codebase? You would need to rewrite the character-
 * dispatching logic for each language.
 *
 * This module takes a different approach, inspired by classic tools like
 * [Lex](https://en.wikipedia.org/wiki/Lex_(software)) and
 * [Flex](https://en.wikipedia.org/wiki/Flex_(lexical_analyser_generator)).
 * Instead of hardcoding patterns in TypeScript, we read token definitions
 * from a `.tokens` file (parsed by the `grammar-tools` package) and use
 * those definitions to drive tokenization at runtime.
 *
 * Extensions for Starlark/Python-like Languages
 * ----------------------------------------------
 *
 * Beyond basic regex-driven tokenization, this module supports:
 *
 * - **Skip patterns**: Whitespace and comment patterns that are consumed
 *   without producing tokens (defined in the `skip:` section of .tokens).
 * - **Type aliases**: A token definition like `STRING_DQ -> STRING` emits
 *   tokens with type "STRING" instead of "STRING_DQ".
 * - **Reserved keywords**: Identifiers that must not appear in source code
 *   (e.g., `class` and `import` in Starlark). Raises LexerError on match.
 * - **Indentation mode**: For Python-like languages, tracks indentation
 *   levels and emits synthetic INDENT/DEDENT/NEWLINE tokens.
 */

import type { TokenGrammar } from "@coding-adventures/grammar-tools";

import type { Token } from "./token.js";
import { LexerError } from "./tokenizer.js";

// ---------------------------------------------------------------------------
// Compiled Pattern
// ---------------------------------------------------------------------------

interface CompiledPattern {
  readonly name: string;
  readonly pattern: RegExp;
  readonly alias?: string;
}

// ---------------------------------------------------------------------------
// Escape helper for literal patterns
// ---------------------------------------------------------------------------

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// ---------------------------------------------------------------------------
// Token Type Resolution
// ---------------------------------------------------------------------------

/**
 * Resolve a grammar token name and matched value to a token type string.
 *
 * Handles reserved keywords (panic), regular keywords (promote to KEYWORD),
 * and type aliases (alias takes precedence over definition name).
 */
function resolveTokenType(
  tokenName: string,
  value: string,
  keywordSet: ReadonlySet<string>,
  reservedSet: ReadonlySet<string>,
  alias: string | undefined,
  line: number,
  column: number,
): string {
  // Reserved keyword check — these are hard errors
  if (tokenName === "NAME" && reservedSet.has(value)) {
    throw new LexerError(
      `Reserved keyword '${value}' cannot be used as an identifier`,
      line,
      column,
    );
  }

  // Regular keyword check — promote NAME to KEYWORD
  if (tokenName === "NAME" && keywordSet.has(value)) {
    return "KEYWORD";
  }

  // Alias takes precedence
  if (alias) {
    return alias;
  }

  return tokenName;
}

// ---------------------------------------------------------------------------
// Escape Sequence Processing
// ---------------------------------------------------------------------------

function processEscapes(s: string): string {
  const result: string[] = [];
  let i = 0;

  while (i < s.length) {
    if (s[i] === "\\" && i + 1 < s.length) {
      const escapeMap: Record<string, string> = {
        n: "\n",
        t: "\t",
        "\\": "\\",
        '"': '"',
      };
      const nextChar = s[i + 1];
      result.push(escapeMap[nextChar] ?? nextChar);
      i += 2;
    } else {
      result.push(s[i]);
      i += 1;
    }
  }

  return result.join("");
}

// ---------------------------------------------------------------------------
// The Grammar-Driven Lexer
// ---------------------------------------------------------------------------

/**
 * Tokenize source code using a grammar (parsed from a `.tokens` file).
 *
 * Supports skip patterns, type aliases, reserved keywords, and indentation
 * mode. The function auto-detects whether to use standard or indentation
 * tokenization based on `grammar.mode`.
 */
export function grammarTokenize(source: string, grammar: TokenGrammar): Token[] {
  // Shared state
  let pos = 0;
  let line = 1;
  let column = 1;

  const keywordSet: ReadonlySet<string> = new Set(grammar.keywords);
  const reservedSet: ReadonlySet<string> = new Set(grammar.reservedKeywords ?? []);

  // Compile token patterns
  const patterns: CompiledPattern[] = grammar.definitions.map((defn) => {
    const patternSource = defn.isRegex ? defn.pattern : escapeRegExp(defn.pattern);
    return {
      name: defn.name,
      pattern: new RegExp(patternSource),
      alias: defn.alias,
    };
  });

  // Compile skip patterns
  const skipPatterns: RegExp[] = (grammar.skipDefinitions ?? []).map((defn) => {
    const patternSource = defn.isRegex ? defn.pattern : escapeRegExp(defn.pattern);
    return new RegExp(patternSource);
  });

  // Advance helper
  function advance(): void {
    if (pos < source.length) {
      if (source[pos] === "\n") {
        line += 1;
        column = 1;
      } else {
        column += 1;
      }
      pos += 1;
    }
  }

  // Try to match and consume a skip pattern. Returns true if matched.
  function trySkip(): boolean {
    const remaining = source.slice(pos);
    for (const pat of skipPatterns) {
      const match = pat.exec(remaining);
      if (match !== null && match.index === 0) {
        for (let i = 0; i < match[0].length; i++) {
          advance();
        }
        return true;
      }
    }
    return false;
  }

  // Try to match a token at current position. Returns token or null.
  function tryMatchToken(): Token | null {
    const remaining = source.slice(pos);

    for (const { name, pattern, alias } of patterns) {
      const match = pattern.exec(remaining);
      if (match !== null && match.index === 0) {
        let value = match[0];
        const startLine = line;
        const startColumn = column;

        const tokenType = resolveTokenType(
          name, value, keywordSet, reservedSet, alias, startLine, startColumn,
        );

        // Handle STRING tokens: strip quotes and process escapes
        if (name === "STRING" || (alias && alias.includes("STRING"))) {
          if (value.length >= 2 && (value[0] === '"' || value[0] === "'")) {
            const inner = value.slice(1, -1);
            value = processEscapes(inner);
          }
        }

        const tok: Token = { type: tokenType, value, line: startLine, column: startColumn };

        for (let i = 0; i < match[0].length; i++) {
          advance();
        }

        return tok;
      }
    }
    return null;
  }

  // Dispatch to the appropriate tokenization mode
  if (grammar.mode === "indentation") {
    return tokenizeIndentation();
  }
  return tokenizeStandard();

  // ---------------------------------------------------------------------------
  // Standard tokenization (no indentation tracking)
  // ---------------------------------------------------------------------------

  function tokenizeStandard(): Token[] {
    const tokens: Token[] = [];

    while (pos < source.length) {
      const char = source[pos];

      // Skip whitespace
      if (char === " " || char === "\t" || char === "\r") {
        advance();
        continue;
      }

      // Newlines become NEWLINE tokens
      if (char === "\n") {
        tokens.push({ type: "NEWLINE", value: "\\n", line, column });
        advance();
        continue;
      }

      // Try skip patterns
      if (trySkip()) {
        continue;
      }

      // Try token patterns
      const tok = tryMatchToken();
      if (tok !== null) {
        tokens.push(tok);
        continue;
      }

      throw new LexerError(
        `Unexpected character: ${JSON.stringify(char)}`,
        line,
        column,
      );
    }

    tokens.push({ type: "EOF", value: "", line, column });
    return tokens;
  }

  // ---------------------------------------------------------------------------
  // Indentation tokenization (Python-like INDENT/DEDENT)
  // ---------------------------------------------------------------------------

  function tokenizeIndentation(): Token[] {
    const tokens: Token[] = [];
    const indentStack: number[] = [0];
    let bracketDepth = 0;
    let atLineStart = true;

    while (pos < source.length) {
      // Process line start (indentation)
      if (atLineStart && bracketDepth === 0) {
        const result = processLineStart(indentStack);
        if (result === "skip") {
          continue;
        }
        tokens.push(...result);
        atLineStart = false;
        if (pos >= source.length) {
          break;
        }
      }

      const char = source[pos];

      // Newline handling
      if (char === "\n") {
        if (bracketDepth === 0) {
          tokens.push({ type: "NEWLINE", value: "\\n", line, column });
        }
        advance();
        atLineStart = true;
        continue;
      }

      // Inside brackets: skip whitespace
      if (bracketDepth > 0 && (char === " " || char === "\t" || char === "\r")) {
        advance();
        continue;
      }

      // Try skip patterns
      if (trySkip()) {
        continue;
      }

      // Try token patterns
      const tok = tryMatchToken();
      if (tok !== null) {
        // Track bracket depth
        if (tok.value === "(" || tok.value === "[" || tok.value === "{") {
          bracketDepth++;
        } else if (tok.value === ")" || tok.value === "]" || tok.value === "}") {
          bracketDepth--;
        }
        tokens.push(tok);
        continue;
      }

      throw new LexerError(
        `Unexpected character: ${JSON.stringify(char)}`,
        line,
        column,
      );
    }

    // EOF: emit remaining DEDENTs
    while (indentStack.length > 1) {
      indentStack.pop();
      tokens.push({ type: "DEDENT", value: "", line, column });
    }

    // Final NEWLINE if needed
    if (tokens.length === 0 || tokens[tokens.length - 1].type !== "NEWLINE") {
      tokens.push({ type: "NEWLINE", value: "\\n", line, column });
    }

    tokens.push({ type: "EOF", value: "", line, column });
    return tokens;
  }

  /**
   * Process indentation at the start of a logical line.
   * Returns "skip" if the line should be skipped (blank/comment),
   * or an array of INDENT/DEDENT tokens.
   */
  function processLineStart(indentStack: number[]): "skip" | Token[] {
    let indent = 0;
    while (pos < source.length) {
      const char = source[pos];
      if (char === " ") {
        indent++;
        advance();
      } else if (char === "\t") {
        throw new LexerError(
          "Tab character in indentation (use spaces only)",
          line,
          column,
        );
      } else {
        break;
      }
    }

    // Blank line or EOF
    if (pos >= source.length) {
      return "skip";
    }
    if (source[pos] === "\n") {
      advance(); // Consume newline to avoid infinite loop
      return "skip";
    }

    // Comment-only line — check skip patterns
    const remaining = source.slice(pos);
    for (const pat of skipPatterns) {
      const match = pat.exec(remaining);
      if (match !== null && match.index === 0) {
        const peekPos = pos + match[0].length;
        if (peekPos >= source.length || source[peekPos] === "\n") {
          for (let i = 0; i < match[0].length; i++) {
            advance();
          }
          if (pos < source.length && source[pos] === "\n") {
            advance();
          }
          return "skip";
        }
      }
    }

    // Compare indent to current level
    const currentIndent = indentStack[indentStack.length - 1];
    const indentTokens: Token[] = [];

    if (indent > currentIndent) {
      indentStack.push(indent);
      indentTokens.push({ type: "INDENT", value: "", line, column: 1 });
    } else if (indent < currentIndent) {
      while (
        indentStack.length > 1 &&
        indentStack[indentStack.length - 1] > indent
      ) {
        indentStack.pop();
        indentTokens.push({ type: "DEDENT", value: "", line, column: 1 });
      }
      if (indentStack[indentStack.length - 1] !== indent) {
        throw new LexerError(
          "Inconsistent dedent",
          line,
          column,
        );
      }
    }

    return indentTokens;
  }
}
