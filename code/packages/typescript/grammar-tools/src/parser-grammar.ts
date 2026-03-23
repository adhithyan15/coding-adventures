/**
 * parser-grammar.ts — Parser and validator for .grammar files.
 *
 * A .grammar file describes the syntactic structure of a programming language
 * using EBNF (Extended Backus-Naur Form). Where a .tokens file says "these
 * are the words," a .grammar file says "these are the sentences."
 *
 * EBNF: a brief history
 * ---------------------
 *
 * BNF (Backus-Naur Form) was invented in the late 1950s by John Backus and
 * Peter Naur to describe the syntax of ALGOL 60. It was one of the first
 * formal notations for programming language grammars. EBNF extends BNF with
 * three conveniences:
 *
 *     { x }   — zero or more repetitions of x (replaces recursive rules)
 *     [ x ]   — optional x (shorthand for x | epsilon)
 *     ( x )   — grouping (to clarify precedence in alternations)
 *
 * These extensions don't add any theoretical power — anything expressible in
 * EBNF can be written in plain BNF — but they make grammars dramatically
 * more readable. Compare:
 *
 *     BNF:   statements ::= <empty> | statement statements
 *     EBNF:  statements = { statement } ;
 *
 * The recursive descent parser
 * ----------------------------
 *
 * This module contains a hand-written recursive descent parser for the EBNF
 * notation used in .grammar files. This is the "chicken-and-egg" solution
 * mentioned in the README: we need a parser to read grammar files, so we
 * write one by hand.
 *
 * A recursive descent parser works by having one function per grammar rule.
 * Each function:
 *   1. Looks at the current token (character or word)
 *   2. Decides which alternative to take
 *   3. Calls other parsing functions as needed
 *   4. Returns an AST node
 *
 * For our EBNF parser, the grammar of the grammar (the "meta-grammar") is:
 *
 *     grammar_file  = { rule } ;
 *     rule          = rule_name "=" body ";" ;
 *     body          = sequence { "|" sequence } ;
 *     sequence      = { element } ;
 *     element       = rule_ref | token_ref | literal
 *                   | "{" body "}"
 *                   | "[" body "]"
 *                   | "(" body ")" ;
 *     rule_ref      = lowercase_identifier ;
 *     token_ref     = UPPERCASE_IDENTIFIER ;
 *     literal       = '"' characters '"' ;
 *
 * Each level of this meta-grammar becomes a function in our parser.
 *
 * Why not use regex? Because EBNF has nested structure (parentheses inside
 * braces inside brackets), and regex cannot handle arbitrary nesting. This
 * is exactly the kind of problem that context-free grammars (and recursive
 * descent parsers) were invented to solve.
 */

// ---------------------------------------------------------------------------
// Exceptions
// ---------------------------------------------------------------------------

/**
 * Thrown when a .grammar file cannot be parsed.
 *
 * Properties:
 *   message: Human-readable description of the problem.
 *   lineNumber: 1-based line number where the error occurred.
 */
export class ParserGrammarError extends Error {
  public readonly lineNumber: number;

  constructor(message: string, lineNumber: number) {
    super(`Line ${lineNumber}: ${message}`);
    this.lineNumber = lineNumber;
    this.name = "ParserGrammarError";
  }
}

// ---------------------------------------------------------------------------
// AST node types (the "grammar elements")
// ---------------------------------------------------------------------------
// These interfaces form a tree that represents the parsed body of a grammar
// rule. Together they can express anything that EBNF can express.
//
// We use TypeScript discriminated unions: every node has a `type` field that
// uniquely identifies its kind. This lets us write functions that accept
// "any node" and switch on `node.type`. This is TypeScript's equivalent of
// a tagged union / sum type / algebraic data type.
//
// Why discriminated unions instead of classes? Because:
//   1. They are plain data — easy to serialize, log, and test with deepEqual.
//   2. The `type` field is checked at compile time — typos are caught by tsc.
//   3. Exhaustiveness checking: a switch on `type` warns if you miss a case.
// ---------------------------------------------------------------------------

/**
 * A reference to another grammar rule (lowercase name).
 *
 * In EBNF, `expression` refers to the rule named "expression". Rule
 * references are always lowercase by convention, which distinguishes
 * them from token references (UPPERCASE).
 */
export interface RuleReference {
  readonly type: "rule_reference";
  readonly name: string;
}

/**
 * A reference to a token type (UPPERCASE name).
 *
 * In EBNF, `NUMBER` refers to the token type NUMBER from the .tokens file.
 * Token references are always UPPERCASE by convention.
 */
export interface TokenReference {
  readonly type: "token_reference";
  readonly name: string;
}

/**
 * A literal string match in the grammar, written as "..." in EBNF.
 *
 * This is less common than token references — usually you define tokens
 * in the .tokens file and reference them by name. But sometimes it is
 * convenient to write a literal directly in the grammar.
 */
export interface Literal {
  readonly type: "literal";
  readonly value: string;
}

/**
 * Explicit grouping, written as ( x ) in EBNF.
 *
 * ( PLUS | MINUS ) groups the alternation so it can be used as a
 * single element in a sequence: term { ( PLUS | MINUS ) term }.
 */
export interface Group {
  readonly type: "group";
  readonly element: GrammarElement;
}

/**
 * Optional element, written as [ x ] in EBNF.
 *
 * [ ELSE block ] means "optionally an ELSE followed by a block."
 * Equivalent to x | epsilon in BNF.
 */
export interface Optional {
  readonly type: "optional";
  readonly element: GrammarElement;
}

/**
 * Zero-or-more repetition, written as { x } in EBNF.
 *
 * { statement } means "zero or more statements." This replaces
 * the recursive rules that plain BNF requires.
 */
export interface Repetition {
  readonly type: "repetition";
  readonly element: GrammarElement;
}

/**
 * A choice between alternatives, written with | in EBNF.
 *
 * A | B | C means "either A, or B, or C." The parser tries each
 * alternative in order (for predictive parsers) or uses lookahead to
 * decide.
 */
export interface Alternation {
  readonly type: "alternation";
  readonly choices: readonly GrammarElement[];
}

/**
 * A sequence of elements that must appear in order.
 *
 * In EBNF, juxtaposition means sequencing: A B C means "A followed
 * by B followed by C." This is the most fundamental combinator.
 */
export interface Sequence {
  readonly type: "sequence";
  readonly elements: readonly GrammarElement[];
}

// The union of all grammar element types. This is what recursive functions
// over the grammar tree accept and pattern-match on.
export type GrammarElement =
  | RuleReference
  | TokenReference
  | Literal
  | Group
  | Optional
  | Repetition
  | Alternation
  | Sequence;

// ---------------------------------------------------------------------------
// Data model for the complete grammar
// ---------------------------------------------------------------------------

/**
 * A single rule from a .grammar file.
 *
 * Properties:
 *   name: The rule name (lowercase identifier).
 *   body: The parsed EBNF body as a tree of GrammarElement nodes.
 *   lineNumber: The 1-based line number where this rule appeared.
 */
export interface GrammarRule {
  readonly name: string;
  readonly body: GrammarElement;
  readonly lineNumber: number;
}

/**
 * The complete contents of a parsed .grammar file.
 *
 * Properties:
 *   rules: Ordered list of grammar rules. The first rule is the
 *       entry point (start symbol).
 */
export interface ParserGrammar {
  readonly rules: readonly GrammarRule[];
}

// ---------------------------------------------------------------------------
// AST traversal helpers
// ---------------------------------------------------------------------------

/**
 * Return all defined rule names.
 */
export function ruleNames(grammar: ParserGrammar): Set<string> {
  return new Set(grammar.rules.map((r) => r.name));
}

/**
 * Return all UPPERCASE token names referenced anywhere in the grammar.
 *
 * These should correspond to token names in the .tokens file.
 */
export function grammarTokenReferences(grammar: ParserGrammar): Set<string> {
  const refs = new Set<string>();
  for (const rule of grammar.rules) {
    collectTokenRefs(rule.body, refs);
  }
  return refs;
}

/**
 * Return all lowercase rule names referenced anywhere in the grammar.
 *
 * These should correspond to other rule names in this grammar.
 */
export function grammarRuleReferences(grammar: ParserGrammar): Set<string> {
  const refs = new Set<string>();
  for (const rule of grammar.rules) {
    collectRuleRefs(rule.body, refs);
  }
  return refs;
}

// ---------------------------------------------------------------------------
// Internal AST walkers
// ---------------------------------------------------------------------------
// These functions walk the grammar element tree to collect references.
// They use the discriminated union's `type` field to decide which branch
// to take at each node — the TypeScript equivalent of pattern matching.
// ---------------------------------------------------------------------------

function collectTokenRefs(node: GrammarElement, refs: Set<string>): void {
  switch (node.type) {
    case "token_reference":
      refs.add(node.name);
      break;
    case "rule_reference":
    case "literal":
      // Leaf nodes that don't contain token references.
      break;
    case "sequence":
      for (const e of node.elements) {
        collectTokenRefs(e, refs);
      }
      break;
    case "alternation":
      for (const c of node.choices) {
        collectTokenRefs(c, refs);
      }
      break;
    case "repetition":
    case "optional":
    case "group":
      collectTokenRefs(node.element, refs);
      break;
  }
}

function collectRuleRefs(node: GrammarElement, refs: Set<string>): void {
  switch (node.type) {
    case "rule_reference":
      refs.add(node.name);
      break;
    case "token_reference":
    case "literal":
      // Leaf nodes that don't contain rule references.
      break;
    case "sequence":
      for (const e of node.elements) {
        collectRuleRefs(e, refs);
      }
      break;
    case "alternation":
      for (const c of node.choices) {
        collectRuleRefs(c, refs);
      }
      break;
    case "repetition":
    case "optional":
    case "group":
      collectRuleRefs(node.element, refs);
      break;
  }
}

// ---------------------------------------------------------------------------
// Tokenizer for .grammar files
// ---------------------------------------------------------------------------
// Before we can parse the EBNF, we need to break the raw text into tokens.
// This is a simple hand-written tokenizer — much simpler than the lexers we
// are trying to generate, because the grammar notation uses only a few token
// types.
// ---------------------------------------------------------------------------

/**
 * Internal token type for the grammar file tokenizer.
 *
 * Token types:
 *   IDENT   — an identifier (rule name or token reference)
 *   STRING  — a quoted literal "..."
 *   EQUALS  — the = sign separating rule name from body
 *   SEMI    — the ; terminating a rule
 *   PIPE    — the | alternation operator
 *   LBRACE / RBRACE — { }
 *   LBRACKET / RBRACKET — [ ]
 *   LPAREN / RPAREN — ( )
 *   EOF     — end of input
 */
interface Token {
  readonly kind: string;
  readonly value: string;
  readonly line: number;
}

function tokenizeGrammar(source: string): Token[] {
  const tokens: Token[] = [];
  const lines = source.split("\n");

  for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
    const lineNumber = lineIdx + 1;
    const line = lines[lineIdx].replace(/\s+$/, "");
    const stripped = line.trim();

    // Skip blanks and comments.
    if (stripped === "" || stripped.startsWith("#")) {
      continue;
    }

    let i = 0;
    while (i < line.length) {
      const ch = line[i];

      // Skip whitespace.
      if (ch === " " || ch === "\t") {
        i++;
        continue;
      }

      // Skip inline comments.
      if (ch === "#") {
        break; // Rest of line is a comment.
      }

      // Single-character tokens.
      if (ch === "=") {
        tokens.push({ kind: "EQUALS", value: "=", line: lineNumber });
        i++;
      } else if (ch === ";") {
        tokens.push({ kind: "SEMI", value: ";", line: lineNumber });
        i++;
      } else if (ch === "|") {
        tokens.push({ kind: "PIPE", value: "|", line: lineNumber });
        i++;
      } else if (ch === "{") {
        tokens.push({ kind: "LBRACE", value: "{", line: lineNumber });
        i++;
      } else if (ch === "}") {
        tokens.push({ kind: "RBRACE", value: "}", line: lineNumber });
        i++;
      } else if (ch === "[") {
        tokens.push({ kind: "LBRACKET", value: "[", line: lineNumber });
        i++;
      } else if (ch === "]") {
        tokens.push({ kind: "RBRACKET", value: "]", line: lineNumber });
        i++;
      } else if (ch === "(") {
        tokens.push({ kind: "LPAREN", value: "(", line: lineNumber });
        i++;
      } else if (ch === ")") {
        tokens.push({ kind: "RPAREN", value: ")", line: lineNumber });
        i++;
      }
      // Quoted string literal.
      else if (ch === '"') {
        let j = i + 1;
        while (j < line.length && line[j] !== '"') {
          if (line[j] === "\\") {
            j++; // Skip escaped character.
          }
          j++;
        }
        if (j >= line.length) {
          throw new ParserGrammarError(
            "Unterminated string literal",
            lineNumber
          );
        }
        // Store the string content without quotes, so we can distinguish
        // literals from identifiers later.
        tokens.push({ kind: "STRING", value: line.slice(i + 1, j), line: lineNumber });
        i = j + 1;
      }
      // Identifier (rule name or token reference).
      else if (/[a-zA-Z_]/.test(ch)) {
        let j = i;
        while (j < line.length && /[a-zA-Z0-9_]/.test(line[j])) {
          j++;
        }
        tokens.push({ kind: "IDENT", value: line.slice(i, j), line: lineNumber });
        i = j;
      } else {
        throw new ParserGrammarError(
          `Unexpected character: '${ch}'`,
          lineNumber
        );
      }
    }
  }

  tokens.push({ kind: "EOF", value: "", line: lines.length });
  return tokens;
}

// ---------------------------------------------------------------------------
// Recursive descent parser for EBNF
// ---------------------------------------------------------------------------
// The parser consumes the token list produced by tokenizeGrammar and
// builds a tree of GrammarElement nodes. Each function corresponds to one
// level of the meta-grammar:
//
//   parseGrammarFile  ->  { rule }
//   parseRule         ->  name "=" body ";"
//   parseBody         ->  sequence { "|" sequence }
//   parseSequence     ->  { element }
//   parseElement      ->  ident | string | "{" body "}" | "[" body "]" | "(" body ")"
// ---------------------------------------------------------------------------

class Parser {
  private tokens: Token[];
  private pos: number;

  constructor(tokens: Token[]) {
    this.tokens = tokens;
    this.pos = 0;
  }

  /** Look at the current token without consuming it. */
  private peek(): Token {
    return this.tokens[this.pos];
  }

  /** Consume and return the current token. */
  private advance(): Token {
    const tok = this.tokens[this.pos];
    this.pos++;
    return tok;
  }

  /** Consume a token of the expected kind, or throw an error. */
  private expect(kind: string): Token {
    const tok = this.advance();
    if (tok.kind !== kind) {
      throw new ParserGrammarError(
        `Expected ${kind}, got ${tok.kind} ('${tok.value}')`,
        tok.line
      );
    }
    return tok;
  }

  // --- Top level: grammar file = { rule } ---

  /** Parse all rules in the grammar file. */
  parse(): GrammarRule[] {
    const rules: GrammarRule[] = [];
    while (this.peek().kind !== "EOF") {
      rules.push(this.parseRule());
    }
    return rules;
  }

  // --- rule = name "=" body ";" ---

  /** Parse a single grammar rule. */
  private parseRule(): GrammarRule {
    const nameTok = this.expect("IDENT");
    this.expect("EQUALS");
    const body = this.parseBody();
    this.expect("SEMI");
    return { name: nameTok.value, body, lineNumber: nameTok.line };
  }

  // --- body = sequence { "|" sequence } ---

  /**
   * Parse alternation: one or more sequences separated by '|'.
   *
   * If there is only one sequence (no '|'), we return it directly
   * rather than wrapping it in an Alternation node. This keeps the
   * AST clean — a rule like `factor = NUMBER ;` produces a simple
   * TokenReference, not an Alternation with one choice containing a
   * Sequence with one element.
   */
  private parseBody(): GrammarElement {
    const first = this.parseSequence();
    const alternatives: GrammarElement[] = [first];

    while (this.peek().kind === "PIPE") {
      this.advance(); // consume '|'
      alternatives.push(this.parseSequence());
    }

    if (alternatives.length === 1) {
      return alternatives[0];
    }
    return { type: "alternation", choices: alternatives };
  }

  // --- sequence = { element } ---

  /**
   * Parse a sequence of elements.
   *
   * A sequence ends when we hit something that cannot start an element:
   * '|', ';', '}', ']', ')' or EOF. If the sequence has only one
   * element, we return it directly (no Sequence wrapper).
   */
  private parseSequence(): GrammarElement {
    const elements: GrammarElement[] = [];
    const stopKinds = new Set([
      "PIPE",
      "SEMI",
      "RBRACE",
      "RBRACKET",
      "RPAREN",
      "EOF",
    ]);

    while (!stopKinds.has(this.peek().kind)) {
      elements.push(this.parseElement());
    }

    if (elements.length === 0) {
      throw new ParserGrammarError(
        "Expected at least one element in sequence",
        this.peek().line
      );
    }
    if (elements.length === 1) {
      return elements[0];
    }
    return { type: "sequence", elements };
  }

  // --- element = ident | string | "{" body "}" | "[" body "]" | "(" body ")" ---

  /**
   * Parse a single grammar element.
   *
   * This is where the recursive descent happens: braces, brackets,
   * and parentheses cause us to recurse back into parseBody.
   */
  private parseElement(): GrammarElement {
    const tok = this.peek();

    if (tok.kind === "IDENT") {
      this.advance();
      // UPPERCASE = token reference, lowercase = rule reference.
      // We check if the first character is an uppercase letter to distinguish.
      const isToken =
        tok.value === tok.value.toUpperCase() && /[A-Z]/.test(tok.value[0]);
      if (isToken) {
        return { type: "token_reference", name: tok.value };
      }
      return { type: "rule_reference", name: tok.value };
    }

    if (tok.kind === "STRING") {
      this.advance();
      return { type: "literal", value: tok.value };
    }

    if (tok.kind === "LBRACE") {
      this.advance();
      const body = this.parseBody();
      this.expect("RBRACE");
      return { type: "repetition", element: body };
    }

    if (tok.kind === "LBRACKET") {
      this.advance();
      const body = this.parseBody();
      this.expect("RBRACKET");
      return { type: "optional", element: body };
    }

    if (tok.kind === "LPAREN") {
      this.advance();
      const body = this.parseBody();
      this.expect("RPAREN");
      return { type: "group", element: body };
    }

    throw new ParserGrammarError(
      `Unexpected token: ${tok.kind} ('${tok.value}')`,
      tok.line
    );
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Parse the text of a .grammar file into a ParserGrammar.
 *
 * This function tokenizes the source, then runs a recursive descent
 * parser over the token stream to produce an AST of grammar elements.
 *
 * @param source - The full text content of a .grammar file.
 * @returns A ParserGrammar containing all parsed rules.
 * @throws ParserGrammarError if the source cannot be parsed.
 */
export function parseParserGrammar(source: string): ParserGrammar {
  const tokens = tokenizeGrammar(source);
  const parser = new Parser(tokens);
  const rules = parser.parse();
  return { rules };
}

// ---------------------------------------------------------------------------
// Validator
// ---------------------------------------------------------------------------

/**
 * Check a parsed ParserGrammar for common problems.
 *
 * Validation checks:
 * - **Undefined rule references**: A lowercase name is used in a rule
 *   body but never defined as a rule. This means the parser would have
 *   no idea how to parse that construct.
 * - **Undefined token references**: An UPPERCASE name is used but does
 *   not appear in the provided tokenNames set. (Only checked if
 *   tokenNames is provided.)
 * - **Duplicate rule names**: Two rules with the same name. The second
 *   would shadow the first.
 * - **Non-lowercase rule names**: By convention, rule names are lowercase
 *   to distinguish them from token names.
 * - **Unreachable rules**: A rule that is defined but never referenced
 *   by any other rule. The first rule (start symbol) is exempt.
 *
 * @param grammar - A parsed ParserGrammar to validate.
 * @param tokenNamesSet - Optional set of valid token names from a .tokens file.
 *     If provided, UPPERCASE references are checked against it.
 * @returns A list of warning/error strings. An empty list means no issues.
 */
export function validateParserGrammar(
  grammar: ParserGrammar,
  tokenNamesSet?: Set<string> | null
): string[] {
  const issues: string[] = [];
  const defined = ruleNames(grammar);
  const referencedRules = grammarRuleReferences(grammar);
  const referencedTokens = grammarTokenReferences(grammar);

  // --- Duplicate rule names ---
  const seen = new Map<string, number>();
  for (const rule of grammar.rules) {
    const firstLine = seen.get(rule.name);
    if (firstLine !== undefined) {
      issues.push(
        `Line ${rule.lineNumber}: Duplicate rule name ` +
          `'${rule.name}' (first defined on line ${firstLine})`
      );
    } else {
      seen.set(rule.name, rule.lineNumber);
    }
  }

  // --- Non-lowercase rule names ---
  for (const rule of grammar.rules) {
    if (rule.name !== rule.name.toLowerCase()) {
      issues.push(
        `Line ${rule.lineNumber}: Rule name '${rule.name}' ` +
          `should be lowercase`
      );
    }
  }

  // --- Undefined rule references ---
  for (const ref of [...referencedRules].sort()) {
    if (!defined.has(ref)) {
      issues.push(`Undefined rule reference: '${ref}'`);
    }
  }

  // --- Undefined token references ---
  if (tokenNamesSet != null) {
    // Synthetic tokens are always valid — the lexer produces these
    // implicitly without needing a .tokens definition:
    //   NEWLINE — emitted at bare '\n' when skip pattern excludes newlines
    //   INDENT/DEDENT — emitted in indentation mode
    //   EOF — always emitted at end of input
    const syntheticTokens = new Set(["NEWLINE", "INDENT", "DEDENT", "EOF"]);
    for (const ref of [...referencedTokens].sort()) {
      if (!tokenNamesSet.has(ref) && !syntheticTokens.has(ref)) {
        issues.push(`Undefined token reference: '${ref}'`);
      }
    }
  }

  // --- Unreachable rules ---
  if (grammar.rules.length > 0) {
    const startRule = grammar.rules[0].name;
    for (const rule of grammar.rules) {
      if (rule.name !== startRule && !referencedRules.has(rule.name)) {
        issues.push(
          `Line ${rule.lineNumber}: Rule '${rule.name}' is ` +
            `defined but never referenced (unreachable)`
        );
      }
    }
  }

  return issues;
}
