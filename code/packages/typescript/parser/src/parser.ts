/**
 * Parser — Recursive Descent Parser for the Computing Stack.
 *
 * =============================================================================
 * WHAT IS A PARSER?
 * =============================================================================
 *
 * A parser is the second stage of a compiler or interpreter pipeline. It takes
 * a flat list of tokens (produced by the lexer) and builds a tree structure
 * called an Abstract Syntax Tree (AST). The AST captures the *meaning* of the
 * code by encoding the relationships between tokens.
 *
 * Think of it like diagramming a sentence in English class:
 *
 *     "The cat sat on the mat"
 *
 *     Sentence
 *     +-- Subject: "The cat"
 *     +-- Predicate: "sat on the mat"
 *
 * Similarly, for code:
 *
 *     "x = 1 + 2 * 3"
 *
 *     Assignment
 *     +-- target: Name("x")
 *     +-- value: BinaryOp("+")
 *         +-- left: NumberLiteral(1)
 *         +-- right: BinaryOp("*")
 *             +-- left: NumberLiteral(2)
 *             +-- right: NumberLiteral(3)
 *
 * Notice how `2 * 3` is deeper in the tree than `1 +`. This means multiplication
 * gets evaluated FIRST. The tree structure itself encodes operator precedence —
 * no parentheses needed in the tree!
 *
 * =============================================================================
 * RECURSIVE DESCENT PARSING
 * =============================================================================
 *
 * This parser uses "recursive descent" — the simplest and most intuitive parsing
 * technique. The idea is beautifully direct:
 *
 *     Each grammar rule becomes a TypeScript method.
 *
 * The grammar:
 *
 *     program     = statement*
 *     statement   = assignment | expression_stmt
 *     assignment  = NAME EQUALS expression NEWLINE
 *     expression  = term ((PLUS | MINUS) term)*
 *     term        = factor ((STAR | SLASH) factor)*
 *     factor      = NUMBER | STRING | NAME | LPAREN expression RPAREN
 *
 * Maps to methods:
 *
 *     parseProgram()     -> calls parseStatement() in a loop
 *     parseStatement()   -> tries parseAssignment(), falls back to expression
 *     parseExpression()  -> calls parseTerm(), loops on + and -
 *     parseTerm()        -> calls parseFactor(), loops on * and /
 *     parseFactor()      -> handles numbers, strings, names, parenthesized exprs
 *
 * The "recursive" part comes from the fact that these methods call each other.
 * For example, parseFactor() can call parseExpression() when it encounters
 * a parenthesized expression — and parseExpression() will call parseTerm()
 * which calls parseFactor() again. It's recursive!
 *
 * =============================================================================
 * OPERATOR PRECEDENCE
 * =============================================================================
 *
 * Operator precedence is encoded by the *depth* of the grammar rules:
 *
 *     expression  ->  handles + and -     (LOWEST precedence — evaluated LAST)
 *     term        ->  handles * and /     (HIGHER precedence — evaluated FIRST)
 *     factor      ->  handles atoms       (HIGHEST precedence — numbers, names)
 *
 * When we parse `1 + 2 * 3`:
 *
 *     1. parseExpression() calls parseTerm()
 *     2. parseTerm() calls parseFactor() -> gets NumberLiteral(1)
 *     3. parseTerm() sees no * or /, returns NumberLiteral(1)
 *     4. parseExpression() sees +, advances
 *     5. parseExpression() calls parseTerm() again
 *     6. parseTerm() calls parseFactor() -> gets NumberLiteral(2)
 *     7. parseTerm() sees *, advances
 *     8. parseTerm() calls parseFactor() -> gets NumberLiteral(3)
 *     9. parseTerm() builds BinaryOp(NumberLiteral(2), "*", NumberLiteral(3))
 *     10. parseExpression() builds BinaryOp(NumberLiteral(1), "+", BinaryOp(...))
 *
 * The multiplication was handled at a deeper level, so it becomes a subtree.
 */

import type { Token } from "@coding-adventures/lexer";

// =============================================================================
// AST NODE TYPES
// =============================================================================
//
// These are the building blocks of our Abstract Syntax Tree. Each interface
// represents a different kind of syntactic construct in our language.
//
// We use TypeScript interfaces with a discriminant `kind` field for these
// because AST nodes are simple containers of data — they hold their children
// and nothing else. The `kind` field lets TypeScript narrow the type in
// switch/if statements, giving us exhaustive pattern matching.
//
// Think of AST nodes like LEGO bricks. Each type has a specific shape
// (its fields), and you snap them together to build the full tree.
// =============================================================================

/**
 * A numeric literal like 42 or 7.
 *
 * This is a "leaf" node in the AST — it has no children, just a value.
 * It's the simplest possible expression: a number that evaluates to itself.
 *
 * Example:
 *     The source code `42` becomes { kind: "NumberLiteral", value: 42 }.
 *
 * Properties:
 *     value: The integer value of the literal. We store it as a number
 *            rather than keeping the string representation because the
 *            parser's job is to convert syntax into structured meaning.
 */
export interface NumberLiteral {
  readonly kind: "NumberLiteral";
  readonly value: number;
}

/**
 * A string literal like "hello" or "world".
 *
 * Similar to NumberLiteral, this is a leaf node. The lexer has already
 * stripped the surrounding quotes, so we just store the string content.
 *
 * Example:
 *     The source code `"hello"` becomes { kind: "StringLiteral", value: "hello" }.
 *
 * Properties:
 *     value: The string content, without surrounding quotes.
 */
export interface StringLiteral {
  readonly kind: "StringLiteral";
  readonly value: string;
}

/**
 * A variable name (identifier) like x, total, or my_var.
 *
 * Names are references to values stored elsewhere. When we evaluate
 * `x + 1`, the Name node means "look up the current value of x."
 *
 * In a compiler or interpreter, Name nodes are resolved by looking up
 * the name in a symbol table or environment.
 *
 * Example:
 *     The source code `x` becomes { kind: "Name", name: "x" }.
 *
 * Properties:
 *     name: The identifier string. We call it 'name' rather than 'value'
 *           to emphasize that this is a reference, not a literal value.
 */
export interface Name {
  readonly kind: "Name";
  readonly name: string;
}

/**
 * A binary operation like `1 + 2` or `x * y`.
 *
 * "Binary" means two operands (left and right), not binary numbers.
 * This is an "interior" node in the AST — it has children (the operands).
 *
 * The tree structure naturally encodes operator precedence:
 *     `1 + 2 * 3` becomes:
 *         BinaryOp {
 *             left: NumberLiteral(1),
 *             op: "+",
 *             right: BinaryOp {
 *                 left: NumberLiteral(2),
 *                 op: "*",
 *                 right: NumberLiteral(3)
 *             }
 *         }
 *
 * The multiplication is deeper in the tree, so it gets evaluated first.
 *
 * Properties:
 *     left:  The left operand (any expression).
 *     op:    The operator as a string: "+", "-", "*", or "/".
 *     right: The right operand (any expression).
 */
export interface BinaryOp {
  readonly kind: "BinaryOp";
  readonly left: Expression;
  readonly op: string;
  readonly right: Expression;
}

/**
 * A variable assignment like `x = 1 + 2`.
 *
 * Assignments bind a name to a value. The target is always a Name node
 * (the variable being assigned to), and the value is any expression.
 *
 * Example:
 *     `x = 1 + 2` becomes:
 *         Assignment {
 *             target: Name("x"),
 *             value: BinaryOp(NumberLiteral(1), "+", NumberLiteral(2))
 *         }
 *
 * Properties:
 *     target: The Name being assigned to.
 *     value:  The expression whose result will be stored in the variable.
 */
export interface Assignment {
  readonly kind: "Assignment";
  readonly target: Name;
  readonly value: Expression;
}

/**
 * The root node of the AST — represents an entire program.
 *
 * A program is simply a sequence of statements executed in order.
 * This node exists so that the parser always returns a single root node,
 * even when the source code contains multiple statements.
 *
 * Example:
 *     The source code:
 *         x = 1
 *         y = 2
 *
 *     Becomes:
 *         Program { statements: [
 *             Assignment(Name("x"), NumberLiteral(1)),
 *             Assignment(Name("y"), NumberLiteral(2)),
 *         ]}
 *
 * Properties:
 *     statements: A list of all top-level statements in the program.
 */
export interface Program {
  readonly kind: "Program";
  readonly statements: readonly Statement[];
}

// =============================================================================
// TYPE ALIASES
// =============================================================================
//
// TypeScript's union types let us define which node types can appear in which
// positions. This serves as documentation AND enables type-checker enforcement.
//
// Expression: anything that produces a value when evaluated.
// Statement: anything that performs an action (assignment, expression statement).
// =============================================================================

/**
 * An Expression is any AST node that evaluates to a value.
 *
 * Numbers, strings, and names evaluate to themselves (or their bound value).
 * BinaryOp evaluates by computing its operation on the left and right values.
 */
export type Expression = NumberLiteral | StringLiteral | Name | BinaryOp;

/**
 * A Statement is a top-level action in the program.
 *
 * An Assignment binds a value to a name. An expression used as a statement
 * is simply evaluated for its side effects (or, in a REPL, its value is printed).
 */
export type Statement = Assignment | Expression;

// =============================================================================
// PARSE ERROR
// =============================================================================

/**
 * Raised when the parser encounters an unexpected token.
 *
 * Parse errors include the problematic token so error messages can report
 * the exact line and column where things went wrong. This is essential for
 * a good developer experience — nobody likes "syntax error" with no location!
 *
 * Properties:
 *     message: A human-readable description of what went wrong (inherited).
 *     token:   The token where the error was detected.
 */
export class ParseError extends Error {
  public readonly token: Token;

  constructor(message: string, token: Token) {
    super(`${message} at line ${token.line}, column ${token.column}`);
    this.name = "ParseError";
    this.token = token;
  }
}

// =============================================================================
// THE PARSER
// =============================================================================

/**
 * A recursive descent parser that builds an AST from a token stream.
 *
 * ==========================================================================
 * HOW IT WORKS
 * ==========================================================================
 *
 * The parser maintains a cursor (this.pos) into the token list. It reads
 * tokens one at a time, advancing the cursor as it consumes them. Each
 * grammar rule is a method that:
 *
 *     1. Looks at the current token(s) to decide what to parse
 *     2. Consumes the tokens it needs
 *     3. Returns an AST node
 *
 * The "recursive descent" comes from the way these methods call each other
 * in a pattern that mirrors the grammar rules. Higher-level rules (like
 * parseExpression) call lower-level rules (like parseTerm), which call
 * even lower-level rules (like parseFactor).
 *
 * ==========================================================================
 * USAGE
 * ==========================================================================
 *
 *     const tokens = [
 *       { type: "NUMBER", value: "42", line: 1, column: 1 },
 *       { type: "EOF", value: "", line: 1, column: 3 },
 *     ];
 *     const parser = new Parser(tokens);
 *     const ast = parser.parse();
 *     // ast is now a Program node containing the AST
 *
 * ==========================================================================
 * GRAMMAR
 * ==========================================================================
 *
 *     program         = statement*
 *     statement       = assignment | expression_stmt
 *     assignment      = NAME EQUALS expression NEWLINE
 *     expression_stmt = expression NEWLINE
 *     expression      = term ((PLUS | MINUS) term)*
 *     term            = factor ((STAR | SLASH) factor)*
 *     factor          = NUMBER | STRING | NAME | LPAREN expression RPAREN
 *
 * Each grammar rule maps directly to a parse* method below.
 */
export class Parser {
  /**
   * The complete list of tokens from the lexer.
   */
  private readonly tokens: readonly Token[];

  /**
   * The current position in the token list (our "cursor").
   */
  private pos: number;

  /**
   * Initialize the parser with a list of tokens.
   *
   * The token list should end with an EOF token — this is the standard
   * convention from the lexer and serves as a sentinel value so we
   * never accidentally read past the end of the input.
   *
   * @param tokens - A list of Token objects, typically ending with EOF.
   */
  constructor(tokens: readonly Token[]) {
    this.tokens = tokens;
    this.pos = 0;
  }

  // =========================================================================
  // HELPER METHODS
  // =========================================================================
  //
  // These utility methods abstract away the mechanics of reading tokens.
  // They keep the grammar methods clean and focused on structure rather
  // than bookkeeping.
  // =========================================================================

  /**
   * Look at the current token WITHOUT consuming it.
   *
   * This is like glancing at the next card in a deck without picking it up.
   * We use peek() when we need to decide what to do based on the current
   * token, but haven't committed to consuming it yet.
   *
   * @returns The token at the current position. If we've reached the end
   *          of the token list, returns the last token (which should be EOF).
   */
  private peek(): Token {
    if (this.pos < this.tokens.length) {
      return this.tokens[this.pos];
    }
    // Safety: return the last token (should be EOF) if we're past the end.
    // This prevents index-out-of-bounds errors in edge cases.
    return this.tokens[this.tokens.length - 1];
  }

  /**
   * Consume the current token and move to the next one.
   *
   * This is like picking up a card from the deck. Once consumed, we
   * can't go back (this is a simple parser with no backtracking).
   *
   * @returns The token that was consumed (the one at the old position).
   */
  private advance(): Token {
    const token = this.peek();
    this.pos++;
    return token;
  }

  /**
   * Consume the current token, but ONLY if it has the expected type.
   *
   * This is used when the grammar requires a specific token. For example,
   * after seeing `(`, we know a `)` must eventually appear. If it doesn't,
   * that's a syntax error.
   *
   * Think of it like a bouncer at a door: "I'm expecting a RPAREN.
   * If you're not an RPAREN, you can't come in."
   *
   * @param tokenType - The token type string we expect to see.
   * @returns The consumed token (if it matched).
   * @throws ParseError if the current token doesn't match the expected type.
   */
  private expect(tokenType: string): Token {
    const token = this.peek();
    if (token.type !== tokenType) {
      throw new ParseError(
        `Expected ${tokenType}, got ${token.type} (${JSON.stringify(token.value)})`,
        token,
      );
    }
    return this.advance();
  }

  /**
   * Consume the current token if it matches ANY of the given types.
   *
   * This is a "soft" version of expect(). Instead of raising an error
   * when there's no match, it simply returns null. This is perfect for
   * optional grammar elements — like the operator in `expression`:
   *
   *     expression = term ((PLUS | MINUS) term)*
   *
   * The `(PLUS | MINUS)` part is optional (the `*` means zero or more).
   * We use match() to check: "Is there a + or - here? If so, consume it.
   * If not, that's fine — we're done with this expression."
   *
   * @param tokenTypes - One or more token type strings to try matching.
   * @returns The consumed token if it matched, or null if it didn't.
   */
  private match(...tokenTypes: string[]): Token | null {
    const token = this.peek();
    if (tokenTypes.includes(token.type)) {
      return this.advance();
    }
    return null;
  }

  /**
   * Check if we've reached the end of the token stream.
   *
   * @returns True if the current token is EOF, false otherwise.
   */
  private atEnd(): boolean {
    return this.peek().type === "EOF";
  }

  /**
   * Skip over any NEWLINE tokens at the current position.
   *
   * Newlines are significant in our grammar (they terminate statements),
   * but between statements we want to skip any extra blank lines.
   */
  private skipNewlines(): void {
    while (this.peek().type === "NEWLINE") {
      this.advance();
    }
  }

  // =========================================================================
  // PUBLIC API
  // =========================================================================

  /**
   * Parse the token stream and return a complete AST.
   *
   * This is the main entry point. It delegates to parseProgram(),
   * which handles the top-level grammar rule.
   *
   * @returns A Program node containing all parsed statements.
   * @throws ParseError if the token stream contains syntax errors.
   */
  parse(): Program {
    return this.parseProgram();
  }

  // =========================================================================
  // GRAMMAR METHODS
  // =========================================================================
  //
  // Each method below corresponds to one rule in the grammar. The method
  // names follow the pattern parse<RuleName>().
  //
  // These methods form the heart of the recursive descent parser. Reading
  // them alongside the grammar rules makes the mapping clear:
  //
  //   Grammar Rule                        -> Method
  //   program = statement*                -> parseProgram()
  //   statement = assignment | expr_stmt  -> parseStatement()
  //   assignment = NAME EQUALS expr NL    -> parseAssignment()
  //   expression = term ((+|-) term)*     -> parseExpression()
  //   term = factor ((*|/) factor)*       -> parseTerm()
  //   factor = NUMBER | STRING | NAME |   -> parseFactor()
  //            LPAREN expr RPAREN
  // =========================================================================

  /**
   * Parse the top-level program: a sequence of statements.
   *
   * Grammar: program = statement*
   *
   * A program is zero or more statements. We keep parsing statements
   * until we hit EOF (end of file). This is the root of our recursive
   * descent — everything starts here.
   *
   * @returns A Program node containing all parsed statements.
   */
  private parseProgram(): Program {
    const statements: Statement[] = [];

    // Skip any leading newlines (blank lines at the top of the file).
    this.skipNewlines();

    // Parse statements until we reach the end of the token stream.
    while (!this.atEnd()) {
      const statement = this.parseStatement();
      statements.push(statement);

      // Skip any newlines between statements (blank lines).
      this.skipNewlines();
    }

    return { kind: "Program", statements };
  }

  /**
   * Parse a single statement: either an assignment or an expression.
   *
   * Grammar: statement = assignment | expression_stmt
   *
   * The tricky part here is distinguishing assignments from expressions.
   * Both can start with a NAME token:
   *     - `x = 1 + 2`  <- assignment (NAME followed by EQUALS)
   *     - `x + 1`      <- expression starting with a name
   *
   * We use LOOKAHEAD: peek at the current token and the next one.
   * If we see NAME followed by EQUALS, it's an assignment.
   * Otherwise, it's an expression statement.
   *
   * @returns Either an Assignment node or an Expression node.
   */
  private parseStatement(): Statement {
    // Lookahead: check if this is an assignment (NAME EQUALS ...).
    // We need to look TWO tokens ahead — the current one (NAME) and
    // the next one (EQUALS). This is called "LL(2)" lookahead.
    if (
      this.peek().type === "NAME" &&
      this.pos + 1 < this.tokens.length &&
      this.tokens[this.pos + 1].type === "EQUALS"
    ) {
      return this.parseAssignment();
    }

    return this.parseExpressionStmt();
  }

  /**
   * Parse an assignment statement: NAME EQUALS expression NEWLINE.
   *
   * Grammar: assignment = NAME EQUALS expression NEWLINE
   *
   * An assignment binds the result of an expression to a variable name.
   * For example, `x = 1 + 2` means "evaluate 1 + 2 and store the
   * result (3) in a variable called x."
   *
   * @returns An Assignment node with the target name and value expression.
   */
  private parseAssignment(): Assignment {
    // Consume the variable name (the left side of the =).
    const nameToken = this.expect("NAME");
    const target: Name = { kind: "Name", name: nameToken.value };

    // Consume the = sign.
    this.expect("EQUALS");

    // Parse the value expression (the right side of the =).
    const value = this.parseExpression();

    // Consume the newline that terminates the statement.
    // If we're at EOF, that's also acceptable (last line without newline).
    if (!this.atEnd()) {
      this.expect("NEWLINE");
    }

    return { kind: "Assignment", target, value };
  }

  /**
   * Parse an expression used as a statement.
   *
   * Grammar: expression_stmt = expression NEWLINE
   *
   * Sometimes an expression appears on its own line, not as part of
   * an assignment. For example, in a REPL:
   *
   *     >>> 1 + 2
   *     3
   *
   * The `1 + 2` is an expression statement — it's evaluated and the
   * result is displayed.
   *
   * @returns An Expression node (the expression itself IS the statement).
   */
  private parseExpressionStmt(): Expression {
    const expression = this.parseExpression();

    // Consume the trailing newline (or accept EOF).
    if (!this.atEnd()) {
      this.expect("NEWLINE");
    }

    return expression;
  }

  /**
   * Parse an expression: addition and subtraction.
   *
   * Grammar: expression = term ((PLUS | MINUS) term)*
   *
   * This handles the LOWEST precedence operators: + and -. It calls
   * parseTerm() for the operands, which handles * and / (higher
   * precedence).
   *
   * The pattern `left = ...; while match(op): right = ...; left = BinaryOp(...)`
   * is the standard way to handle left-associative operators in recursive
   * descent. It ensures that `1 + 2 + 3` parses as `(1 + 2) + 3`.
   *
   * HOW LEFT-ASSOCIATIVITY WORKS:
   *
   *     Parsing `1 + 2 + 3`:
   *
   *     1. left = NumberLiteral(1)         [parseTerm returns 1]
   *     2. See +, consume it
   *     3. right = NumberLiteral(2)         [parseTerm returns 2]
   *     4. left = BinaryOp(1, "+", 2)       [combine into tree]
   *     5. See +, consume it
   *     6. right = NumberLiteral(3)         [parseTerm returns 3]
   *     7. left = BinaryOp((1+2), "+", 3)   [combine again — LEFT associative!]
   *
   * @returns An Expression node representing the parsed expression.
   */
  private parseExpression(): Expression {
    // Start by parsing the left operand (a term).
    let left: Expression = this.parseTerm();

    // Keep consuming + and - operators, building up the tree.
    let opToken: Token | null;
    while ((opToken = this.match("PLUS", "MINUS")) !== null) {
      // Parse the right operand (another term).
      const right = this.parseTerm();
      // Build a BinaryOp node with the left and right operands.
      // This becomes the new "left" for the next iteration, which
      // is what makes the result left-associative.
      left = { kind: "BinaryOp", left, op: opToken.value, right };
    }

    return left;
  }

  /**
   * Parse a term: multiplication and division.
   *
   * Grammar: term = factor ((STAR | SLASH) factor)*
   *
   * This handles HIGHER precedence operators: * and /. It calls
   * parseFactor() for the operands, which handles the highest
   * precedence items (numbers, names, parenthesized expressions).
   *
   * The structure is identical to parseExpression() — just with
   * different operators and different sub-rule calls. This pattern
   * can be extended to any number of precedence levels.
   *
   * @returns An Expression node representing the parsed term.
   */
  private parseTerm(): Expression {
    // Start by parsing the left operand (a factor).
    let left: Expression = this.parseFactor();

    // Keep consuming * and / operators.
    let opToken: Token | null;
    while ((opToken = this.match("STAR", "SLASH")) !== null) {
      // Parse the right operand (another factor).
      const right = this.parseFactor();
      left = { kind: "BinaryOp", left, op: opToken.value, right };
    }

    return left;
  }

  /**
   * Parse a factor: the highest-precedence items.
   *
   * Grammar: factor = NUMBER | STRING | NAME | LPAREN expression RPAREN
   *
   * Factors are the "atoms" of our expressions — the indivisible units.
   * They include:
   *
   * - NUMBER: A numeric literal like 42.
   * - STRING: A string literal like "hello".
   * - NAME:   A variable reference like x.
   * - (expr): A parenthesized expression, which lets the programmer
   *           override the default precedence. `(1 + 2) * 3` forces
   *           addition to happen before multiplication.
   *
   * The parenthesized case is where the "recursive" in "recursive descent"
   * really shines. When we see `(`, we call parseExpression() — which
   * might call parseTerm() which calls parseFactor() which might
   * see another `(` and call parseExpression() again! The call stack
   * naturally handles arbitrarily nested parentheses.
   *
   * @returns An Expression node for the parsed factor.
   * @throws ParseError if the current token can't start a factor.
   */
  private parseFactor(): Expression {
    const token = this.peek();

    // --- NUMBER literal ---
    // A bare number like 42. We consume it and wrap it in a NumberLiteral.
    if (token.type === "NUMBER") {
      this.advance();
      return { kind: "NumberLiteral", value: parseInt(token.value, 10) };
    }

    // --- STRING literal ---
    // A quoted string like "hello". The lexer already stripped the quotes.
    if (token.type === "STRING") {
      this.advance();
      return { kind: "StringLiteral", value: token.value };
    }

    // --- NAME (variable reference) ---
    // A variable name like x or total. We wrap it in a Name node.
    if (token.type === "NAME") {
      this.advance();
      return { kind: "Name", name: token.value };
    }

    // --- Parenthesized expression ---
    // An expression wrapped in ( and ). This lets the programmer override
    // the default operator precedence.
    //
    // We consume the (, parse the inner expression (which can be
    // arbitrarily complex), then consume the closing ).
    if (token.type === "LPAREN") {
      this.advance(); // consume the (
      const expression = this.parseExpression();
      this.expect("RPAREN"); // consume the ) — error if missing!
      return expression;
    }

    // --- ERROR: unexpected token ---
    // If we get here, the current token can't start a factor. This is
    // a syntax error. Common causes:
    //   - A stray operator: `+ 1` (nothing on the left)
    //   - A missing operand: `1 +` (nothing on the right)
    //   - A typo or unexpected character
    throw new ParseError(
      `Unexpected token ${token.type} (${JSON.stringify(token.value)})`,
      token,
    );
  }
}
