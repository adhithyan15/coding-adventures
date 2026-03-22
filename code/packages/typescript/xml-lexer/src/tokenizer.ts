/**
 * XML Lexer — tokenizes XML using pattern groups and callback hooks.
 *
 * This module is the first lexer wrapper that uses the **pattern group**
 * and **on-token callback** features of the grammar-driven lexer. It loads
 * the `xml.tokens` grammar and registers a callback that switches between
 * pattern groups based on which token was just matched.
 *
 * Context-Sensitive Lexing
 * ------------------------
 *
 * XML requires context-sensitive lexing because different parts of an XML
 * document follow different lexical rules:
 *
 * - **Between tags** (default group): Text content, entity references
 *   like `&amp;`, and opening delimiters for tags/comments/CDATA/PIs.
 *
 * - **Inside a tag** (tag group): Tag names, attribute names (same regex
 *   as tag names), equals signs, quoted attribute values, and closing
 *   delimiters like `>` and `/>`.
 *
 * - **Inside a comment** (comment group): Everything is comment text until
 *   `-->` is seen. Whitespace is significant (not skipped).
 *
 * - **Inside CDATA** (cdata group): Everything is raw text until `]]>`
 *   is seen. No entity processing, no tag recognition.
 *
 * - **Inside a processing instruction** (pi group): Target name and text
 *   content until `?>` is seen.
 *
 * The Callback
 * ------------
 *
 * The `xmlOnToken` function is the callback that drives group switching.
 * It follows a simple state machine:
 *
 *     default ──OPEN_TAG_START──> tag ──TAG_CLOSE──> default
 *             ──CLOSE_TAG_START─> tag ──SELF_CLOSE─> default
 *             ──COMMENT_START───> comment ──COMMENT_END──> default
 *             ──CDATA_START─────> cdata ──CDATA_END──> default
 *             ──PI_START────────> pi ──PI_END──> default
 *
 * For comment, CDATA, and PI groups, the callback also disables skip
 * patterns (so whitespace is preserved as content) and re-enables them
 * when leaving the group.
 *
 * Locating the Grammar File
 * --------------------------
 *
 * The `xml.tokens` file lives in `code/grammars/` at the repository root.
 * We navigate there from this file's location:
 *
 *     src/tokenizer.ts -> xml-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { GrammarLexer, LexerContext } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

// ---------------------------------------------------------------------------
// Grammar File Location
// ---------------------------------------------------------------------------

/**
 * Resolve __dirname for ESM modules.
 *
 * In CommonJS, __dirname is a global. In ESM, it does not exist — we must
 * derive it from import.meta.url, which gives the file URL of the current
 * module (e.g., "file:///path/to/tokenizer.ts"). The fileURLToPath + dirname
 * pattern converts this to a directory path.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Navigate from src/ up to the grammars/ directory.
 *
 * The path traversal:
 *   __dirname = .../xml-lexer/src/
 *   ..         = .../xml-lexer/
 *   ../..      = .../typescript/
 *   ../../..   = .../packages/
 *   ../../../.. = .../code/
 *   + grammars  = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const XML_TOKENS_PATH = join(GRAMMARS_DIR, "xml.tokens");

// ---------------------------------------------------------------------------
// XML On-Token Callback
// ---------------------------------------------------------------------------
//
// This callback drives the group transitions. It is a pure function of
// the token type — no external state is needed. The LexerContext provides
// all the control we need (push/pop groups, toggle skip).
//
// The pattern is simple:
// - Opening delimiters push a group
// - Closing delimiters pop the group
// - Comment/CDATA/PI groups disable skip (whitespace is content)
// ---------------------------------------------------------------------------

/**
 * Callback that switches pattern groups for XML tokenization.
 *
 * This function fires after each token match. It examines the token
 * type and pushes/pops pattern groups accordingly:
 *
 * - `OPEN_TAG_START` (`<`) or `CLOSE_TAG_START` (`</`):
 *   Push the "tag" group so the lexer recognizes tag names, attributes,
 *   and tag closers.
 *
 * - `TAG_CLOSE` (`>`) or `SELF_CLOSE` (`/>`):
 *   Pop the "tag" group to return to default (text content).
 *
 * - `COMMENT_START` (`<!--`):
 *   Push "comment" group and disable skip (whitespace is significant).
 *
 * - `COMMENT_END` (`-->`):
 *   Pop "comment" group and re-enable skip.
 *
 * - `CDATA_START` (`<![CDATA[`):
 *   Push "cdata" group and disable skip.
 *
 * - `CDATA_END` (`]]>`):
 *   Pop "cdata" group and re-enable skip.
 *
 * - `PI_START` (`<?`):
 *   Push "pi" group and disable skip.
 *
 * - `PI_END` (`?>`):
 *   Pop "pi" group and re-enable skip.
 *
 * @param token - The token that was just matched.
 * @param ctx - The LexerContext for controlling the lexer.
 */
export function xmlOnToken(token: Token, ctx: LexerContext): void {
  const tokenType = token.type;

  switch (tokenType) {
    // --- Tag boundaries ---
    //
    // When we see `<` or `</`, we push the "tag" group. This activates
    // the tag-specific patterns (TAG_NAME, ATTR_EQUALS, ATTR_VALUE,
    // TAG_CLOSE, SELF_CLOSE) and deactivates the default-group patterns
    // (TEXT, ENTITY_REF, etc.).
    case "OPEN_TAG_START":
    case "CLOSE_TAG_START":
      ctx.pushGroup("tag");
      break;

    // When we see `>` or `/>`, we pop the "tag" group to return to
    // the default group where text content and entity references live.
    case "TAG_CLOSE":
    case "SELF_CLOSE":
      ctx.popGroup();
      break;

    // --- Comment boundaries ---
    //
    // Comments are special: whitespace inside them is significant.
    // We disable skip patterns so that spaces, tabs, and newlines
    // inside `<!-- ... -->` are preserved as COMMENT_TEXT content.
    case "COMMENT_START":
      ctx.pushGroup("comment");
      ctx.setSkipEnabled(false);
      break;
    case "COMMENT_END":
      ctx.popGroup();
      ctx.setSkipEnabled(true);
      break;

    // --- CDATA boundaries ---
    //
    // CDATA sections contain raw character data — no entity processing,
    // no tag recognition. Everything is literal text. Like comments,
    // whitespace is significant so we disable skip.
    case "CDATA_START":
      ctx.pushGroup("cdata");
      ctx.setSkipEnabled(false);
      break;
    case "CDATA_END":
      ctx.popGroup();
      ctx.setSkipEnabled(true);
      break;

    // --- Processing instruction boundaries ---
    //
    // PIs like `<?xml version="1.0"?>` contain a target name and
    // optional text content. Whitespace in the PI text is significant,
    // so we disable skip.
    case "PI_START":
      ctx.pushGroup("pi");
      ctx.setSkipEnabled(false);
      break;
    case "PI_END":
      ctx.popGroup();
      ctx.setSkipEnabled(true);
      break;
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Create a `GrammarLexer` configured for XML text.
 *
 * This function reads the `xml.tokens` file, parses it into a
 * `TokenGrammar` object, creates a `GrammarLexer`, and registers the
 * XML on-token callback for pattern group switching.
 *
 * @param source - The XML text to tokenize.
 * @returns A `GrammarLexer` instance configured with XML token definitions
 *   and the group-switching callback. Call `.tokenize()` to get the token list.
 *
 * @example
 *     const lexer = createXMLLexer('<div>hello</div>');
 *     const tokens = lexer.tokenize();
 */
export function createXMLLexer(source: string): GrammarLexer {
  /**
   * Read and parse the grammar file. The `xml.tokens` file defines
   * 5 pattern groups (default, tag, comment, cdata, pi) with their
   * respective token patterns.
   */
  const grammarText = readFileSync(XML_TOKENS_PATH, "utf-8");
  const grammar = parseTokenGrammar(grammarText);

  /**
   * Create the lexer and register the callback. The GrammarLexer
   * handles the actual regex matching and position tracking. Our
   * callback handles the context-sensitive group switching.
   */
  const lexer = new GrammarLexer(source, grammar);
  lexer.setOnToken(xmlOnToken);
  return lexer;
}

/**
 * Tokenize XML text and return an array of tokens.
 *
 * This is the main entry point for the XML lexer. Pass in a string
 * of XML text, and get back a flat array of `Token` objects. The
 * array always ends with an `EOF` token.
 *
 * Token types you will see:
 *
 * **Default group** (content between tags):
 *
 * - **TEXT** — text content (e.g., `Hello world`)
 * - **ENTITY_REF** — entity reference (e.g., `&amp;`)
 * - **CHAR_REF** — character reference (e.g., `&#65;`, `&#x41;`)
 * - **OPEN_TAG_START** — `<`
 * - **CLOSE_TAG_START** — `</`
 * - **COMMENT_START** — `<!--`
 * - **CDATA_START** — `<![CDATA[`
 * - **PI_START** — `<?`
 *
 * **Tag group** (inside tags):
 *
 * - **TAG_NAME** — tag or attribute name (e.g., `div`, `class`)
 * - **ATTR_EQUALS** — `=`
 * - **ATTR_VALUE** — quoted attribute value (e.g., `"main"`)
 * - **TAG_CLOSE** — `>`
 * - **SELF_CLOSE** — `/>`
 *
 * **Comment group**:
 *
 * - **COMMENT_TEXT** — comment content
 * - **COMMENT_END** — `-->`
 *
 * **CDATA group**:
 *
 * - **CDATA_TEXT** — raw text content
 * - **CDATA_END** — `]]>`
 *
 * **Processing instruction group**:
 *
 * - **PI_TARGET** — PI target name (e.g., `xml`)
 * - **PI_TEXT** — PI content
 * - **PI_END** — `?>`
 *
 * **Always present**:
 *
 * - **EOF** — end of input
 *
 * @param source - The XML text to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = tokenizeXML('<p>Hello &amp; world</p>');
 *     // [Token(OPEN_TAG_START, '<'), Token(TAG_NAME, 'p'),
 *     //  Token(TAG_CLOSE, '>'), Token(TEXT, 'Hello '),
 *     //  Token(ENTITY_REF, '&amp;'), Token(TEXT, ' world'),
 *     //  Token(CLOSE_TAG_START, '</'), Token(TAG_NAME, 'p'),
 *     //  Token(TAG_CLOSE, '>'), Token(EOF, '')]
 */
export function tokenizeXML(source: string): Token[] {
  return createXMLLexer(source).tokenize();
}
