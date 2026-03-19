/**
 * Grammar-Driven Parser — Parsing from .grammar Files
 * ======================================================
 *
 * Instead of hardcoding grammar rules as TypeScript methods (one method per rule),
 * this parser reads grammar rules from a .grammar file and interprets them
 * at runtime. The same TypeScript code can parse Python, Ruby, or any language —
 * just swap the .grammar file.
 *
 * This is how tools like ANTLR and Yacc work: you define a grammar, the tool
 * generates (or interprets) a parser. We're doing the same thing at runtime.
 *
 * =============================================================================
 * WHY A GRAMMAR-DRIVEN PARSER?
 * =============================================================================
 *
 * The hand-written ``Parser`` in ``parser.ts`` is great for learning — you can
 * see exactly how each grammar rule maps to a TypeScript method. But it has a
 * limitation: the grammar is *baked into the code*. If you want to change the
 * grammar (add a new operator, a new statement type), you have to modify
 * TypeScript code.
 *
 * A grammar-driven parser separates the *grammar* from the *parsing engine*.
 * The grammar lives in a ``.grammar`` file (like ``python.grammar``), and the
 * parsing engine reads that file and follows its rules. To support a new
 * language, you write a new ``.grammar`` file — no TypeScript changes needed.
 *
 * This separation of concerns is the same principle behind:
 *
 * - **ANTLR**: You write a ``.g4`` grammar file, ANTLR generates a parser.
 * - **Yacc/Bison**: You write a ``.y`` grammar file, the tool generates C code.
 * - **PEG parsers**: You write a PEG grammar, the runtime interprets it.
 *
 * Our approach is closest to PEG (Parsing Expression Grammars) — we interpret
 * the grammar at runtime with backtracking, rather than generating code ahead
 * of time.
 *
 * =============================================================================
 * HOW IT WORKS
 * =============================================================================
 *
 * The ``GrammarParser`` receives two inputs:
 *
 * 1. A ``ParserGrammar`` — the parsed representation of a ``.grammar`` file,
 *    produced by ``parseParserGrammar()`` from grammar-tools. This contains a
 *    list of ``GrammarRule`` objects, each with a name and a body (a tree of
 *    EBNF elements like ``Sequence``, ``Alternation``, ``Repetition``, etc.).
 *
 * 2. A list of ``Token`` objects — the output of the lexer.
 *
 * The parser walks the grammar rule tree, trying to match each element against
 * the token stream. The key insight is that each EBNF element type has a
 * natural interpretation:
 *
 * - **RuleReference** (lowercase, e.g., ``expression``): Recursively parse
 *   that grammar rule.
 * - **TokenReference** (uppercase, e.g., ``NUMBER``): Match a token of that type.
 * - **Sequence** (``A B C``): Match A, then B, then C — all must succeed.
 * - **Alternation** (``A | B | C``): Try A first; if it fails, backtrack and
 *   try B; if B fails, try C.
 * - **Repetition** (``{ A }``): Match A zero or more times.
 * - **Optional** (``[ A ]``): Match A zero or one time.
 * - **Literal** (``"++"``): Match a token whose text value is exactly that string.
 * - **Group** (``( A )``): Just a parenthesized sub-expression.
 *
 * =============================================================================
 * BACKTRACKING
 * =============================================================================
 *
 * When an ``Alternation`` tries its first choice and it fails, the parser needs
 * to "undo" any tokens it consumed during that failed attempt. This is called
 * **backtracking**. We implement it by saving the position before each attempt
 * and restoring it on failure.
 *
 * Backtracking makes the parser simple but potentially slow for ambiguous
 * grammars (exponential in the worst case). For the grammars we use — which are
 * designed for predictive parsing — backtracking rarely goes more than a few
 * tokens deep.
 *
 * =============================================================================
 * GENERIC AST NODES
 * =============================================================================
 *
 * Unlike the hand-written parser (which produces specific nodes like
 * ``NumberLiteral``, ``BinaryOp``, etc.), the grammar-driven parser produces
 * **generic** ``ASTNode`` objects. Each node records:
 *
 * - ``ruleName``: Which grammar rule produced it (e.g., "expression", "factor").
 * - ``children``: The matched elements — a mix of sub-nodes and raw tokens.
 *
 * This makes the grammar-driven parser language-agnostic. The same ``ASTNode``
 * type works for Python, Ruby, JavaScript, or any language whose grammar is
 * written in a ``.grammar`` file.
 *
 * The trade-off is that consumers (like a bytecode compiler) need to inspect
 * ``ruleName`` and walk ``children`` to extract meaning, rather than pattern-
 * matching on specific node types. An adapter layer can bridge this gap if
 * needed.
 */

import type { Token } from "@coding-adventures/lexer";
import type {
  GrammarElement,
  GrammarRule,
  ParserGrammar,
} from "@coding-adventures/grammar-tools";

// =============================================================================
// GENERIC AST NODES
// =============================================================================
//
// These are the building blocks of the grammar-driven AST. Unlike the hand-
// written parser's specific node types (NumberLiteral, BinaryOp, etc.), these
// are generic containers that work for any grammar.
//
// Think of it like the difference between a custom-built bookshelf (hand-
// written parser) and a modular shelving system (grammar-driven parser).
// The custom bookshelf fits perfectly but only holds books of certain sizes.
// The modular system adapts to anything, but you need to label the shelves
// yourself.
// =============================================================================

/**
 * A generic AST node produced by grammar-driven parsing.
 *
 * Every node in the grammar-driven AST is an instance of this interface.
 * The ``ruleName`` tells you *what* grammar rule created the node, and
 * ``children`` contains the matched sub-structure.
 *
 * For example, parsing ``1 + 2`` with a grammar rule like:
 *
 *     expression = term { ( PLUS | MINUS ) term } ;
 *
 * Might produce:
 *
 *     ASTNode {
 *         ruleName: "expression",
 *         children: [
 *             ASTNode { ruleName: "term", children: [Token(NUMBER, "1")] },
 *             Token(PLUS, "+"),
 *             ASTNode { ruleName: "term", children: [Token(NUMBER, "2")] },
 *         ]
 *     }
 *
 * Properties:
 *     ruleName: Which grammar rule produced this node (e.g., "expression",
 *         "assignment", "factor"). This is the key for interpreting the node.
 *     children: The matched elements — a mix of ``ASTNode`` (from parsing
 *         sub-rules) and ``Token`` (from matching token types or literals).
 */
export interface ASTNode {
  readonly ruleName: string;
  readonly children: ReadonlyArray<ASTNode | Token>;
}

/**
 * Check if a child element is an ASTNode (not a Token).
 *
 * Since both ASTNode and Token are plain objects, we distinguish them
 * by checking for the presence of the ``ruleName`` property. Tokens
 * have ``type`` and ``value``; ASTNodes have ``ruleName`` and ``children``.
 *
 * @param child - An element from an ASTNode's children array.
 * @returns True if the child is an ASTNode, false if it's a Token.
 */
export function isASTNode(child: ASTNode | Token): child is ASTNode {
  return "ruleName" in child;
}

/**
 * Check if an ASTNode is a leaf node (wraps a single token).
 *
 * Leaf nodes typically come from grammar rules like:
 *
 *     factor = NUMBER | STRING | NAME ;
 *
 * Where the rule matches exactly one token. This function is a
 * convenience for code that needs to distinguish leaf nodes from
 * interior nodes.
 *
 * @param node - The ASTNode to check.
 * @returns True if ``children`` contains exactly one element and that
 *          element is a Token.
 */
export function isLeafNode(node: ASTNode): boolean {
  return node.children.length === 1 && !isASTNode(node.children[0]);
}

/**
 * Get the token from a leaf node, or null for non-leaf nodes.
 *
 * This is a shortcut for the common pattern of checking isLeafNode()
 * and then accessing children[0]. It returns null for non-leaf
 * nodes so callers can use it safely without checking first.
 *
 * @param node - The ASTNode to extract a token from.
 * @returns The Token if this is a leaf node, null otherwise.
 */
export function getLeafToken(node: ASTNode): Token | null {
  if (isLeafNode(node) && !isASTNode(node.children[0])) {
    return node.children[0] as Token;
  }
  return null;
}

// =============================================================================
// PARSE ERROR
// =============================================================================

/**
 * Error during grammar-driven parsing.
 *
 * Raised when the grammar-driven parser encounters a token that doesn't
 * match any of the expected grammar alternatives. Like the hand-written
 * parser's ``ParseError``, this includes the problematic token so error
 * messages can report the exact location.
 *
 * Properties:
 *     token: The token where the error was detected, or null if at EOF.
 */
export class GrammarParseError extends Error {
  public readonly token: Token | null;

  constructor(message: string, token?: Token | null) {
    if (token) {
      super(`Parse error at ${token.line}:${token.column}: ${message}`);
    } else {
      super(`Parse error: ${message}`);
    }
    this.name = "GrammarParseError";
    this.token = token ?? null;
  }
}

// =============================================================================
// THE GRAMMAR-DRIVEN PARSER
// =============================================================================
//
// This is the heart of the module. The GrammarParser takes a ParserGrammar
// (the parsed .grammar file) and a token list, and produces an AST by
// interpreting the grammar rules at runtime.
//
// The key method is matchElement(), which dispatches on the type of
// grammar element (Sequence, Alternation, etc.) and recursively matches
// the token stream. This is essentially a tree-walking interpreter for
// EBNF grammars.
// =============================================================================

/**
 * A parser driven by a ParserGrammar (parsed from a .grammar file).
 *
 * This parser interprets EBNF grammar rules at runtime, using backtracking
 * to handle alternations. It produces a tree of generic ``ASTNode`` objects.
 *
 * ==========================================================================
 * HOW TO USE
 * ==========================================================================
 *
 * 1. Parse a ``.grammar`` file using ``parseParserGrammar()`` from grammar-tools.
 * 2. Tokenize source code using ``tokenize()`` from the lexer.
 * 3. Feed both to ``GrammarParser`` and call ``.parse()``.
 *
 * Example:
 *
 *     import { parseParserGrammar } from "@coding-adventures/grammar-tools";
 *     import { tokenize } from "@coding-adventures/lexer";
 *
 *     const grammarText = fs.readFileSync("python.grammar", "utf-8");
 *     const grammar = parseParserGrammar(grammarText);
 *
 *     const tokens = tokenize("x = 1 + 2");
 *     const parser = new GrammarParser(tokens, grammar);
 *     const ast = parser.parse();
 *
 * ==========================================================================
 * GRAMMAR ELEMENT INTERPRETATION
 * ==========================================================================
 *
 * Each EBNF element type is interpreted as follows:
 *
 * - **TokenReference** (uppercase, e.g., ``NUMBER``): Match a token whose
 *   type matches. Skips newlines automatically unless the reference is
 *   to ``NEWLINE`` itself.
 *
 * - **RuleReference** (lowercase, e.g., ``expression``): Recursively parse
 *   the named grammar rule. If parsing fails, backtrack.
 *
 * - **Sequence** (``A B C``): Match all elements in order. If any fails,
 *   the whole sequence fails and we backtrack.
 *
 * - **Alternation** (``A | B | C``): Try each choice in order. The first
 *   one that succeeds wins. Failed choices backtrack automatically.
 *
 * - **Repetition** (``{ A }``): Match zero or more occurrences of A.
 *   Always succeeds (zero matches is fine). Stops when A fails to match.
 *
 * - **Optional** (``[ A ]``): Match zero or one occurrence of A.
 *   Always succeeds (no match returns an empty list).
 *
 * - **Literal** (``"+"``): Match a token whose ``.value`` equals the
 *   literal string exactly.
 *
 * - **Group** (``( A )``): Just delegates to the sub-element. Groups
 *   exist only for syntactic clarity in the grammar.
 */
export class GrammarParser {
  /**
   * The complete list of tokens from the lexer.
   */
  private readonly tokens: readonly Token[];

  /**
   * The parsed grammar (from a .grammar file).
   */
  private readonly grammar: ParserGrammar;

  /**
   * Current position in the token list.
   */
  private pos: number;

  /**
   * Lookup dict mapping rule names to GrammarRule objects.
   */
  private readonly rules: ReadonlyMap<string, GrammarRule>;

  /**
   * Initialize the grammar-driven parser.
   *
   * @param tokens - A list of Token objects from the lexer, typically ending
   *     with an EOF token.
   * @param grammar - A ParserGrammar containing the rules to parse by.
   */
  constructor(tokens: readonly Token[], grammar: ParserGrammar) {
    this.tokens = tokens;
    this.grammar = grammar;
    this.pos = 0;
    // Build a lookup map: rule_name -> GrammarRule
    // This lets us find any rule in O(1) time when we encounter a
    // RuleReference during parsing.
    const ruleMap = new Map<string, GrammarRule>();
    for (const rule of grammar.rules) {
      ruleMap.set(rule.name, rule);
    }
    this.rules = ruleMap;
  }

  /**
   * Parse the token stream using the first grammar rule as entry point.
   *
   * The first rule in a ``.grammar`` file is always the start symbol
   * (the top-level rule). For our Python subset, this is ``program``.
   *
   * After parsing the start rule, we verify that all tokens have been
   * consumed (except for trailing newlines and the EOF token). If there
   * are leftover tokens, something went wrong — the grammar didn't
   * account for all the input.
   *
   * @returns An ASTNode representing the complete parse tree.
   * @throws GrammarParseError if the grammar has no rules, the input doesn't
   *     match the grammar, or there are unconsumed tokens.
   */
  parse(): ASTNode {
    if (this.grammar.rules.length === 0) {
      throw new GrammarParseError("Grammar has no rules");
    }

    const entryRule = this.grammar.rules[0];
    const result = this.parseRule(entryRule.name);

    // Skip trailing newlines — these are insignificant whitespace
    // that often appears at the end of source files.
    while (
      this.pos < this.tokens.length &&
      this.current().type === "NEWLINE"
    ) {
      this.pos++;
    }

    // Verify we consumed all tokens (except EOF).
    // If there are leftover tokens, the grammar didn't fully describe
    // the input — there's something the parser doesn't understand.
    if (
      this.pos < this.tokens.length &&
      this.current().type !== "EOF"
    ) {
      throw new GrammarParseError(
        `Unexpected token: ${JSON.stringify(this.current().value)}`,
        this.current(),
      );
    }

    return result;
  }

  // =========================================================================
  // HELPERS
  // =========================================================================

  /**
   * Get the current token without consuming it.
   *
   * If we're past the end of the token list, returns the last token
   * (which should be EOF). This prevents index-out-of-bounds errors.
   *
   * @returns The token at the current position.
   */
  private current(): Token {
    if (this.pos < this.tokens.length) {
      return this.tokens[this.pos];
    }
    return this.tokens[this.tokens.length - 1]; // EOF
  }

  // =========================================================================
  // RULE PARSING
  // =========================================================================

  /**
   * Parse a named grammar rule.
   *
   * This is the entry point for parsing any grammar rule. It looks up
   * the rule by name, matches its body against the token stream, and
   * wraps the result in an ``ASTNode`` tagged with the rule name.
   *
   * @param ruleName - The name of the grammar rule to parse.
   * @returns An ASTNode with ``ruleName`` set to the rule name and
   *     ``children`` containing all matched elements.
   * @throws GrammarParseError if the rule is undefined or the token stream
   *     doesn't match the rule's body.
   */
  private parseRule(ruleName: string): ASTNode {
    const rule = this.rules.get(ruleName);
    if (!rule) {
      throw new GrammarParseError(`Undefined rule: ${ruleName}`);
    }

    const children = this.matchElement(rule.body);

    if (children === null) {
      throw new GrammarParseError(
        `Expected ${ruleName}, got ${JSON.stringify(this.current().value)}`,
        this.current(),
      );
    }

    return { ruleName, children };
  }

  // =========================================================================
  // ELEMENT MATCHING — THE CORE OF THE GRAMMAR INTERPRETER
  // =========================================================================
  //
  // matchElement() is the workhorse of the grammar-driven parser. It
  // takes a single grammar element (Sequence, Alternation, etc.) and tries
  // to match it against the token stream starting at the current position.
  //
  // It returns either:
  //   - A list of matched children (tokens and sub-nodes) on SUCCESS
  //   - null on FAILURE (no match)
  //
  // On failure, the position is restored to where it was before the attempt
  // (backtracking). This is critical for Alternation — if the first choice
  // fails, we need to try the second choice from the same position.
  //
  // The method dispatches on the `type` field of the grammar element using
  // a switch statement. Each element type has its own matching logic,
  // documented inline.
  // =========================================================================

  /**
   * Try to match a grammar element against the token stream.
   *
   * This is a recursive interpreter for EBNF grammar elements. It
   * handles each element type differently:
   *
   * - **Sequence**: Match all sub-elements in order; fail if any fails.
   * - **Alternation**: Try each choice; succeed on first match.
   * - **Repetition**: Match zero or more times; always succeeds.
   * - **Optional**: Match zero or one time; always succeeds.
   * - **Group**: Delegate to the inner element.
   * - **TokenReference**: Match a token by type.
   * - **RuleReference**: Recursively parse the named rule.
   * - **Literal**: Match a token by exact text value.
   *
   * @param element - A grammar element from the parsed grammar.
   * @returns A list of matched children (ASTNode and Token objects) on
   *     success, or null if the element doesn't match. Position is
   *     restored on failure.
   */
  private matchElement(
    element: GrammarElement,
  ): Array<ASTNode | Token> | null {
    const savePos = this.pos;

    switch (element.type) {
      // -----------------------------------------------------------------
      // SEQUENCE: A B C — match all elements in order
      // -----------------------------------------------------------------
      // A sequence succeeds only if ALL its elements match in order.
      // If any element fails, the entire sequence fails and we restore
      // the position to before the first element.
      //
      // Example: The grammar rule ``NAME EQUALS expression`` is a
      // Sequence of three elements. We must match a NAME token, then
      // an EQUALS token, then the expression sub-rule — in that order.
      case "sequence": {
        const children: Array<ASTNode | Token> = [];
        for (const sub of element.elements) {
          const result = this.matchElement(sub);
          if (result === null) {
            this.pos = savePos;
            return null;
          }
          children.push(...result);
        }
        return children;
      }

      // -----------------------------------------------------------------
      // ALTERNATION: A | B | C — try each choice, first wins
      // -----------------------------------------------------------------
      // An alternation tries each choice in order. The first choice that
      // matches wins. If a choice fails, we restore the position and try
      // the next one. If ALL choices fail, the alternation fails.
      //
      // This is where backtracking happens. For example, in the grammar:
      //   statement = assignment | expression_stmt ;
      //
      // We first try to parse an assignment. If that fails (because the
      // input isn't NAME EQUALS ...), we backtrack and try expression_stmt.
      case "alternation": {
        for (const choice of element.choices) {
          this.pos = savePos;
          const result = this.matchElement(choice);
          if (result !== null) {
            return result;
          }
        }
        this.pos = savePos;
        return null;
      }

      // -----------------------------------------------------------------
      // REPETITION: { A } — zero or more matches
      // -----------------------------------------------------------------
      // A repetition matches its element as many times as possible, then
      // stops. It ALWAYS succeeds — zero matches is fine. This implements
      // the Kleene star (*) from regular expressions.
      //
      // Example: ``{ statement }`` matches zero or more statements.
      // We keep parsing statements until we can't parse another one.
      case "repetition": {
        const children: Array<ASTNode | Token> = [];
        while (true) {
          const saveRep = this.pos;
          const result = this.matchElement(element.element);
          if (result === null) {
            this.pos = saveRep;
            break;
          }
          children.push(...result);
        }
        return children; // Always succeeds (zero matches is fine)
      }

      // -----------------------------------------------------------------
      // OPTIONAL: [ A ] — zero or one match
      // -----------------------------------------------------------------
      // An optional element matches zero or one time. Like repetition,
      // it always succeeds — if the element doesn't match, we return
      // an empty list (no children).
      //
      // Example: ``[ ELSE block ]`` optionally matches an else clause.
      case "optional": {
        const result = this.matchElement(element.element);
        if (result === null) {
          return []; // Optional: no match is fine
        }
        return result;
      }

      // -----------------------------------------------------------------
      // GROUP: ( A ) — just a parenthesized sub-expression
      // -----------------------------------------------------------------
      // Groups exist purely for syntactic clarity in the grammar file.
      // ``( PLUS | MINUS )`` groups the alternation so it can be used as
      // a single element in a sequence. We just delegate to the inner
      // element.
      case "group":
        return this.matchElement(element.element);

      // -----------------------------------------------------------------
      // TOKEN REFERENCE (uppercase) — match a token by type
      // -----------------------------------------------------------------
      // A TokenReference like ``NUMBER`` means "match a token whose type
      // is NUMBER." We compare the token's type string against the
      // reference name.
      //
      // Special handling: we skip NEWLINE tokens when looking for non-
      // NEWLINE token types. This is because newlines are significant
      // as statement terminators but should be transparent within
      // expressions. For example, in multi-line expressions or when the
      // grammar doesn't explicitly include NEWLINE tokens.
      case "token_reference": {
        let token = this.current();

        // Skip newlines when matching non-NEWLINE tokens.
        // Newlines are statement terminators, not part of expressions.
        while (
          token.type === "NEWLINE" &&
          element.name !== "NEWLINE"
        ) {
          this.pos++;
          token = this.current();
        }

        if (token.type === element.name) {
          this.pos++;
          return [token];
        }

        return null;
      }

      // -----------------------------------------------------------------
      // RULE REFERENCE (lowercase) — parse another grammar rule
      // -----------------------------------------------------------------
      // A RuleReference like ``expression`` means "recursively parse the
      // grammar rule named expression." If parsing fails, we catch the
      // error and backtrack.
      case "rule_reference": {
        try {
          const node = this.parseRule(element.name);
          return [node];
        } catch (e) {
          if (e instanceof GrammarParseError) {
            this.pos = savePos;
            return null;
          }
          throw e;
        }
      }

      // -----------------------------------------------------------------
      // LITERAL — match a token by exact text value
      // -----------------------------------------------------------------
      // A literal like ``"++"`` matches a token whose .value equals
      // the literal string. This is less common than token references
      // but useful for matching specific keywords or symbols that don't
      // have their own token type.
      case "literal": {
        const token = this.current();
        if (token.value === element.value) {
          this.pos++;
          return [token];
        }
        return null;
      }

      // If we get here, we encountered an unknown grammar element type.
      // This shouldn't happen if the grammar was produced by grammar_tools.
      default:
        return null; // istanbul ignore next
    }
  }
}
