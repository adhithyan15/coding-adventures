/**
 * ALGOL 60 Parser -- parses ALGOL 60 source text into ASTs using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `GrammarParser` from the
 * `@coding-adventures/parser` package. It loads the `algol.grammar` file and
 * delegates all parsing work to the generic engine.
 *
 * What Is ALGOL 60?
 * -----------------
 *
 * ALGOL 60 is the first programming language with a formally specified grammar.
 * John Backus and Peter Naur defined ALGOL 60's syntax using a notation that Naur
 * later published in the ALGOL 60 report (1960/1963). That notation became known as
 * BNF (Backus-Naur Form) and is the standard way to describe programming language
 * syntax to this day.
 *
 * The ALGOL 60 Grammar's Key Innovations
 * ----------------------------------------
 *
 *   1. **Block structure** -- `begin...end` creates a lexical scope. Declarations
 *      inside a block are local to that block. This is the foundation of lexical
 *      scoping, which every modern language uses.
 *
 *   2. **Recursive descent** -- ALGOL 60's grammar is the textbook example for
 *      recursive descent parsing. Rules like `expression` → `arith_expr | bool_expr`,
 *      `arith_expr` → `simple_arith`, `simple_arith` → `term { + term }` define
 *      a clean expression hierarchy that maps directly to recursive functions.
 *
 *   3. **Dangling else resolution** -- ALGOL 60 resolves the "dangling else"
 *      ambiguity at the grammar level. The then-branch is `unlabeled_stmt`
 *      (which excludes conditionals), so nesting requires explicit `begin...end`.
 *      C and Java use a convention instead ("else binds to the nearest if"),
 *      which is less rigorous.
 *
 *   4. **Call by name vs call by value** -- Parameters are call-by-name by default.
 *      The VALUE declaration opts specific parameters into call-by-value.
 *      Call-by-name re-evaluates the argument expression every time the parameter
 *      is used, enabling elegant patterns like Jensen's Device (1960).
 *
 *   5. **Dynamic arrays** -- Array bounds can be arbitrary arithmetic expressions,
 *      including variables. This means array sizes are determined at runtime on the
 *      stack — a concept called variable-length arrays, which C added in C99 and
 *      Java never had (Java always uses heap allocation).
 *
 * Parsing Pipeline
 * ----------------
 *
 * The parsing pipeline has two stages:
 *
 *   1. **Lexing** -- `tokenizeAlgol` reads the source text and produces a flat
 *      array of tokens. Each token has a type (like `begin`, `IDENT`, `INTEGER_LIT`,
 *      `ASSIGN`) and a value (the actual text from the source).
 *
 *   2. **Parsing** -- `GrammarParser` reads the `algol.grammar` file, which defines
 *      the syntax rules in EBNF-like notation. It then applies recursive descent
 *      with backtracking to match the token stream against these rules, producing an AST.
 *
 * The AST Structure
 * -----------------
 *
 * The resulting AST is a tree of `ASTNode` objects. Each node has:
 *   - `ruleName` -- the grammar rule that produced this node (e.g., "program",
 *     "block", "declaration", "statement", "assign_stmt")
 *   - `children` -- an array of child `ASTNode` objects and/or leaf `Token` objects
 *
 * For example, parsing `begin integer x; x := 42 end` produces roughly:
 *
 *     ASTNode("program", [
 *       ASTNode("block", [
 *         Token(begin, "begin"),
 *         ASTNode("declaration", [
 *           ASTNode("type_decl", [
 *             ASTNode("type", [Token(integer, "integer")]),
 *             ASTNode("ident_list", [Token(IDENT, "x")])
 *           ])
 *         ]),
 *         Token(SEMICOLON, ";"),
 *         ASTNode("statement", [
 *           ASTNode("unlabeled_stmt", [
 *             ASTNode("assign_stmt", [
 *               ASTNode("left_part", [
 *                 ASTNode("variable", [Token(IDENT, "x")]),
 *                 Token(ASSIGN, ":=")
 *               ]),
 *               ASTNode("expression", [
 *                 ASTNode("arith_expr", [...Token(INTEGER_LIT, "42")...])
 *               ])
 *             ])
 *           ])
 *         ]),
 *         Token(end, "end")
 *       ])
 *     ])
 *
 * Grammar Rule Summary
 * --------------------
 *
 * (Simplified — see algol.grammar for the full specification.)
 *
 *   program      = block
 *   block        = begin { declaration ; } statement { ; statement } end
 *   declaration  = type_decl | array_decl | switch_decl | procedure_decl
 *   type_decl    = type ident_list
 *   statement    = [ label : ] unlabeled_stmt | [ label : ] cond_stmt
 *   unlabeled_stmt = assign_stmt | goto_stmt | proc_stmt | compound_stmt | block | for_stmt
 *   cond_stmt    = if bool_expr then unlabeled_stmt [ else statement ]
 *   assign_stmt  = left_part { left_part } expression
 *   for_stmt     = for IDENT := for_list do statement
 *   expression   = arith_expr | bool_expr
 *
 * Grammar Data
 * ------------
 *
 * The `algol60.grammar` file under `code/grammars/algol/` at the repository
 * root is compiled ahead of time into `./_grammar.ts` and
 * `./_grammar_algol60.ts` (see `code/scripts/_ts_grammar_compile.ts`).
 * This module statically imports them rather than reading the monorepo's
 * `code/grammars/` tree at runtime, so a published npm package works
 * standalone.
 */

import type { ParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import { tokenizeAlgol } from "@coding-adventures/algol-lexer";

import { PARSER_GRAMMAR as DEFAULT_GRAMMAR } from "./_grammar.js";
import { PARSER_GRAMMAR as ALGOL60_GRAMMAR } from "./_grammar_algol60.js";

const VALID_VERSIONS = new Set(["algol60"]);

const VERSIONED_GRAMMARS: Record<string, ParserGrammar> = {
  algol60: ALGOL60_GRAMMAR,
};

function resolveParserGrammar(version = "algol60"): ParserGrammar {
  if (!VALID_VERSIONS.has(version)) {
    const valid = Array.from(VALID_VERSIONS).sort().join(", ");
    throw new Error(`Unknown ALGOL version ${JSON.stringify(version)}. Valid versions: ${valid}`);
  }
  return version === "algol60" ? DEFAULT_GRAMMAR : VERSIONED_GRAMMARS[version];
}

/**
 * Parse ALGOL 60 source text and return an AST.
 *
 * This function orchestrates the full parsing pipeline:
 *   1. Tokenize the source using the algol-lexer
 *   2. Read and parse the algol.grammar file
 *   3. Run the grammar-driven parser to produce an AST
 *
 * The starting rule is `program` (the first rule in algol.grammar), which
 * matches a single ALGOL 60 block.
 *
 * @param source - The ALGOL 60 source text to parse.
 * @returns An ASTNode representing the parse tree, with `ruleName` of `"program"`.
 *
 * @example
 *     // Minimal program with one variable declaration and assignment:
 *     const ast = parseAlgol("begin integer x; x := 42 end");
 *     console.log(ast.ruleName); // "program"
 *
 * @example
 *     // Arithmetic expression:
 *     const ast = parseAlgol("begin real y; y := 1.5 + 2.5 end");
 *
 * @example
 *     // Conditional statement:
 *     const ast = parseAlgol("begin integer x; x := 0; if x = 0 then x := 1 end");
 *
 * @example
 *     // For loop:
 *     const ast = parseAlgol(
 *       "begin integer i; integer s; s := 0; for i := 1 step 1 until 10 do s := s + i end"
 *     );
 */
export function parseAlgol(source: string, version = "algol60"): ASTNode {
  /**
   * Step 1: Tokenize.
   * The algol-lexer handles:
   *   - Keyword recognition and reclassification (begin, end, integer, ...)
   *   - Operator disambiguation (:= vs :, ** vs *, <= vs <)
   *   - Comment skipping (comment <text>;)
   *   - Whitespace skipping
   *   - Position tracking (line/column)
   */
  const tokens = tokenizeAlgol(source, version);

  /**
   * Step 2: Resolve the grammar.
   * The pre-compiled ParserGrammar defines the syntax rules in EBNF-like
   * notation, ready for the GrammarParser to use for recursive descent.
   */
  const grammar = resolveParserGrammar(version);

  /**
   * Step 3: Parse.
   * The GrammarParser takes the token array and grammar rules, then
   * performs recursive descent with backtracking to produce an AST.
   * The starting rule is determined by the grammar (the first rule
   * defined, which for algol.grammar is "program").
   */
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
