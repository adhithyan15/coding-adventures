/**
 * Browser-compatible Lattice transpiler — backward-compatible re-export.
 *
 * This file previously contained a standalone browser transpiler with
 * embedded grammar strings and its own tokenization/parsing pipeline.
 * That workaround is no longer needed: the underlying lattice-lexer and
 * lattice-parser packages now use pre-compiled grammar objects (from
 * `_grammar.ts` files) instead of reading grammar files from disk.
 *
 * The main `transpileLattice()` function from `index.ts` works in all
 * environments — Node.js, browsers, edge runtimes — without any special
 * configuration. This file simply re-exports it under the old name for
 * backward compatibility with existing consumers.
 *
 * Migration
 * ---------
 *
 * Replace:
 *     import { transpileLatticeInBrowser } from
 *       "@coding-adventures/lattice-transpiler/src/browser.js";
 *
 * With:
 *     import { transpileLattice } from "@coding-adventures/lattice-transpiler";
 *
 * Both functions are now identical.
 */

import { transpileLattice } from "./index.js";
import type { TranspileOptions } from "./index.js";

/**
 * Transpile Lattice source text to CSS.
 *
 * Verbatim copy of code/grammars/lattice.tokens
 * Defines all CSS token types plus 5 Lattice extensions:
 *   VARIABLE, EQUALS_EQUALS, NOT_EQUALS, GREATER_EQUALS, LESS_EQUALS
 */
const LATTICE_TOKENS_GRAMMAR = `# Token definitions for Lattice — a CSS superset language
#
# Lattice extends CSS with variables ($var), mixins (@mixin/@include),
# control flow (@if/@for/@each), functions (@function/@return), and
# modules (@use). This file is a standalone copy of all CSS token
# definitions plus 5 new tokens for Lattice-specific constructs.
#
# The existing css.tokens is NOT modified. This file is self-contained.
#
# New tokens:
#   VARIABLE       — $color, $font-size (CSS never uses $)
#   EQUALS_EQUALS  — == (equality comparison in @if expressions)
#   NOT_EQUALS     — != (inequality comparison)
#   GREATER_EQUALS — >= (greater-or-equal comparison)
#   LESS_EQUALS    — <= (less-or-equal comparison)
#
# Format:
#   TOKEN_NAME = /regex/        — regex-based token pattern
#   TOKEN_NAME = "literal"      — exact literal match
#   TOKEN_NAME = /regex/ -> T   — regex pattern, emitted as token type T

# ============================================================================
# Escape Mode
# ============================================================================

escapes: none

# ============================================================================
# Skip Patterns
# ============================================================================

skip:
  LINE_COMMENT = /\\/\\/[^\\n]*/
  COMMENT      = /\\/\\*[\\s\\S]*?\\*\\//
  WHITESPACE   = /[ \\t\\r\\n]+/

# ============================================================================
# String Literals
# ============================================================================

STRING_DQ = /"([^"\\\\\\n]|\\\\.)*"/ -> STRING
STRING_SQ = /'([^'\\\\\\n]|\\\\.)*'/ -> STRING

# ============================================================================
# Variable Token (NEW — Lattice extension)
# ============================================================================

VARIABLE = /\\$[a-zA-Z_][a-zA-Z0-9_-]*/

# ============================================================================
# Numeric Literals and Dimensions
# ============================================================================

DIMENSION   = /-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?[a-zA-Z]+/
PERCENTAGE  = /-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?%/
NUMBER      = /-?[0-9]*\\.?[0-9]+([eE][+-]?[0-9]+)?/

# ============================================================================
# Hash Token
# ============================================================================

HASH = /#[a-zA-Z0-9_-]+/

# ============================================================================
# At-Keyword Token
# ============================================================================

AT_KEYWORD = /@-?[a-zA-Z][a-zA-Z0-9-]*/

# ============================================================================
# URL Token
# ============================================================================

URL_TOKEN = /url\\([^)'"]*\\)/

# ============================================================================
# Function Token
# ============================================================================

FUNCTION = /-?[a-zA-Z_][a-zA-Z0-9_-]*\\(/

# ============================================================================
# CDO / CDC (Legacy HTML Comment Delimiters)
# ============================================================================

CDO = "<!--"
CDC = "-->"

# ============================================================================
# Identifiers
# ============================================================================

UNICODE_RANGE   = /[Uu]\\+[0-9a-fA-F?]{1,6}(-[0-9a-fA-F]{1,6})?/
CUSTOM_PROPERTY = /--[a-zA-Z_][a-zA-Z0-9_-]*/
IDENT           = /-?[a-zA-Z_][a-zA-Z0-9_-]*/

# ============================================================================
# Multi-Character Operators
# ============================================================================

COLON_COLON    = "::"
TILDE_EQUALS   = "~="
PIPE_EQUALS    = "|="
CARET_EQUALS   = "^="
DOLLAR_EQUALS  = "$="
STAR_EQUALS    = "*="

# Lattice comparison operators (NEW)
EQUALS_EQUALS  = "=="
NOT_EQUALS     = "!="
GREATER_EQUALS = ">="
LESS_EQUALS    = "<="

# ============================================================================
# Single-Character Delimiters and Operators
# ============================================================================

LBRACE    = "{"
RBRACE    = "}"
LPAREN    = "("
RPAREN    = ")"
LBRACKET  = "["
RBRACKET  = "]"
SEMICOLON = ";"
COLON     = ":"
COMMA     = ","
DOT       = "."
PLUS      = "+"
GREATER   = ">"
TILDE     = "~"
STAR      = "*"
PIPE      = "|"
BANG      = "!"
SLASH     = "/"
EQUALS    = "="
AMPERSAND = "&"
MINUS     = "-"
`;

/**
 * The Lattice parser grammar.
 *
 * Verbatim copy of code/grammars/lattice.grammar
 * Defines ~45 EBNF rules: stylesheet, rule, qualified_rule, at_rule,
 * declaration, and all Lattice constructs.
 */
const LATTICE_PARSER_GRAMMAR = `# Parser grammar for Lattice — a CSS superset language
#
# Lattice extends CSS3 with variables, mixins, control flow, functions, and
# modules. This file is a standalone copy of all CSS grammar rules plus ~20
# new rules for Lattice-specific constructs.

# ============================================================================
# Top-Level Structure
# ============================================================================

stylesheet = { rule } ;

rule = lattice_rule | at_rule | qualified_rule ;

lattice_rule = variable_declaration
             | mixin_definition
             | function_definition
             | use_directive
             | lattice_control ;

# ============================================================================
# Lattice: Variables
# ============================================================================

variable_declaration = VARIABLE COLON value_list SEMICOLON ;

# ============================================================================
# Lattice: Mixins
# ============================================================================

mixin_definition = "@mixin" FUNCTION [ mixin_params ] RPAREN block
                 | "@mixin" IDENT block ;

mixin_params = mixin_param { COMMA mixin_param } ;

mixin_param = VARIABLE [ COLON value_list ] ;

include_directive = "@include" FUNCTION [ include_args ] RPAREN ( SEMICOLON | block )
                  | "@include" IDENT ( SEMICOLON | block ) ;

include_args = include_arg { COMMA include_arg } ;

include_arg = VARIABLE COLON value_list | value_list ;

# ============================================================================
# Lattice: Control Flow
# ============================================================================

lattice_control = if_directive | for_directive | each_directive ;

if_directive = "@if" lattice_expression block
               { "@else" "if" lattice_expression block }
               [ "@else" block ] ;

for_directive = "@for" VARIABLE "from" lattice_expression
                ( "through" | "to" ) lattice_expression block ;

each_directive = "@each" VARIABLE { COMMA VARIABLE } "in"
                 each_list block ;

each_list = value { COMMA value } ;

# ============================================================================
# Lattice: Expressions
# ============================================================================

lattice_expression = lattice_or_expr ;

lattice_or_expr = lattice_and_expr { "or" lattice_and_expr } ;

lattice_and_expr = lattice_comparison { "and" lattice_comparison } ;

lattice_comparison = lattice_additive [ comparison_op lattice_additive ] ;

comparison_op = EQUALS_EQUALS | NOT_EQUALS
              | GREATER | GREATER_EQUALS | LESS_EQUALS ;

lattice_additive = lattice_multiplicative
                   { ( PLUS | MINUS ) lattice_multiplicative } ;

lattice_multiplicative = lattice_unary { STAR lattice_unary } ;

lattice_unary = MINUS lattice_unary | lattice_primary ;

lattice_primary = VARIABLE | NUMBER | DIMENSION | PERCENTAGE
                | STRING | IDENT | HASH
                | "true" | "false" | "null"
                | function_call
                | LPAREN lattice_expression RPAREN ;

# ============================================================================
# Lattice: Functions
# ============================================================================

function_definition = "@function" FUNCTION [ mixin_params ] RPAREN
                      function_body ;

function_body = LBRACE { function_body_item } RBRACE ;

function_body_item = variable_declaration | return_directive | lattice_control ;

return_directive = "@return" lattice_expression SEMICOLON ;

# ============================================================================
# Lattice: Modules
# ============================================================================

use_directive = "@use" STRING [ "as" IDENT ] SEMICOLON ;

# ============================================================================
# CSS: At-Rules
# ============================================================================

at_rule = AT_KEYWORD at_prelude ( SEMICOLON | block ) ;

at_prelude = { at_prelude_token } ;

at_prelude_token = IDENT | STRING | NUMBER | DIMENSION | PERCENTAGE
                 | HASH | CUSTOM_PROPERTY | UNICODE_RANGE
                 | VARIABLE
                 | function_in_prelude | paren_block
                 | COLON | COMMA | SLASH | DOT | STAR | PLUS | MINUS
                 | GREATER | TILDE | PIPE | EQUALS | AMPERSAND
                 | CDO | CDC ;

function_in_prelude = FUNCTION at_prelude_tokens RPAREN ;
paren_block = LPAREN at_prelude_tokens RPAREN ;
at_prelude_tokens = { at_prelude_token } ;

# ============================================================================
# CSS: Qualified Rules
# ============================================================================

qualified_rule = selector_list block ;

# ============================================================================
# CSS: Selectors
# ============================================================================

selector_list = complex_selector { COMMA complex_selector } ;

complex_selector = compound_selector { [ combinator ] compound_selector } ;

combinator = GREATER | PLUS | TILDE ;

compound_selector = simple_selector { subclass_selector }
                  | subclass_selector { subclass_selector } ;

simple_selector = IDENT | STAR | AMPERSAND ;

subclass_selector = class_selector | id_selector
                  | attribute_selector | pseudo_class
                  | pseudo_element ;

class_selector = DOT IDENT ;

id_selector = HASH ;

attribute_selector = LBRACKET IDENT [ attr_matcher attr_value [ IDENT ] ] RBRACKET ;

attr_matcher = EQUALS | TILDE_EQUALS | PIPE_EQUALS
             | CARET_EQUALS | DOLLAR_EQUALS | STAR_EQUALS ;

attr_value = IDENT | STRING ;

pseudo_class = COLON FUNCTION pseudo_class_args RPAREN
             | COLON IDENT ;

pseudo_class_args = { pseudo_class_arg } ;

pseudo_class_arg = IDENT | NUMBER | DIMENSION | STRING | HASH
                 | PLUS | COMMA | DOT | STAR | COLON | AMPERSAND
                 | FUNCTION pseudo_class_args RPAREN
                 | LBRACKET pseudo_class_args RBRACKET ;

pseudo_element = COLON_COLON IDENT ;

# ============================================================================
# CSS: Declaration Block (extended for Lattice)
# ============================================================================

block = LBRACE block_contents RBRACE ;

block_contents = { block_item } ;

block_item = lattice_block_item | at_rule | declaration_or_nested ;

lattice_block_item = variable_declaration
                   | include_directive
                   | lattice_control ;

declaration_or_nested = declaration | qualified_rule ;

# ============================================================================
# CSS: Declarations
# ============================================================================

declaration = property COLON value_list [ priority ] SEMICOLON ;

property = IDENT | CUSTOM_PROPERTY ;

priority = BANG "important" ;

# ============================================================================
# CSS: Values (extended with VARIABLE for Lattice)
# ============================================================================

value_list = value { value } ;

value = DIMENSION | PERCENTAGE | NUMBER | STRING | IDENT | HASH
      | CUSTOM_PROPERTY | UNICODE_RANGE | function_call
      | VARIABLE
      | SLASH | COMMA | PLUS | MINUS ;

function_call = FUNCTION function_args RPAREN
              | URL_TOKEN ;

function_args = { function_arg } ;

function_arg = DIMENSION | PERCENTAGE | NUMBER | STRING | IDENT | HASH
             | CUSTOM_PROPERTY | COMMA | SLASH | PLUS | MINUS | STAR
             | VARIABLE
             | FUNCTION function_args RPAREN ;
`;

// =============================================================================
// Cached Grammar Objects
// =============================================================================
//
// Parsing the grammar text on every call would be wasteful. We parse once
// at module load time and cache the results. This is safe because the grammar
// is fixed (embedded as string constants above).
//
// The lazy singleton pattern avoids parsing on module import (which could
// slow down startup). Grammars are parsed on the first call to
// transpileLatticeInBrowser() and reused for all subsequent calls.

let _cachedTokenGrammar: ReturnType<typeof parseTokenGrammar> | null = null;
let _cachedParserGrammar: ReturnType<typeof parseParserGrammar> | null = null;

/** Get or create the cached TokenGrammar. */
function getTokenGrammar(): ReturnType<typeof parseTokenGrammar> {
  if (!_cachedTokenGrammar) {
    _cachedTokenGrammar = parseTokenGrammar(LATTICE_TOKENS_GRAMMAR);
  }
  return _cachedTokenGrammar;
}

/** Get or create the cached ParserGrammar. */
function getParserGrammar(): ReturnType<typeof parseParserGrammar> {
  if (!_cachedParserGrammar) {
    _cachedParserGrammar = parseParserGrammar(LATTICE_PARSER_GRAMMAR);
  }
  return _cachedParserGrammar;
}

// =============================================================================
// Browser-Compatible Pipeline Internals
// =============================================================================

/**
 * Tokenize Lattice source text using the embedded token grammar.
 *
 * Unlike the Node.js `lattice-lexer`, this function does not read files.
 * It uses the `LATTICE_TOKENS_GRAMMAR` constant instead.
 *
 * @param source - Lattice source text.
 * @returns Array of Token objects.
 */
function tokenizeBrowser(source: string) {
  const tokenGrammar = getTokenGrammar();
  return grammarTokenize(source, tokenGrammar);
}

/**
 * Parse Lattice tokens into an AST using the embedded parser grammar.
 *
 * Unlike the Node.js `lattice-parser`, this function does not read files.
 * It uses the `LATTICE_PARSER_GRAMMAR` constant instead.
 *
 * @param source - Lattice source text.
 * @returns An ASTNode with ruleName "stylesheet".
 */
function parseBrowser(source: string) {
  const tokens = tokenizeBrowser(source);
  const parserGrammar = getParserGrammar();
  const parser = new GrammarParser(tokens, parserGrammar);
  return parser.parse();
}

// =============================================================================
// Public API
// =============================================================================

/**
 * Transpile Lattice source text to CSS in a browser environment.
 *
 * This function is the browser-compatible equivalent of `transpileLattice()`.
 * It uses embedded grammar strings instead of reading grammar files from disk.
 *
 * The pipeline is:
 * 1. Tokenize with embedded `lattice.tokens` grammar.
 * 2. Parse with embedded `lattice.grammar` rules.
 * 3. Transform with `LatticeTransformer` (three passes).
 * 4. Emit with `CSSEmitter`.
 *
 * @param source - Lattice source text.
 * @param options - Optional formatting options (minified, indent).
 * @returns CSS text string. Empty string for empty/whitespace-only input.
 *
 * @example
 *     const css = transpileLatticeInBrowser(`
 *       $brand: #4a90d9;
 *       h1 { color: $brand; }
 *     `);
 *     // → "h1 {\n  color: #4a90d9;\n}\n"
 *
 * @example
 *     const minCss = transpileLatticeInBrowser(
 *       "$c: red; p { color: $c; }",
 *       { minified: true }
 *     );
 *     // → "p{color:red;}"
 */
export function transpileLatticeInBrowser(
  source: string,
  options: TranspileOptions = {}
): string {
  return transpileLattice(source, options);
}

// Re-export everything from the main entry point for backward compatibility
export type { TranspileOptions };

export {
  LatticeError,
  LatticeModuleNotFoundError,
  ReturnOutsideFunctionError,
  UndefinedVariableError,
  UndefinedMixinError,
  UndefinedFunctionError,
  WrongArityError,
  CircularReferenceError,
  TypeErrorInExpression,
  UnitMismatchError,
  MissingReturnError,
} from "@coding-adventures/lattice-ast-to-css";

export const VERSION = "0.1.0";
