/**
 * Mosaic Lexer — tokenizes `.mosaic` source using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from `@coding-adventures/lexer`. All tokenization logic lives in the generic
 * engine; this module simply provides the Mosaic token grammar.
 *
 * What Is Mosaic?
 * ---------------
 *
 * Mosaic is a **UI component description language**. A `.mosaic` file declares
 * one component: its typed data slots and its visual node tree. There is no
 * imperative logic, no binding expressions, and no runtime — Mosaic compiles
 * forward-only to native code on each target platform (React, Web Components,
 * SwiftUI, Compose, etc.).
 *
 * A minimal Mosaic component looks like this:
 *
 *     component Button {
 *       slot label: text;
 *       slot disabled: bool = false;
 *
 *       Row {
 *         Text { content: @label; }
 *       }
 *     }
 *
 * Why A Separate Language?
 * ------------------------
 *
 * Mosaic exists to give UI components a platform-neutral description:
 *
 *   - **Typed slots** act like props/attributes but are declared explicitly,
 *     making the component's public API machine-readable.
 *   - **Abstract properties** (padding, background, align, etc.) decouple
 *     layout intent from platform-specific style APIs.
 *   - **Forward-only compilation** means no partial evaluation, no tree
 *     diffing, no virtual DOM — just a one-shot code generator.
 *
 * Token Types
 * -----------
 *
 * | Token      | Example           | Description                                   |
 * |------------|-------------------|-----------------------------------------------|
 * | STRING     | "hello"           | Double-quoted string literal                  |
 * | DIMENSION  | 16dp, 100%        | Number immediately followed by a unit suffix  |
 * | NUMBER     | 42, -3.14         | Integer or decimal (no unit)                  |
 * | COLOR_HEX  | #2563eb, #fff     | Hex color: #rgb, #rrggbb, or #rrggbbaa        |
 * | KEYWORD    | component, slot   | Reserved words (see full list below)          |
 * | IDENT      | Button, label     | Component or property name; allows hyphens    |
 * | LBRACE     | {                 | Open block                                    |
 * | RBRACE     | }                 | Close block                                   |
 * | LANGLE     | <                 | Open generic type bracket                     |
 * | RANGLE     | >                 | Close generic type bracket                    |
 * | COLON      | :                 | Type annotation or property separator         |
 * | SEMICOLON  | ;                 | Statement terminator                          |
 * | COMMA      | ,                 | Separator                                     |
 * | DOT        | .                 | Enum namespace separator (e.g., align.center) |
 * | EQUALS     | =                 | Default value assignment                      |
 * | AT         | @                 | Slot reference sigil (e.g., @title)           |
 * | EOF        | (synthetic)       | End of input                                  |
 *
 * Keywords
 * --------
 *
 * The following identifiers are reserved and tokenized as KEYWORD:
 *
 *   component  slot  import  from  as
 *   text  number  bool  image  color  node  list
 *   true  false
 *   when  each
 *
 * Order Matters: DIMENSION before NUMBER
 * ---------------------------------------
 *
 * The grammar lists DIMENSION before NUMBER because both start with digits.
 * The lexer engine tries patterns in declaration order and picks the longest
 * match. "16dp" must match DIMENSION (not NUMBER + IDENT) — this requires
 * DIMENSION to appear first in the definition list.
 *
 * IDENT and Hyphens
 * -----------------
 *
 * Mosaic identifiers allow hyphens (`[a-zA-Z_][a-zA-Z0-9_-]*`) to support
 * CSS-like property names: `corner-radius`, `a11y-label`, `display-name`.
 * This differs from most programming languages where `-` is an operator.
 * Since Mosaic has no arithmetic expressions, hyphens in names are safe.
 *
 * Skip Patterns
 * -------------
 *
 * Three skip patterns are silently discarded (not emitted as tokens):
 *   - `// line comments`
 *   - `/* block comments *\/`
 *   - Whitespace (spaces, tabs, newlines)
 */

import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";
import { TOKEN_GRAMMAR } from "./_grammar.js";

/**
 * Tokenize Mosaic source text and return a flat array of tokens.
 *
 * The function delegates entirely to the generic `grammarTokenize` engine,
 * which handles pattern matching, keyword reclassification, skip patterns,
 * and position tracking.
 *
 * @param source - The `.mosaic` source text to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = tokenizeMosaic('component Button { slot label: text; }');
 *     // [Token(KEYWORD,"component"), Token(IDENT,"Button"), Token(LBRACE,"{"),
 *     //  Token(KEYWORD,"slot"), Token(IDENT,"label"), Token(COLON,":"),
 *     //  Token(KEYWORD,"text"), Token(SEMICOLON,";"), Token(RBRACE,"}"),
 *     //  Token(EOF,"")]
 *
 * @example
 *     // Slot reference in node tree:
 *     const tokens = tokenizeMosaic('Text { content: @title; }');
 *     // [..., Token(AT,"@"), Token(IDENT,"title"), ...]
 *
 * @example
 *     // Dimension and color literals:
 *     const tokens = tokenizeMosaic('padding: 16dp; background: #2563eb;');
 *     // [..., Token(DIMENSION,"16dp"), ..., Token(COLOR_HEX,"#2563eb"), ...]
 */
export function tokenizeMosaic(source: string): Token[] {
  /**
   * Delegate to the generic engine with the pre-compiled Mosaic grammar.
   *
   * The grammar is imported as a TypeScript constant (see _grammar.ts) rather
   * than read from disk, which avoids filesystem I/O and works in environments
   * where the grammars directory might not be available (bundlers, test VMs).
   */
  return grammarTokenize(source, TOKEN_GRAMMAR);
}
