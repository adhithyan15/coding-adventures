/**
 * Grammar-Driven Parser — Parsing from .grammar Files
 * ======================================================
 *
 * Instead of hardcoding grammar rules as TypeScript methods (one method per rule),
 * this parser reads grammar rules from a .grammar file and interprets them
 * at runtime. The same TypeScript code can parse Python, Ruby, or any language —
 * just swap the .grammar file.
 *
 * =============================================================================
 * EXTENSIONS FOR STARLARK-LIKE LANGUAGES
 * =============================================================================
 *
 * This parser supports several extensions beyond basic EBNF interpretation:
 *
 * - **Packrat memoization**: Caches parse results for each (rule, position) pair,
 *   avoiding exponential backtracking. Essential for grammars with ~40 rules.
 *
 * - **Significant newlines**: If the grammar references NEWLINE tokens, they are
 *   treated as significant (not auto-skipped). Otherwise, NEWLINEs are transparent.
 *
 * - **Furthest failure tracking**: When parsing fails, the error message reports
 *   what was expected at the furthest position reached, not just the first failure.
 *
 * =============================================================================
 * HOW IT WORKS
 * =============================================================================
 *
 * The ``GrammarParser`` receives two inputs:
 *
 * 1. A ``ParserGrammar`` — parsed from a ``.grammar`` file.
 * 2. A list of ``Token`` objects — the output of the lexer.
 *
 * The parser walks the grammar rule tree, trying to match each element against
 * the token stream. Each EBNF element type has a natural interpretation:
 *
 * - **RuleReference** (lowercase): Recursively parse that grammar rule.
 * - **TokenReference** (uppercase): Match a token of that type.
 * - **Sequence** (``A B C``): Match A, then B, then C — all must succeed.
 * - **Alternation** (``A | B | C``): Try A; if fail, backtrack and try B.
 * - **Repetition** (``{ A }``): Match zero or more times.
 * - **Optional** (``[ A ]``): Match zero or one time.
 * - **Literal** (``"++"``): Match a token whose text value is that string.
 * - **Group** (``( A )``): Just a parenthesized sub-expression.
 */

import type { Token, Trivia } from "@coding-adventures/lexer";
import type {
  GrammarElement,
  GrammarRule,
  ParserGrammar,
} from "@coding-adventures/grammar-tools";

// =============================================================================
// GENERIC AST NODES
// =============================================================================

/**
 * A generic AST node produced by grammar-driven parsing.
 */
export interface ASTNode {
  readonly ruleName: string;
  readonly children: ReadonlyArray<ASTNode | Token>;
  readonly startLine?: number;
  readonly startColumn?: number;
  readonly endLine?: number;
  readonly endColumn?: number;
  readonly startOffset?: number;
  readonly endOffset?: number;
  readonly firstTokenIndex?: number;
  readonly lastTokenIndex?: number;
  readonly leadingTrivia?: readonly Trivia[];
}

/**
 * Check if a child element is an ASTNode (not a Token).
 */
export function isASTNode(child: ASTNode | Token): child is ASTNode {
  return "ruleName" in child;
}

/**
 * Check if an ASTNode is a leaf node (wraps a single token).
 */
export function isLeafNode(node: ASTNode): boolean {
  return node.children.length === 1 && !isASTNode(node.children[0]);
}

/**
 * Get the token from a leaf node, or null for non-leaf nodes.
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
// MEMO ENTRY — Packrat memoization cache entry
// =============================================================================

interface MemoEntry {
  readonly children: ReadonlyArray<ASTNode | Token> | null;
  readonly endPos: number;
  readonly ok: boolean;
}

// =============================================================================
// THE GRAMMAR-DRIVEN PARSER
// =============================================================================

/**
 * Options for configuring a GrammarParser instance.
 *
 * Properties:
 *   trace: When true, the parser emits a ``[TRACE]`` line to ``process.stderr``
 *       for each rule attempt, reporting the rule name, the current token
 *       index, the current token (type + value), and whether the rule
 *       matched or failed.
 *
 *       Example output::
 *
 *           [TRACE] rule 'qualified_rule' at token 5 (IDENT "h1") → match
 *           [TRACE] rule 'at_rule' at token 5 (IDENT "h1") → fail
 *
 *       Trace output goes to stderr so it does not interfere with any
 *       structured output written to stdout by the caller. This mirrors
 *       the convention used by many parser generators (yacc's ``YYDEBUG``,
 *       ANTLR's ``DiagnosticErrorListener``, etc.).
 */
export interface GrammarParserOptions {
  readonly trace?: boolean;
  readonly preserveSourceInfo?: boolean;
}

export class GrammarParser {
  private tokens: readonly Token[];
  private readonly grammar: ParserGrammar;
  private pos: number;
  private readonly rules: ReadonlyMap<string, GrammarRule>;

  /** Index of each rule name for memo key generation. */
  private readonly ruleIndex: ReadonlyMap<string, number>;

  /** Whether newlines are significant in this grammar. */
  private readonly newlinesSignificant: boolean;

  /** Packrat memoization cache: [ruleIndex, position] -> MemoEntry. */
  private readonly memo: Map<string, MemoEntry>;

  /** Furthest position reached during parsing. */
  private furthestPos: number;

  /** What was expected at the furthest position. */
  private furthestExpected: string[];

  /** Pre-parse hooks: transform token list before parsing. */
  private _preParseHooks: Array<(tokens: Token[]) => Token[]> = [];

  /** Post-parse hooks: transform AST after parsing. */
  private _postParseHooks: Array<(ast: ASTNode) => ASTNode> = [];

  /** Whether trace mode is enabled. */
  private readonly trace: boolean;

  /** Whether AST nodes should retain token-derived source info. */
  private readonly preserveSourceInfo: boolean;

  constructor(tokens: readonly Token[], grammar: ParserGrammar, options?: GrammarParserOptions) {
    this.tokens = tokens;
    this.grammar = grammar;
    this.pos = 0;
    this.memo = new Map();
    this.furthestPos = 0;
    this.furthestExpected = [];
    this.trace = options?.trace ?? false;
    this.preserveSourceInfo = options?.preserveSourceInfo ?? false;

    const ruleMap = new Map<string, GrammarRule>();
    const ruleIndex = new Map<string, number>();
    for (let i = 0; i < grammar.rules.length; i++) {
      const rule = grammar.rules[i];
      ruleMap.set(rule.name, rule);
      ruleIndex.set(rule.name, i);
    }
    this.rules = ruleMap;
    this.ruleIndex = ruleIndex;
    this.newlinesSignificant = this.grammarReferencesNewline();
  }

  /**
   * Whether newlines are treated as significant tokens in this grammar.
   */
  isNewlinesSignificant(): boolean {
    return this.newlinesSignificant;
  }

  /**
   * Register a token transform to run before parsing.
   *
   * The hook receives the token list and returns a (possibly modified)
   * token list. Multiple hooks compose left-to-right.
   */
  addPreParse(hook: (tokens: Token[]) => Token[]): void {
    this._preParseHooks.push(hook);
  }

  /**
   * Register an AST transform to run after parsing.
   *
   * The hook receives the final AST and returns a (possibly modified)
   * AST. Multiple hooks compose left-to-right.
   */
  addPostParse(hook: (ast: ASTNode) => ASTNode): void {
    this._postParseHooks.push(hook);
  }

  /**
   * Parse the token stream using the first grammar rule as entry point.
   */
  parse(): ASTNode {
    // Stage 1: Pre-parse hooks transform the token list.
    if (this._preParseHooks.length > 0) {
      let mutableTokens = [...this.tokens];
      for (const hook of this._preParseHooks) {
        mutableTokens = hook(mutableTokens);
      }
      this.tokens = mutableTokens;
    }

    if (this.grammar.rules.length === 0) {
      throw new GrammarParseError("Grammar has no rules");
    }

    const entryRule = this.grammar.rules[0];
    const result = this.parseRule(entryRule.name);

    if (result === null) {
      const tok = this.current();
      if (this.furthestExpected.length > 0) {
        const expected = this.furthestExpected.join(" or ");
        const furthestTok = this.furthestPos < this.tokens.length
          ? this.tokens[this.furthestPos]
          : tok;
        throw new GrammarParseError(
          `Expected ${expected}, got ${JSON.stringify(furthestTok.value)}`,
          furthestTok,
        );
      }
      throw new GrammarParseError("Failed to parse", tok);
    }

    // Skip trailing newlines
    while (
      this.pos < this.tokens.length &&
      this.current().type === "NEWLINE"
    ) {
      this.pos++;
    }

    // Verify all tokens consumed
    if (
      this.pos < this.tokens.length &&
      this.current().type !== "EOF"
    ) {
      const tok = this.current();
      if (this.furthestExpected.length > 0 && this.furthestPos > this.pos) {
        const expected = this.furthestExpected.join(" or ");
        const furthestTok = this.furthestPos < this.tokens.length
          ? this.tokens[this.furthestPos]
          : tok;
        throw new GrammarParseError(
          `Expected ${expected}, got ${JSON.stringify(furthestTok.value)}`,
          furthestTok,
        );
      }
      throw new GrammarParseError(
        `Unexpected token: ${JSON.stringify(tok.value)}`,
        tok,
      );
    }

    // Stage 2: Post-parse hooks transform the AST.
    let ast: ASTNode = result;
    for (const hook of this._postParseHooks) {
      ast = hook(ast);
    }

    return ast;
  }

  // =========================================================================
  // HELPERS
  // =========================================================================

  private current(): Token {
    if (this.pos < this.tokens.length) {
      return this.tokens[this.pos];
    }
    return this.tokens[this.tokens.length - 1]; // EOF
  }

  private recordFailure(expected: string): void {
    if (this.pos > this.furthestPos) {
      this.furthestPos = this.pos;
      this.furthestExpected = [expected];
    } else if (this.pos === this.furthestPos) {
      if (!this.furthestExpected.includes(expected)) {
        this.furthestExpected.push(expected);
      }
    }
  }

  // =========================================================================
  // NEWLINE DETECTION
  // =========================================================================

  private grammarReferencesNewline(): boolean {
    for (const rule of this.grammar.rules) {
      if (this.elementReferencesNewline(rule.body)) {
        return true;
      }
    }
    return false;
  }

  private elementReferencesNewline(element: GrammarElement): boolean {
    switch (element.type) {
      case "token_reference":
        return element.name === "NEWLINE";
      case "sequence":
        return element.elements.some((e) => this.elementReferencesNewline(e));
      case "alternation":
        return element.choices.some((c) => this.elementReferencesNewline(c));
      case "repetition":
      case "optional":
      case "group":
      case "positive_lookahead":
      case "negative_lookahead":
      case "one_or_more":
        return this.elementReferencesNewline(element.element);
      case "separated_repetition":
        return (
          this.elementReferencesNewline(element.element) ||
          this.elementReferencesNewline(element.separator)
        );
      default:
        return false;
    }
  }

  // =========================================================================
  // RULE PARSING (with packrat memoization)
  // =========================================================================

  private parseRule(ruleName: string): ASTNode | null {
    const rule = this.rules.get(ruleName);
    if (!rule) {
      return null;
    }

    // Check memo cache
    const idx = this.ruleIndex.get(ruleName);
    if (idx !== undefined) {
      const key = `${idx},${this.pos}`;
      const cached = this.memo.get(key);
      if (cached !== undefined) {
        this.pos = cached.endPos;
        if (!cached.ok) {
          return null;
        }
        return this.buildNode(
          ruleName,
          cached.children as ReadonlyArray<ASTNode | Token>,
        );
      }
    }

    const startPos = this.pos;

    // Left-recursion guard: seed the memo with a failure entry BEFORE
    // parsing the rule body. If the rule references itself (directly or
    // indirectly) at the same position, the memo check above will find
    // this failure entry and return null, breaking the infinite recursion.
    //
    // After the initial parse, if it succeeded, we iteratively try to
    // grow the match. This is the standard technique for handling left
    // recursion in packrat parsers (Warth et al., "Packrat Parsers Can
    // Support Left Recursion", 2008).
    if (idx !== undefined) {
      const key = `${idx},${startPos}`;
      this.memo.set(key, { children: null, endPos: startPos, ok: false });
    }

    // Emit a [TRACE] line before attempting the rule (if trace mode is on).
    // We capture the current token here, before matchElement() advances pos,
    // so the trace line shows the token *at the point of attempt*, not after.
    // Format: [TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → match|fail
    if (this.trace) {
      const tok = this.current();
      process.stderr.write(
        `[TRACE] rule '${ruleName}' at token ${startPos} (${tok.type} "${tok.value}") → `
      );
    }

    let children = this.matchElement(rule.body);

    // Emit the match/fail outcome, completing the line started above.
    if (this.trace) {
      process.stderr.write(children !== null ? "match\n" : "fail\n");
    }

    // Cache result
    if (idx !== undefined) {
      const key = `${idx},${startPos}`;
      if (children !== null) {
        this.memo.set(key, { children, endPos: this.pos, ok: true });
      } else {
        this.memo.set(key, { children: null, endPos: this.pos, ok: false });
      }

      // If the initial parse succeeded, iteratively try to grow the match.
      // Each iteration re-parses the rule body with the previous successful
      // result cached, allowing the left-recursive alternative to consume
      // more input.
      if (children !== null) {
        for (;;) {
          const prevEnd = this.pos;
          this.pos = startPos;
          this.memo.set(key, { children, endPos: prevEnd, ok: true });
          const newChildren = this.matchElement(rule.body);
          if (newChildren === null || this.pos <= prevEnd) {
            // Could not grow the match — restore the best result.
            this.pos = prevEnd;
            this.memo.set(key, { children, endPos: prevEnd, ok: true });
            break;
          }
          children = newChildren;
        }
      }
    }

    if (children === null) {
      this.pos = startPos;
      this.recordFailure(ruleName);
      return null;
    }

    return this.buildNode(ruleName, children);
  }

  // =========================================================================
  // ELEMENT MATCHING
  // =========================================================================

  private matchElement(
    element: GrammarElement,
  ): Array<ASTNode | Token> | null {
    const savePos = this.pos;

    switch (element.type) {
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
        return children;
      }

      case "optional": {
        const result = this.matchElement(element.element);
        if (result === null) {
          return [];
        }
        return result;
      }

      case "group":
        return this.matchElement(element.element);

      case "token_reference":
        return this.matchTokenReference(element.name);

      case "rule_reference": {
        const node = this.parseRule(element.name);
        if (node !== null) {
          return [node];
        }
        this.pos = savePos;
        return null;
      }

      case "literal": {
        let token = this.current();
        // Skip insignificant newlines before literal matching
        if (!this.newlinesSignificant) {
          while (token.type === "NEWLINE") {
            this.pos++;
            token = this.current();
          }
        }
        if (token.value === element.value) {
          this.pos++;
          return [token];
        }
        this.recordFailure(`"${element.value}"`);
        return null;
      }

      // ---------------------------------------------------------------
      // Extension: Syntactic predicates (lookahead without consuming)
      // ---------------------------------------------------------------

      case "positive_lookahead": {
        // Succeed if inner element matches, but consume no input.
        const result = this.matchElement(element.element);
        this.pos = savePos;
        return result !== null ? [] : null;
      }

      case "negative_lookahead": {
        // Succeed if inner element does NOT match, consume no input.
        const result = this.matchElement(element.element);
        this.pos = savePos;
        return result === null ? [] : null;
      }

      // ---------------------------------------------------------------
      // Extension: One-or-more repetition
      // ---------------------------------------------------------------

      case "one_or_more": {
        // Match one required, then zero or more additional.
        const first = this.matchElement(element.element);
        if (first === null) {
          this.pos = savePos;
          return null;
        }
        const children: Array<ASTNode | Token> = [...first];
        while (true) {
          const saveRep = this.pos;
          const result = this.matchElement(element.element);
          if (result === null) {
            this.pos = saveRep;
            break;
          }
          children.push(...result);
        }
        return children;
      }

      // ---------------------------------------------------------------
      // Extension: Separated repetition
      // ---------------------------------------------------------------

      case "separated_repetition": {
        // Match: element { separator element }
        // Or with atLeastOne=false: [ element { separator element } ]
        const first = this.matchElement(element.element);
        if (first === null) {
          this.pos = savePos;
          if (element.atLeastOne) return null;
          return []; // zero occurrences is valid
        }
        const children: Array<ASTNode | Token> = [...first];
        while (true) {
          const saveSep = this.pos;
          const sep = this.matchElement(element.separator);
          if (sep === null) {
            this.pos = saveSep;
            break;
          }
          const next = this.matchElement(element.element);
          if (next === null) {
            this.pos = saveSep;
            break;
          }
          children.push(...sep, ...next);
        }
        return children;
      }

      default:
        return null;
    }
  }

  // =========================================================================
  // TOKEN REFERENCE MATCHING
  // =========================================================================

  private matchTokenReference(
    expectedType: string,
  ): Array<ASTNode | Token> | null {
    let token = this.current();

    // Skip newlines when matching non-NEWLINE tokens (if insignificant)
    if (!this.newlinesSignificant && expectedType !== "NEWLINE") {
      while (token.type === "NEWLINE") {
        this.pos++;
        token = this.current();
      }
    }

    if (token.type === expectedType) {
      this.pos++;
      return [token];
    }

    this.recordFailure(expectedType);
    return null;
  }

  private buildNode(
    ruleName: string,
    children: ReadonlyArray<ASTNode | Token>,
  ): ASTNode {
    const pos = computeNodePosition(children);
    const sourceInfo = this.preserveSourceInfo
      ? computeNodeSourceInfo(children)
      : null;

    return {
      ruleName,
      children,
      ...(pos ?? {}),
      ...(sourceInfo ?? {}),
    };
  }
}

// ===========================================================================
// AST POSITION COMPUTATION
// ===========================================================================

/**
 * Compute the start and end positions of an AST node from its children.
 *
 * Walks the children tree to find the first and last leaf tokens,
 * then uses their line/column as the node's span. Returns undefined
 * if the children array contains no tokens (e.g., empty repetition).
 */
function computeNodePosition(
  children: ReadonlyArray<ASTNode | Token>,
): { startLine: number; startColumn: number; endLine: number; endColumn: number } | null {
  const first = findFirstToken(children);
  const last = findLastToken(children);
  if (!first || !last) return null;
  return {
    startLine: first.line,
    startColumn: first.column,
    endLine: last.line,
    endColumn: last.column,
  };
}

function computeNodeSourceInfo(
  children: ReadonlyArray<ASTNode | Token>,
): {
  startOffset?: number;
  endOffset?: number;
  firstTokenIndex?: number;
  lastTokenIndex?: number;
  leadingTrivia?: readonly Trivia[];
} | null {
  const first = findFirstToken(children);
  const last = findLastToken(children);
  if (!first || !last) {
    return null;
  }

  const info: {
    startOffset?: number;
    endOffset?: number;
    firstTokenIndex?: number;
    lastTokenIndex?: number;
    leadingTrivia?: readonly Trivia[];
  } = {};

  if (first.startOffset !== undefined) {
    info.startOffset = first.startOffset;
  }
  if (last.endOffset !== undefined) {
    info.endOffset = last.endOffset;
  }
  if (first.tokenIndex !== undefined) {
    info.firstTokenIndex = first.tokenIndex;
  }
  if (last.tokenIndex !== undefined) {
    info.lastTokenIndex = last.tokenIndex;
  }
  if (first.leadingTrivia !== undefined) {
    info.leadingTrivia = first.leadingTrivia;
  }

  return info;
}

function findFirstToken(children: ReadonlyArray<ASTNode | Token>): Token | null {
  for (const child of children) {
    if (isASTNode(child)) {
      const tok = findFirstToken(child.children);
      if (tok) return tok;
    } else {
      return child;
    }
  }
  return null;
}

function findLastToken(children: ReadonlyArray<ASTNode | Token>): Token | null {
  for (let i = children.length - 1; i >= 0; i--) {
    const child = children[i];
    if (isASTNode(child)) {
      const tok = findLastToken(child.children);
      if (tok) return tok;
    } else {
      return child;
    }
  }
  return null;
}

// ===========================================================================
// AST WALKING UTILITIES
// ===========================================================================

/**
 * Visitor interface for walkAST.
 *
 * Both callbacks are optional. Each receives the current node and its parent
 * (null for the root). Returning an ASTNode replaces the visited node;
 * returning void keeps the original.
 */
export interface ASTVisitor {
  enter?(node: ASTNode, parent: ASTNode | null): ASTNode | void;
  leave?(node: ASTNode, parent: ASTNode | null): ASTNode | void;
}

/**
 * Depth-first walk of an AST tree with enter/leave visitor callbacks.
 *
 * Visitor callbacks can return a replacement node or void (keep original).
 * Token children are not visited — only ASTNode children are walked.
 *
 * This is the generic traversal primitive. Language packages use it for
 * cover grammar rewriting, desugaring, and semantic analysis.
 */
export function walkAST(node: ASTNode, visitor: ASTVisitor): ASTNode {
  return walkNode(node, null, visitor);
}

function walkNode(
  node: ASTNode,
  parent: ASTNode | null,
  visitor: ASTVisitor,
): ASTNode {
  // Enter phase — visitor may replace the node.
  let current = node;
  if (visitor.enter) {
    const replacement = visitor.enter(current, parent);
    if (replacement !== undefined) {
      current = replacement;
    }
  }

  // Walk children recursively.
  let childrenChanged = false;
  const newChildren: Array<ASTNode | Token> = [];
  for (const child of current.children) {
    if (isASTNode(child)) {
      const walked = walkNode(child, current, visitor);
      if (walked !== child) childrenChanged = true;
      newChildren.push(walked);
    } else {
      newChildren.push(child);
    }
  }

  // If children changed, create a new node with updated children.
  if (childrenChanged) {
    current = { ...current, children: newChildren };
  }

  // Leave phase — visitor may replace the node.
  if (visitor.leave) {
    const replacement = visitor.leave(current, parent);
    if (replacement !== undefined) {
      current = replacement;
    }
  }

  return current;
}

/**
 * Find all nodes matching a rule name (depth-first order).
 */
export function findNodes(node: ASTNode, ruleName: string): ASTNode[] {
  const results: ASTNode[] = [];
  walkAST(node, {
    enter(n) {
      if (n.ruleName === ruleName) results.push(n);
    },
  });
  return results;
}

/**
 * Collect all tokens in depth-first order, optionally filtered by type.
 */
export function collectTokens(node: ASTNode, type?: string): Token[] {
  const results: Token[] = [];
  function walk(n: ASTNode): void {
    for (const child of n.children) {
      if (isASTNode(child)) {
        walk(child);
      } else {
        if (type === undefined || child.type === type) {
          results.push(child);
        }
      }
    }
  }
  walk(node);
  return results;
}
