/**
 * VHDL Lexer — tokenizes VHDL (IEEE 1076-2008) source code.
 *
 * This module is a wrapper around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads the `vhdl.tokens`
 * grammar file, delegates all tokenization to the generic engine, and then
 * applies VHDL-specific post-processing: case normalization.
 *
 * What Is VHDL?
 * -------------
 *
 * VHDL (VHSIC Hardware Description Language) is a Hardware Description
 * Language used to model and synthesize digital circuits. Developed by
 * the US Department of Defense in the 1980s, VHDL was derived from Ada
 * and inherits Ada's emphasis on strong typing, readability, and explicit
 * declarations.
 *
 * A simple VHDL entity:
 *
 *     entity adder is
 *       port(
 *         a, b : in  std_logic_vector(7 downto 0);
 *         sum  : out std_logic_vector(7 downto 0)
 *       );
 *     end entity adder;
 *
 * This describes the INTERFACE of an 8-bit adder. The implementation
 * goes in a separate `architecture` block — VHDL strictly separates
 * interface from implementation, unlike Verilog which mixes them.
 *
 * VHDL vs Verilog Tokenization
 * ----------------------------
 *
 * VHDL has several unique lexical features that distinguish it from Verilog:
 *
 * 1. **Case insensitivity**: ENTITY, Entity, and entity are identical.
 *    After tokenization, we normalize NAME and KEYWORD values to lowercase.
 *    This happens in a post-processing step so the grammar file doesn't
 *    need case-insensitive regex (which would be complex and slow).
 *
 * 2. **No preprocessor**: VHDL has no `define/`ifdef equivalent. Code
 *    reuse uses libraries and packages; conditional compilation uses
 *    generics and generate statements — all at the language level.
 *
 * 3. **Character literals**: 'A', '0', '1', 'X', 'Z'
 *    Single characters in tick marks. The std_logic type uses these
 *    for its nine value system (U, X, 0, 1, Z, W, L, H, -).
 *
 * 4. **Bit string literals**: B"1010", X"FF", O"77"
 *    Prefixed strings that represent binary/hex/octal bit patterns.
 *    Similar to Verilog's sized literals but with different syntax.
 *
 * 5. **Keyword operators**: and, or, xor, not, nand, nor, xnor
 *    Logical operations are keywords, not symbols. VHDL prefers
 *    readability: `a and b` instead of `a & b`.
 *
 * 6. **The tick mark (')**:  Does double duty as a character literal
 *    delimiter ('X') AND an attribute access operator (signal'event).
 *    The grammar handles this via ordering: CHAR_LITERAL matches first.
 *
 * 7. **Variable assignment (:=) vs signal assignment (<=)**:
 *    Two different assignment operators for two different kinds of
 *    data storage. Variables update immediately (like software);
 *    signals schedule updates (like hardware wiring).
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `vhdl.tokens` file lives in `code/grammars/` at the repository root.
 * We navigate from this module's location up to that directory:
 *
 *     src/tokenizer.ts -> vhdl-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import type { TokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize, GrammarLexer } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

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
 *   __dirname = .../vhdl-lexer/src/
 *   ..         = .../vhdl-lexer/
 *   ../..      = .../typescript/
 *   ../../..   = .../packages/
 *   ../../../.. = .../code/
 *   + grammars  = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const VHDL_TOKENS_PATH = join(GRAMMARS_DIR, "vhdl.tokens");

/**
 * Load and parse the VHDL grammar.
 *
 * This is factored out so both `tokenizeVhdl` and `createVhdlLexer`
 * can share it. In a production system you would cache this, but for an
 * educational codebase, clarity beats performance.
 */
function loadVhdlGrammar() {
  const grammarText = readFileSync(VHDL_TOKENS_PATH, "utf-8");
  return parseTokenGrammar(grammarText);
}

/**
 * Apply VHDL case normalization to a token array.
 *
 * VHDL is case-insensitive for identifiers and keywords. The IEEE 1076
 * standard says:
 *
 *   "Basic identifiers differing only in the use of corresponding
 *    upper and lower case letters are considered as the same."
 *
 * To implement this, we lowercase the `value` field of every NAME and
 * KEYWORD token after tokenization. This means:
 *
 *   - `ENTITY` becomes `entity` (a KEYWORD)
 *   - `My_Signal` becomes `my_signal` (a NAME)
 *   - `X"FF"` stays `X"FF"` (a BIT_STRING — not normalized)
 *   - `"Hello"` stays `"Hello"` (a STRING — not normalized)
 *
 * Why post-process instead of normalizing in the grammar?
 *
 * The grammar file uses regex patterns. Making every letter pattern
 * case-insensitive would be complex and error-prone. It's simpler
 * and clearer to tokenize with case-preserving patterns and then
 * normalize in a single pass afterward.
 *
 * There is a subtlety: because the grammar's keyword list is lowercase
 * (e.g., "entity"), but the input might be "ENTITY", the grammar lexer
 * produces a NAME token (since "ENTITY" doesn't match the keyword "entity").
 * After lowercasing the value to "entity", we must re-check whether it
 * matches a keyword and promote the token type from NAME to KEYWORD.
 *
 * @param tokens - The raw token array from grammarTokenize.
 * @param keywordSet - The set of keywords from the grammar.
 * @returns A new array with NAME and KEYWORD values lowercased,
 *          and uppercase keywords reclassified from NAME to KEYWORD.
 */
function normalizeCasing(tokens: Token[], keywordSet: ReadonlySet<string>): Token[] {
  return tokens.map((token) => {
    if (token.type === "NAME" || token.type === "KEYWORD") {
      const lowered = token.value.toLowerCase();
      /**
       * Re-classify: if the lowercased value matches a keyword,
       * this token should be KEYWORD, not NAME. This handles the
       * case where "ENTITY" was tokenized as NAME (because the
       * grammar only lists lowercase "entity" as a keyword).
       */
      const type = keywordSet.has(lowered) ? "KEYWORD" : "NAME";
      return {
        type,
        value: lowered,
        line: token.line,
        column: token.column,
      };
    }
    return token;
  });
}

/**
 * Tokenize VHDL source code and return an array of tokens.
 *
 * This is the primary entry point for the VHDL lexer. It reads the
 * `vhdl.tokens` grammar file, tokenizes the source, and then applies
 * case normalization to all NAME and KEYWORD tokens.
 *
 * Unlike the Verilog lexer, there is no preprocessor step. VHDL
 * handles everything through its language-level library and package
 * system.
 *
 * @param source - The VHDL source code to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *          NAME and KEYWORD values are lowercased.
 *
 * @example
 *     const tokens = tokenizeVhdl("ENTITY my_chip IS END ENTITY;");
 *     // [KEYWORD("entity"), NAME("my_chip"), KEYWORD("is"),
 *     //  KEYWORD("end"), KEYWORD("entity"), SEMICOLON(";"), EOF("")]
 *
 * @example
 *     // Case normalization in action:
 *     const tokens = tokenizeVhdl("Signal MY_SIG : STD_LOGIC;");
 *     // KEYWORD("signal"), NAME("my_sig"), COLON(":"),
 *     // NAME("std_logic"), SEMICOLON(";"), EOF("")
 *
 * @example
 *     // Bit strings are NOT normalized:
 *     const tokens = tokenizeVhdl('X"FF"');
 *     // BIT_STRING('X"FF"'), EOF("")
 */
export function tokenizeVhdl(source: string): Token[] {
  /**
   * Step 1: Load and parse the VHDL grammar.
   */
  const grammar = loadVhdlGrammar();

  /**
   * Step 2: Run the generic tokenizer with the VHDL grammar.
   */
  const rawTokens = grammarTokenize(source, grammar);

  /**
   * Step 3: Apply case normalization.
   *
   * This is the VHDL-specific post-processing step. NAME and KEYWORD
   * tokens get their values lowercased to reflect VHDL's case
   * insensitivity. All other token types (STRING, BIT_STRING,
   * CHAR_LITERAL, NUMBER, operators, etc.) keep their original casing.
   *
   * We also re-classify NAME tokens as KEYWORD when their lowercased
   * value matches a grammar keyword (e.g., "ENTITY" -> KEYWORD("entity")).
   */
  const keywordSet = new Set(grammar.keywords);
  return normalizeCasing(rawTokens, keywordSet);
}

/**
 * Create a GrammarLexer instance for VHDL source code.
 *
 * Unlike `tokenizeVhdl` which returns an array of all tokens at once,
 * `createVhdlLexer` returns a `GrammarLexer` object that allows
 * incremental tokenization. Note that when using the GrammarLexer
 * directly, you must apply case normalization yourself if needed.
 *
 * This is useful for:
 *   - Large files where you don't want all tokens in memory at once
 *   - Streaming/incremental processing
 *   - Custom token filtering or transformation during lexing
 *
 * @param source - The VHDL source code to tokenize.
 * @returns A GrammarLexer instance ready to tokenize.
 *
 * @example
 *     const lexer = createVhdlLexer("entity top is end entity;");
 *     const tokens = lexer.tokenize();
 */
export function createVhdlLexer(source: string): GrammarLexer {
  const grammar = loadVhdlGrammar();
  return new GrammarLexer(source, grammar);
}
