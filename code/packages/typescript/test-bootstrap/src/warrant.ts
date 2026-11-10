/**
 * Warrant Specification Compiler & Claim Verifier (VER04g).
 *
 * Implements Level 1 Warrant parser, evaluator, host binding compiler, and
 * universal claim verifier, integrating with @coding-adventures/test-bootstrap
 * and the Level 2 PBT engine.
 *
 * Conforms to:
 * - code/specs/VER04-warrant-language.md
 * - code/specs/VER04a-warrant-source-admission.md
 * - code/specs/VER04b-warrant-layout-capture.md
 * - code/specs/VER04c-guarded-warrant-parser.md
 * - code/specs/VER04e-warrant-codegen.md
 * - code/specs/VER04f-claims-to-properties.md
 * - code/specs/VER04g-typescript-warrant-compiler.md
 */

import { Gen, assertProperty } from "./pbt.js";
import { pass, fail } from "./verdict.js";
import type { Verdict } from "./types.js";

// --- Closed Diagnostic Error (VER04c §3) -----------------------------------

export type ParserErrorKind =
  | "InvalidLimits"
  | "Work"
  | "NodeBudget"
  | "RecursionDepth"
  | "UnexpectedToken"
  | "UnexpectedEof"
  | "InconsistentIndentation"
  | "IllegalTab"
  | "MalformedLiteral";

export class WarrantParserError extends Error {
  public readonly code = "W0001";
  constructor(
    public readonly kind: ParserErrorKind,
    public readonly line: number,
    public readonly col: number,
    message?: string
  ) {
    super(`[W0001] ${kind} at line ${line}:${col}${message ? `: ${message}` : ""}`);
    this.name = "WarrantParserError";
  }
}

// --- AST Definitions --------------------------------------------------------

export interface WarrantType {
  name: string;
  genericArg?: WarrantType | number | string;
  genericArgs?: (WarrantType | number | string)[];
}

export interface WarrantParam {
  name: string;
  type: WarrantType;
  refinement?: WarrantExpr;
}

export type WarrantPattern =
  | { kind: "none" }
  | { kind: "some"; binding: string }
  | { kind: "ok"; binding: string }
  | { kind: "err"; binding: string };

export interface WarrantMatchArm {
  pattern: WarrantPattern;
  expr: WarrantExpr;
}

export type WarrantExpr =
  | { kind: "bool"; value: boolean }
  | { kind: "int"; value: bigint }
  | { kind: "bytes"; value: Uint8Array }
  | { kind: "list"; items: WarrantExpr[] }
  | { kind: "ident"; name: string }
  | { kind: "unary"; op: "not" | "-"; expr: WarrantExpr }
  | { kind: "binary"; op: string; left: WarrantExpr; right: WarrantExpr }
  | { kind: "conditional"; condition: WarrantExpr; thenBranch: WarrantExpr; elseBranch: WarrantExpr }
  | { kind: "match"; expr: WarrantExpr; arms: WarrantMatchArm[] }
  | { kind: "quantified"; quantifier: "forall" | "exists"; binders: WarrantParam[]; body: WarrantExpr }
  | { kind: "call"; callee: string; genericArg?: number; args: WarrantExpr[] }
  | { kind: "len"; expr: WarrantExpr }
  | { kind: "ok"; value: WarrantExpr }
  | { kind: "err"; errorType: string; variant: string }
  | { kind: "some"; value: WarrantExpr }
  | { kind: "none" };

export interface WarrantModel {
  name: string;
  params: WarrantParam[];
  returnType: WarrantType;
  body: WarrantExpr;
}

export interface WarrantOp {
  name: string;
  genericParam?: string;
  params: WarrantParam[];
  returnType: WarrantType;
  isFresh: boolean;
}

export interface WarrantExample {
  name: string;
  lhs: WarrantExpr;
  rhs: WarrantExpr;
}

export interface WarrantClaim {
  name: string;
  quantifier: "forall" | "exists";
  binders: WarrantParam[];
  refinement?: WarrantExpr;
  lhs: WarrantExpr;
  rhs: WarrantExpr;
}

export interface WarrantInputContract {
  opName: string;
  inputName: string;
  isHostile: boolean;
  rejects?: {
    condition: WarrantExpr;
    errorType: string;
    variant: string;
  };
  ensures: string[];
}

export interface WarrantModule {
  name: string;
  errors: Map<string, string[]>;
  models: Map<string, WarrantModel>;
  operations: Map<string, WarrantOp>;
  examples: WarrantExample[];
  claims: WarrantClaim[];
  inputContracts: WarrantInputContract[];
}

// --- Lexer & Layout Analyzer (VER04b, VER04c) --------------------------------

export type TokenType =
  | "IDENT"
  | "NUMBER"
  | "BYTES"
  | "SYMBOL"
  | "NEWLINE"
  | "INDENT"
  | "DEDENT"
  | "EOF";

export interface Token {
  type: TokenType;
  text: string;
  line: number;
  col: number;
  hasSpaceAfter?: boolean;
}

const NUM_RE = /(?:0|[1-9][0-9]*)(?:u64|u8|i128)?/y;
const IDENT_RE = /[a-zA-Z_][a-zA-Z0-9_]*/y;

export function tokenizeWarrant(source: string): Token[] {
  const tokens: Token[] = [];
  const lines = source.split(/\r?\n/);
  const indentStack = [0];
  let parenDepth = 0;

  for (let i = 0; i < lines.length; i++) {
    const lineNum = i + 1;
    const rawLine = lines[i];

    // Check for illegal tabs in leading whitespace (VER04a)
    const leadingWhitespaceMatch = rawLine.match(/^[ \t]*/);
    const leadingWhitespace = leadingWhitespaceMatch ? leadingWhitespaceMatch[0] : "";
    if (leadingWhitespace.includes("\t")) {
      throw new WarrantParserError("IllegalTab", lineNum, 1);
    }

    // Strip comments (-- or #)
    let line = rawLine;
    const commentIdx = line.search(/--|#/);
    if (commentIdx !== -1) {
      line = line.substring(0, commentIdx);
    }

    // Skip empty lines
    if (line.trim().length === 0) {
      continue;
    }

    // Layout indentation processing
    if (parenDepth === 0) {
      const indent = leadingWhitespace.length;
      const currentIndent = indentStack[indentStack.length - 1];

      if (indent > currentIndent) {
        indentStack.push(indent);
        tokens.push({ type: "INDENT", text: "", line: lineNum, col: 1 });
      } else if (indent < currentIndent) {
        if (!indentStack.includes(indent)) {
          throw new WarrantParserError("InconsistentIndentation", lineNum, 1);
        }
        while (indentStack[indentStack.length - 1] > indent) {
          indentStack.pop();
          tokens.push({ type: "DEDENT", text: "", line: lineNum, col: 1 });
        }
      }
    }

    let pos = leadingWhitespace.length;
    while (pos < line.length) {
      const ch = line[pos];

      // Whitespace
      if (ch === " " || ch === "\t") {
        pos++;
        if (tokens.length > 0) {
          tokens[tokens.length - 1].hasSpaceAfter = true;
        }
        continue;
      }

      // Track parentheses / bracket depth
      if (ch === "(" || ch === "[") {
        parenDepth++;
      } else if (ch === ")" || ch === "]") {
        parenDepth = Math.max(0, parenDepth - 1);
      }

      // Byte string literal: b"..." (must be lowercase hex pairs: /b"(?:[0-9a-f]{2})*"/)
      if (line.startsWith('b"', pos)) {
        const startPos = pos;
        const endQuote = line.indexOf('"', pos + 2);
        if (endQuote === -1) {
          throw new WarrantParserError("MalformedLiteral", lineNum, pos + 1);
        }
        const hex = line.substring(pos + 2, endQuote);
        if (hex.length % 2 !== 0 || !/^(?:[0-9a-f]{2})*$/.test(hex)) {
          throw new WarrantParserError("MalformedLiteral", lineNum, startPos + 1);
        }
        tokens.push({ type: "BYTES", text: hex, line: lineNum, col: pos + 1 });
        pos = endQuote + 1;
        continue;
      }

      // Multi-character symbols
      if (
        line.startsWith("==", pos) ||
        line.startsWith("!=", pos) ||
        line.startsWith("<=", pos) ||
        line.startsWith(">=", pos) ||
        line.startsWith("=>", pos) ||
        line.startsWith("->", pos)
      ) {
        tokens.push({ type: "SYMBOL", text: line.substring(pos, pos + 2), line: lineNum, col: pos + 1 });
        pos += 2;
        continue;
      }

      // Single character symbols
      if (":=,.<>()[]|+*!&-%^;".includes(ch)) {
        tokens.push({ type: "SYMBOL", text: ch, line: lineNum, col: pos + 1 });
        pos++;
        continue;
      }

      // Numbers with optional suffixes (0|[1-9][0-9]*)
      if (/[0-9]/.test(ch)) {
        // Disallow leading zeros like 01 (VER04 §2)
        if (ch === "0" && pos + 1 < line.length && /[0-9]/.test(line[pos + 1])) {
          throw new WarrantParserError("MalformedLiteral", lineNum, pos + 1);
        }
        NUM_RE.lastIndex = pos;
        const numMatch = NUM_RE.exec(line);
        if (numMatch) {
          tokens.push({ type: "NUMBER", text: numMatch[0], line: lineNum, col: pos + 1 });
          pos += numMatch[0].length;
          continue;
        }
      }

      // Identifiers and keywords
      if (/[a-zA-Z_]/.test(ch)) {
        IDENT_RE.lastIndex = pos;
        const identMatch = IDENT_RE.exec(line);
        if (identMatch) {
          const text = identMatch[0];
          tokens.push({ type: "IDENT", text, line: lineNum, col: pos + 1 });
          pos += text.length;
          continue;
        }
      }

      throw new WarrantParserError("UnexpectedToken", lineNum, pos + 1);
    }

    if (parenDepth === 0) {
      tokens.push({ type: "NEWLINE", text: "\n", line: lineNum, col: line.length + 1 });
    }
  }

  while (indentStack.length > 1) {
    indentStack.pop();
    tokens.push({ type: "DEDENT", text: "", line: lines.length, col: 1 });
  }

  tokens.push({ type: "EOF", text: "", line: lines.length, col: 1 });
  return tokens;
}

// --- Guarded Parser (VER04c) -------------------------------------------------

export class WarrantParser {
  private tokens: Token[];
  private pos = 0;
  private nodeCount = 0;
  private depth = 0;
  private readonly maxNodes = 65_536;
  private readonly maxDepth = 64;

  constructor(tokens: Token[]) {
    this.tokens = tokens;
  }

  private countNode(): void {
    this.nodeCount++;
    if (this.nodeCount > this.maxNodes) {
      const tok = this.peek();
      throw new WarrantParserError("NodeBudget", tok.line, tok.col);
    }
  }

  private enter(): void {
    this.depth++;
    if (this.depth > this.maxDepth) {
      const tok = this.peek();
      throw new WarrantParserError("RecursionDepth", tok.line, tok.col);
    }
  }

  private leave(): void {
    this.depth--;
  }

  private peek(): Token {
    return this.tokens[this.pos];
  }

  private next(): Token {
    const tok = this.tokens[this.pos];
    if (this.pos < this.tokens.length - 1) {
      this.pos++;
    }
    return tok;
  }

  private match(type: TokenType, text?: string): boolean {
    const tok = this.peek();
    if (tok.type === type && (text === undefined || tok.text === text)) {
      this.next();
      return true;
    }
    return false;
  }

  private expect(type: TokenType, text?: string): Token {
    const tok = this.peek();
    if (tok.type !== type || (text !== undefined && tok.text !== text)) {
      if (tok.type === "EOF") {
        throw new WarrantParserError("UnexpectedEof", tok.line, tok.col);
      }
      throw new WarrantParserError("UnexpectedToken", tok.line, tok.col);
    }
    return this.next();
  }

  private skipNewlines(): void {
    while (this.peek().type === "NEWLINE") {
      this.next();
    }
  }

  public parseDocument(): WarrantModule {
    this.skipNewlines();
    this.expect("IDENT", "warrant");
    const moduleName = this.expect("IDENT").text;
    this.expect("NEWLINE");

    const module: WarrantModule = {
      name: moduleName,
      errors: new Map(),
      models: new Map(),
      operations: new Map(),
      examples: [],
      claims: [],
      inputContracts: [],
    };

    if (this.match("INDENT")) {
      while (this.peek().type !== "DEDENT" && this.peek().type !== "EOF") {
        this.skipNewlines();
        if (this.peek().type === "DEDENT" || this.peek().type === "EOF") break;

        const declTok = this.peek();
        if (declTok.type === "IDENT") {
          switch (declTok.text) {
            case "error":
              this.parseErrorDecl(module);
              break;
            case "model":
              this.parseModelDecl(module);
              break;
            case "op":
              this.parseOpDecl(module);
              break;
            case "assume":
              this.parseAssumeDecl();
              break;
            case "labels":
              this.parseLabelsDecl();
              break;
            case "input":
              this.parseInputDecl(module);
              break;
            case "bound":
              this.parseBoundDecl();
              break;
            case "example":
              this.parseExampleDecl(module);
              break;
            case "claim":
              this.parseClaimDecl(module);
              break;
            default:
              throw new WarrantParserError("UnexpectedToken", declTok.line, declTok.col);
          }
        } else {
          throw new WarrantParserError("UnexpectedToken", declTok.line, declTok.col);
        }
        this.skipNewlines();
      }
      this.expect("DEDENT");
    }

    this.skipNewlines();
    if (this.peek().type !== "EOF") {
      const tok = this.peek();
      throw new WarrantParserError("UnexpectedToken", tok.line, tok.col);
    }

    return module;
  }

  private parseErrorDecl(mod: WarrantModule): void {
    this.expect("IDENT", "error");
    const name = this.expect("IDENT").text;
    this.expect("SYMBOL", "=");
    const variants: string[] = [];
    variants.push(this.expect("IDENT").text);
    while (this.match("SYMBOL", "|")) {
      variants.push(this.expect("IDENT").text);
    }
    mod.errors.set(name, variants);
    this.expect("NEWLINE");
  }

  private parseModelDecl(mod: WarrantModule): void {
    this.expect("IDENT", "model");
    const name = this.expect("IDENT").text;
    if (this.match("SYMBOL", "=")) {
      this.parseType();
      this.expect("NEWLINE");
      return;
    }
    this.expect("SYMBOL", "(");
    const params = this.parseParamList();
    this.expect("SYMBOL", ")");
    this.expect("SYMBOL", "->");
    const returnType = this.parseType();
    this.expect("SYMBOL", "=");
    const body = this.parseExpr();
    mod.models.set(name, { name, params, returnType, body });
    this.expect("NEWLINE");
  }

  private parseOpDecl(mod: WarrantModule): void {
    this.expect("IDENT", "op");
    const name = this.expect("IDENT").text;
    let genericParam: string | undefined;

    if (this.match("SYMBOL", "<")) {
      genericParam = this.expect("IDENT").text;
      this.expect("SYMBOL", ":");
      this.parseType();
      this.expect("SYMBOL", ">");
    }

    this.expect("SYMBOL", "(");
    const params = this.parseParamList();
    this.expect("SYMBOL", ")");
    this.expect("SYMBOL", "->");
    const returnType = this.parseType();

    let isFresh = false;
    if (this.peek().type === "IDENT" && this.peek().text === "fresh") {
      this.next();
      isFresh = true;
    }

    mod.operations.set(name, { name, genericParam, params, returnType, isFresh });
    this.expect("NEWLINE");
  }

  private parseAssumeDecl(): void {
    this.expect("IDENT", "assume");
    this.expect("IDENT"); // platform or environment
    this.expect("SYMBOL", ":");
    this.expect("NEWLINE");
    this.expect("INDENT");
    while (this.peek().type !== "DEDENT" && this.peek().type !== "EOF") {
      this.next();
    }
    this.expect("DEDENT");
  }

  private parseLabelsDecl(): void {
    this.expect("IDENT", "labels");
    this.expect("IDENT"); // operation name
    this.expect("SYMBOL", ":");
    this.expect("NEWLINE");
    this.expect("INDENT");
    while (this.peek().type !== "DEDENT" && this.peek().type !== "EOF") {
      this.parseQualifiedName();
      const classTok = this.expect("IDENT");
      if (classTok.text !== "public" && classTok.text !== "secret") {
        throw new WarrantParserError("UnexpectedToken", classTok.line, classTok.col);
      }
      this.expect("NEWLINE");
    }
    this.expect("DEDENT");
  }

  private parseInputDecl(mod: WarrantModule): void {
    this.expect("IDENT", "input");
    const target = this.parseQualifiedName();
    const parts = target.split(".");
    const opName = parts[0];
    const inputName = parts.slice(1).join(".");

    this.expect("SYMBOL", ":");
    this.expect("NEWLINE");
    this.expect("INDENT");

    // Must start with 'hostile' (VER04 §2)
    this.expect("IDENT", "hostile");
    this.expect("NEWLINE");

    let rejects: WarrantInputContract["rejects"];
    if (this.match("IDENT", "rejects")) {
      const condition = this.parseExpr();
      this.expect("IDENT", "with");
      const errTarget = this.parseQualifiedName();
      const errParts = errTarget.split(".");
      rejects = {
        condition,
        errorType: errParts[0],
        variant: errParts[1] ?? "",
      };
      this.expect("NEWLINE");
    }

    // Must end with 'ensures' (VER04 §2)
    this.expect("IDENT", "ensures");
    const ensures: string[] = [];
    ensures.push(this.expect("IDENT").text);
    while (this.match("SYMBOL", ",")) {
      ensures.push(this.expect("IDENT").text);
    }
    this.expect("NEWLINE");
    this.expect("DEDENT");

    mod.inputContracts.push({
      opName,
      inputName,
      isHostile: true,
      rejects,
      ensures,
    });
  }

  private parseBoundDecl(): void {
    this.expect("IDENT", "bound");
    this.expect("IDENT"); // operation name
    this.expect("SYMBOL", ":");
    this.expect("NEWLINE");
    this.expect("INDENT");
    while (this.peek().type !== "DEDENT" && this.peek().type !== "EOF") {
      this.expect("IDENT"); // metric e.g. steps
      this.expect("SYMBOL", "<=");
      this.parseExpr();
      this.expect("NEWLINE");
    }
    this.expect("DEDENT");
  }

  private parseExampleDecl(mod: WarrantModule): void {
    this.expect("IDENT", "example");
    const name = this.expect("IDENT").text;
    this.expect("SYMBOL", ":");
    this.expect("NEWLINE");
    this.expect("INDENT");
    const expr = this.parseExpr();
    let lhs: WarrantExpr = expr;
    let rhs: WarrantExpr = { kind: "bool", value: true };
    if (expr.kind === "binary" && expr.op === "==") {
      lhs = expr.left;
      rhs = expr.right;
    }
    mod.examples.push({ name, lhs, rhs });
    this.expect("NEWLINE");
    this.expect("DEDENT");
  }

  private parseClaimDecl(mod: WarrantModule): void {
    this.expect("IDENT", "claim");
    const name = this.expect("IDENT").text;
    this.expect("SYMBOL", ":");
    this.expect("NEWLINE");
    this.expect("INDENT");

    const expr = this.parseExpr();
    let quantifier: "forall" | "exists" = "forall";
    let binders: WarrantParam[] = [];
    let refinement: WarrantExpr | undefined;
    let body = expr;

    if (expr.kind === "quantified") {
      quantifier = expr.quantifier;
      binders = expr.binders;
      body = expr.body;
      const refParams = binders.filter((b) => b.refinement);
      if (refParams.length > 0) {
        refinement = refParams[0].refinement;
      }
    }

    let lhs: WarrantExpr = body;
    let rhs: WarrantExpr = { kind: "bool", value: true };
    if (body.kind === "binary" && body.op === "==") {
      lhs = body.left;
      rhs = body.right;
    }

    mod.claims.push({
      name,
      quantifier,
      binders,
      refinement,
      lhs,
      rhs,
    });
    this.expect("NEWLINE");
    this.expect("DEDENT");
  }

  private parseParamList(): WarrantParam[] {
    const params: WarrantParam[] = [];
    if (this.peek().type === "IDENT") {
      params.push(this.parseParam());
      while (this.match("SYMBOL", ",")) {
        params.push(this.parseParam());
      }
    }
    return params;
  }

  private parseParam(): WarrantParam {
    const name = this.expect("IDENT").text;
    this.expect("SYMBOL", ":");
    const type = this.parseType();
    let refinement: WarrantExpr | undefined;
    if (this.match("SYMBOL", "|")) {
      refinement = this.parseExpr();
    }
    return { name, type, refinement };
  }

  public parseType(): WarrantType {
    const name = this.expect("IDENT").text;
    if (this.match("SYMBOL", "<")) {
      if (this.match("SYMBOL", ">")) {
        // Empty width is invalid: Bytes<>
        const prev = this.tokens[this.pos - 1];
        throw new WarrantParserError("UnexpectedToken", prev.line, prev.col);
      }

      const args: (WarrantType | number | string)[] = [];
      const parseArg = (): WarrantType | number | string => {
        if (this.peek().type === "NUMBER") {
          return parseInt(this.next().text, 10);
        }
        return this.parseType();
      };

      args.push(parseArg());
      while (this.match("SYMBOL", ",")) {
        args.push(parseArg());
      }
      this.expect("SYMBOL", ">");

      if (name === "Result" && args.length !== 2) {
        const prev = this.tokens[this.pos - 1];
        throw new WarrantParserError("UnexpectedToken", prev.line, prev.col);
      }

      return {
        name,
        genericArg: args[0],
        genericArgs: args,
      };
    }
    return { name };
  }

  private parseQualifiedName(): string {
    let name = this.expect("IDENT").text;
    while (this.match("SYMBOL", ".")) {
      name += "." + this.expect("IDENT").text;
    }
    return name;
  }

  // --- Expression Grammar (warrant.grammar) ----------------------------------

  public parseExpr(): WarrantExpr {
    this.enter();
    this.countNode();
    try {
      const tok = this.peek();

      // 1. Quantified expressions: (forall | exists) binders . expr
      if (tok.type === "IDENT" && (tok.text === "forall" || tok.text === "exists")) {
        const quantifier = this.next().text as "forall" | "exists";
        const binders = this.parseParamList();
        const dotTok = this.expect("SYMBOL", ".");
        if (!dotTok.hasSpaceAfter) {
          throw new WarrantParserError("UnexpectedToken", dotTok.line, dotTok.col);
        }
        const body = this.parseExpr();
        return { kind: "quantified", quantifier, binders, body };
      }

      // 2. Conditional expressions: if expr then expr else expr
      if (tok.type === "IDENT" && tok.text === "if") {
        this.next();
        const condition = this.parseExpr();
        this.expect("IDENT", "then");
        const thenBranch = this.parseExpr();
        this.expect("IDENT", "else");
        const elseBranch = this.parseExpr();
        return { kind: "conditional", condition, thenBranch, elseBranch };
      }

      // 3. Match expressions: match expr with pattern -> expr (| pattern -> expr)*
      if (tok.type === "IDENT" && tok.text === "match") {
        this.next();
        const matchExpr = this.parseExpr();
        this.expect("IDENT", "with");
        const arms: WarrantMatchArm[] = [];
        arms.push(this.parseMatchArm());
        while (this.match("SYMBOL", "|")) {
          arms.push(this.parseMatchArm());
        }
        return { kind: "match", expr: matchExpr, arms };
      }

      // 4. Implication & binary expressions
      return this.parseImplication();
    } finally {
      this.leave();
    }
  }

  private parseMatchArm(): WarrantMatchArm {
    const patternTok = this.expect("IDENT");
    let pattern: WarrantPattern;
    switch (patternTok.text) {
      case "None":
        pattern = { kind: "none" };
        break;
      case "Some":
        this.expect("SYMBOL", "(");
        pattern = { kind: "some", binding: this.expect("IDENT").text };
        this.expect("SYMBOL", ")");
        break;
      case "Ok":
        this.expect("SYMBOL", "(");
        pattern = { kind: "ok", binding: this.expect("IDENT").text };
        this.expect("SYMBOL", ")");
        break;
      case "Err":
        this.expect("SYMBOL", "(");
        pattern = { kind: "err", binding: this.expect("IDENT").text };
        this.expect("SYMBOL", ")");
        break;
      default:
        throw new WarrantParserError("UnexpectedToken", patternTok.line, patternTok.col);
    }
    this.expect("SYMBOL", "->");
    const expr = this.parseExpr();
    return { pattern, expr };
  }

  private parseImplication(): WarrantExpr {
    let left = this.parseOr();
    if (this.match("SYMBOL", "=>")) {
      const right = this.parseExpr();
      left = { kind: "binary", op: "=>", left, right };
    }
    return left;
  }

  private parseOr(): WarrantExpr {
    let left = this.parseAnd();
    while (this.match("IDENT", "or")) {
      const right = this.parseAnd();
      left = { kind: "binary", op: "or", left, right };
    }
    return left;
  }

  private parseAnd(): WarrantExpr {
    let left = this.parseComparison();
    while (this.match("IDENT", "and")) {
      const right = this.parseComparison();
      left = { kind: "binary", op: "and", left, right };
    }
    return left;
  }

  private parseComparison(): WarrantExpr {
    let left = this.parseAddSub();
    if (
      this.peek().type === "SYMBOL" &&
      ["==", "!=", "<", "<=", ">", ">="].includes(this.peek().text)
    ) {
      const op = this.next().text;
      const right = this.parseAddSub();
      left = { kind: "binary", op, left, right };

      // Chained comparisons are prohibited (VER04 §2)
      if (
        this.peek().type === "SYMBOL" &&
        ["==", "!=", "<", "<=", ">", ">="].includes(this.peek().text)
      ) {
        const chainedTok = this.peek();
        throw new WarrantParserError("UnexpectedToken", chainedTok.line, chainedTok.col);
      }
    }
    return left;
  }

  private parseAddSub(): WarrantExpr {
    let left = this.parseMulDiv();
    while (this.peek().type === "SYMBOL" && ["+", "-"].includes(this.peek().text)) {
      const op = this.next().text;
      const right = this.parseMulDiv();
      left = { kind: "binary", op, left, right };
    }
    return left;
  }

  private parseMulDiv(): WarrantExpr {
    let left = this.parseUnary();
    while (this.peek().type === "SYMBOL" && this.peek().text === "*") {
      const op = this.next().text;
      const right = this.parseUnary();
      left = { kind: "binary", op, left, right };
    }
    return left;
  }

  private parseUnary(): WarrantExpr {
    if (this.match("IDENT", "not")) {
      return { kind: "unary", op: "not", expr: this.parseUnary() };
    }
    if (this.match("SYMBOL", "-")) {
      return { kind: "unary", op: "-", expr: this.parseUnary() };
    }
    return this.parsePrimary();
  }

  private parsePrimary(): WarrantExpr {
    const tok = this.peek();

    // Grouping
    if (this.match("SYMBOL", "(")) {
      const expr = this.parseExpr();
      this.expect("SYMBOL", ")");
      return expr;
    }

    // List literal: [elem, ...]
    if (this.match("SYMBOL", "[")) {
      const items: WarrantExpr[] = [];
      if (this.peek().type !== "SYMBOL" || this.peek().text !== "]") {
        items.push(this.parseExpr());
        while (this.match("SYMBOL", ",")) {
          items.push(this.parseExpr());
        }
      }
      this.expect("SYMBOL", "]");
      return { kind: "list", items };
    }

    // Literals
    if (tok.type === "BYTES") {
      this.next();
      return { kind: "bytes", value: decodeHex(tok.text) };
    }
    if (tok.type === "NUMBER") {
      this.next();
      const cleanText = tok.text.replace(/(?:u64|i128|u8)$/, "");
      return { kind: "int", value: BigInt(cleanText) };
    }
    if (tok.type === "IDENT") {
      if (tok.text === "true") {
        this.next();
        return { kind: "bool", value: true };
      }
      if (tok.text === "false") {
        this.next();
        return { kind: "bool", value: false };
      }
      if (tok.text === "None") {
        this.next();
        return { kind: "none" };
      }
      if (tok.text === "Ok") {
        this.next();
        this.expect("SYMBOL", "(");
        const val = this.parseExpr();
        this.expect("SYMBOL", ")");
        return { kind: "ok", value: val };
      }
      if (tok.text === "Err") {
        this.next();
        this.expect("SYMBOL", "(");
        const errorType = this.expect("IDENT").text;
        this.expect("SYMBOL", ".");
        const variant = this.expect("IDENT").text;
        this.expect("SYMBOL", ")");
        return { kind: "err", errorType, variant };
      }
      if (tok.text === "Some") {
        this.next();
        this.expect("SYMBOL", "(");
        const val = this.parseExpr();
        this.expect("SYMBOL", ")");
        return { kind: "some", value: val };
      }

      // Check for len(x)
      if (tok.text === "len") {
        this.next();
        this.expect("SYMBOL", "(");
        const arg = this.parseExpr();
        this.expect("SYMBOL", ")");
        return { kind: "len", expr: arg };
      }

      // Identifier or Function/Op call
      this.next();
      // Generic call e.g. eq_fixed<32>(a, b)
      let genericArg: number | undefined;
      if (
        this.peek().type === "SYMBOL" &&
        this.peek().text === "<" &&
        this.tokens[this.pos + 1]?.type === "NUMBER" &&
        this.tokens[this.pos + 2]?.type === "SYMBOL" &&
        this.tokens[this.pos + 2]?.text === ">" &&
        this.tokens[this.pos + 3]?.type === "SYMBOL" &&
        this.tokens[this.pos + 3]?.text === "("
      ) {
        this.next(); // consume <
        genericArg = parseInt(this.next().text, 10); // consume NUMBER
        this.next(); // consume >
      }

      if (this.match("SYMBOL", "(")) {
        const args: WarrantExpr[] = [];
        if (this.peek().type !== "SYMBOL" || this.peek().text !== ")") {
          args.push(this.parseExpr());
          while (this.match("SYMBOL", ",")) {
            args.push(this.parseExpr());
          }
        }
        this.expect("SYMBOL", ")");
        return { kind: "call", callee: tok.text, genericArg, args };
      }

      return { kind: "ident", name: tok.text };
    }

    throw new WarrantParserError("UnexpectedToken", tok.line, tok.col);
  }
}

export function parseWarrant(source: string): WarrantModule {
  const tokens = tokenizeWarrant(source);
  const parser = new WarrantParser(tokens);
  return parser.parseDocument();
}

// --- Ground Model Evaluator (VER04e) -----------------------------------------

export interface StepBudget {
  count: number;
  limit: number;
}

export interface EvaluatorContext {
  models: Map<string, WarrantModel>;
  hostOps: Record<string, (...args: unknown[]) => unknown>;
  env: Map<string, unknown>;
  depth: number;
  budget: StepBudget;
}

function toBigInt(v: unknown): bigint {
  if (typeof v === "bigint") return v;
  if (typeof v === "number" && Number.isInteger(v)) return BigInt(v);
  throw new Error(`Expected integer value in evaluation, got ${typeof v}`);
}

export function evaluateWarrantExpr(expr: WarrantExpr, ctx: EvaluatorContext): unknown {
  if (ctx.depth > 64) {
    throw new Error("Evaluation depth exceeded budget (W0703)");
  }
  ctx.budget.count++;
  if (ctx.budget.count > ctx.budget.limit) {
    throw new Error("Evaluation step limit exceeded budget (W0703)");
  }

  const childCtx: EvaluatorContext = {
    ...ctx,
    depth: ctx.depth + 1,
  };

  switch (expr.kind) {
    case "bool":
      return expr.value;
    case "int":
      return expr.value;
    case "bytes":
      return expr.value;
    case "list":
      return expr.items.map((item) => evaluateWarrantExpr(item, childCtx));
    case "ident": {
      if (!ctx.env.has(expr.name)) {
        throw new Error(`Unbound identifier '${expr.name}' in model evaluation`);
      }
      return ctx.env.get(expr.name);
    }
    case "len": {
      const val = evaluateWarrantExpr(expr.expr, childCtx);
      if (val instanceof Uint8Array || Array.isArray(val)) {
        return BigInt(val.length);
      }
      throw new Error("len() expects Bytes or List argument");
    }
    case "unary": {
      if (expr.op === "not") {
        return !evaluateWarrantExpr(expr.expr, childCtx);
      }
      if (expr.op === "-") {
        return -toBigInt(evaluateWarrantExpr(expr.expr, childCtx));
      }
      throw new Error(`Unsupported unary op ${expr.op}`);
    }
    case "binary": {
      const left = evaluateWarrantExpr(expr.left, childCtx);
      const right = evaluateWarrantExpr(expr.right, childCtx);

      switch (expr.op) {
        case "==":
          return areWarrantValuesEqual(left, right);
        case "!=":
          return !areWarrantValuesEqual(left, right);
        case "<":
          return toBigInt(left) < toBigInt(right);
        case "<=":
          return toBigInt(left) <= toBigInt(right);
        case ">":
          return toBigInt(left) > toBigInt(right);
        case ">=":
          return toBigInt(left) >= toBigInt(right);
        case "and":
          return Boolean(left) && Boolean(right);
        case "or":
          return Boolean(left) || Boolean(right);
        case "=>":
          return !left || Boolean(right);
        case "+":
          return toBigInt(left) + toBigInt(right);
        case "-":
          return toBigInt(left) - toBigInt(right);
        case "*":
          return toBigInt(left) * toBigInt(right);
        default:
          throw new Error(`Unsupported binary op ${expr.op}`);
      }
    }
    case "conditional": {
      const cond = evaluateWarrantExpr(expr.condition, childCtx);
      if (cond) {
        return evaluateWarrantExpr(expr.thenBranch, childCtx);
      }
      return evaluateWarrantExpr(expr.elseBranch, childCtx);
    }
    case "match": {
      const target = evaluateWarrantExpr(expr.expr, childCtx);
      for (const arm of expr.arms) {
        if (arm.pattern.kind === "none" && target === null) {
          return evaluateWarrantExpr(arm.expr, childCtx);
        }
        if (arm.pattern.kind === "some" && target && typeof target === "object" && "some" in target) {
          const armEnv = new Map(ctx.env);
          armEnv.set(arm.pattern.binding, (target as { some: unknown }).some);
          return evaluateWarrantExpr(arm.expr, { ...ctx, env: armEnv, depth: ctx.depth + 1 });
        }
        if (arm.pattern.kind === "ok" && target && typeof target === "object" && "ok" in target) {
          const armEnv = new Map(ctx.env);
          armEnv.set(arm.pattern.binding, (target as { ok: unknown }).ok);
          return evaluateWarrantExpr(arm.expr, { ...ctx, env: armEnv, depth: ctx.depth + 1 });
        }
        if (arm.pattern.kind === "err" && target && typeof target === "object" && "err" in target) {
          const armEnv = new Map(ctx.env);
          armEnv.set(arm.pattern.binding, (target as { err: unknown }).err);
          return evaluateWarrantExpr(arm.expr, { ...ctx, env: armEnv, depth: ctx.depth + 1 });
        }
      }
      throw new Error("Pattern match was not exhaustive");
    }
    case "call": {
      if (ctx.models.has(expr.callee)) {
        const model = ctx.models.get(expr.callee)!;
        if (expr.args.length !== model.params.length) {
          throw new Error(
            `Arity mismatch calling model '${expr.callee}': expected ${model.params.length} arguments, got ${expr.args.length}`
          );
        }
        const subEnv = new Map<string, unknown>();
        for (let i = 0; i < model.params.length; i++) {
          const argVal = evaluateWarrantExpr(expr.args[i], childCtx);
          subEnv.set(model.params[i].name, argVal);
        }
        return evaluateWarrantExpr(model.body, {
          ...ctx,
          env: subEnv,
          depth: ctx.depth + 1,
        });
      }

      const hostFn = ctx.hostOps[expr.callee];
      if (hostFn) {
        const evaluatedArgs = expr.args.map((a) => evaluateWarrantExpr(a, childCtx));
        if (expr.genericArg !== undefined) {
          return hostFn(expr.genericArg, ...evaluatedArgs);
        }
        return hostFn(...evaluatedArgs);
      }

      throw new Error(`Unknown function or operation '${expr.callee}'`);
    }
    case "ok":
      return { ok: evaluateWarrantExpr(expr.value, childCtx) };
    case "err":
      return { err: `${expr.errorType}.${expr.variant}` };
    case "some":
      return { some: evaluateWarrantExpr(expr.value, childCtx) };
    case "none":
      return null;
    case "quantified":
      throw new Error("Direct evaluation of ungrounded quantified expressions not supported");
  }
}

export function areWarrantValuesEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;

  if (a instanceof Uint8Array && b instanceof Uint8Array) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (a[i] !== b[i]) return false;
    }
    return true;
  }

  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (!areWarrantValuesEqual(a[i], b[i])) return false;
    }
    return true;
  }

  if (typeof a === "object" && a !== null && typeof b === "object" && b !== null) {
    const objA = a as Record<string, unknown>;
    const objB = b as Record<string, unknown>;
    if ("ok" in objA && "ok" in objB) {
      return areWarrantValuesEqual(objA.ok, objB.ok);
    }
    if ("err" in objA && "err" in objB) {
      return objA.err === objB.err;
    }
    if ("some" in objA && "some" in objB) {
      return areWarrantValuesEqual(objA.some, objB.some);
    }
  }

  return false;
}

// --- Warrant Test Suite Compiler (VER04e, VER04f) ----------------------------

export interface WarrantTestSuiteOptions {
  seed?: bigint | number;
  claimSamples?: number;
}

export interface WarrantTestResult {
  name: string;
  kind: "example" | "claim" | "contract";
  passed: boolean;
  verdict: Verdict;
}

export function executeWarrantModule(
  module: WarrantModule,
  hostOps: Record<string, (...args: any[]) => any>,
  options: WarrantTestSuiteOptions = {}
): WarrantTestResult[] {
  const results: WarrantTestResult[] = [];
  const baseCtx: EvaluatorContext = {
    models: module.models,
    hostOps,
    env: new Map(),
    depth: 0,
    budget: { count: 0, limit: 10_000 },
  };

  // 1. Execute ground examples (Level 1 Unit Tests)
  for (const example of module.examples) {
    try {
      baseCtx.budget.count = 0;
      baseCtx.depth = 0;

      let opCalled: WarrantOp | undefined;
      if (example.lhs.kind === "call") {
        opCalled = module.operations.get(example.lhs.callee);
      }

      const actual = evaluateWarrantExpr(example.lhs, baseCtx);
      const expected = evaluateWarrantExpr(example.rhs, baseCtx);

      let hasAliasingError = false;
      if (opCalled?.isFresh && actual && typeof actual === "object" && "ok" in actual) {
        const okVal = (actual as { ok: unknown }).ok;
        if (example.lhs.kind === "call") {
          for (const arg of example.lhs.args) {
            const evaluatedArg = evaluateWarrantExpr(arg, baseCtx);
            if (evaluatedArg === okVal) {
              results.push({
                name: example.name,
                kind: "example",
                passed: false,
                verdict: fail("fresh-aliasing", `fresh operation '${opCalled.name}' leaked alias`),
              });
              hasAliasingError = true;
              break;
            }
          }
        }
      }

      if (hasAliasingError) {
        continue;
      }

      if (areWarrantValuesEqual(actual, expected)) {
        results.push({
          name: example.name,
          kind: "example",
          passed: true,
          verdict: pass(),
        });
      } else {
        results.push({
          name: example.name,
          kind: "example",
          passed: false,
          verdict: fail(
            "example-mismatch",
            `expected ${JSON.stringify(expected, (_, v) => typeof v === "bigint" ? v.toString() : v)}, got ${JSON.stringify(actual, (_, v) => typeof v === "bigint" ? v.toString() : v)}`
          ),
        });
      }
    } catch (err) {
      results.push({
        name: example.name,
        kind: "example",
        passed: false,
        verdict: fail("example-panic", String(err)),
      });
    }
  }

  // 2. Execute universal claims (Level 2 Property-Based Tests)
  const samples = options.claimSamples ?? 50;

  for (const claim of module.claims) {
    try {
      const generators: Array<Gen<any>> = claim.binders.map((b) => {
        switch (b.type.name) {
          case "Bytes":
            return Gen.bytes(0, 32);
          case "Bool":
            return Gen.bool();
          case "Nat":
          case "U64":
          case "Int":
            return Gen.u64();
          case "U8":
            return Gen.u8().map((n) => BigInt(n));
          default:
            throw new Error(`Unsupported claim binder type: ${b.type.name}`);
        }
      });

      const tupleGen = new Gen((rng) => {
        return generators.map((g) => g.sample(rng));
      });

      let claimGen = tupleGen;
      if (claim.refinement) {
      // The attempt cap below (1000) is deliberate; rejection sampling needs
      // headroom. A refinement like `len(a) == len(b)` over two independent Bytes
      // (length 0..32) is accepted only ~1 time in 33, so the old cap of 100 ran
      // out (~5% per sample, which a fixed seed turns into a deterministic
      // "Generator filter exhausted" failure). 1000 makes that ~4e-14 and is
      // still a hard bound.
        claimGen = tupleGen.filter((tupleValues: any[]) => {
          const filterEnv = new Map<string, unknown>();
          for (let i = 0; i < claim.binders.length; i++) {
            filterEnv.set(claim.binders[i].name, tupleValues[i]);
          }
          const filterCtx: EvaluatorContext = {
            ...baseCtx,
            env: filterEnv,
            depth: 0,
            budget: { count: 0, limit: 10_000 },
          };
          return Boolean(evaluateWarrantExpr(claim.refinement!, filterCtx));
        }, 1000);
      }

      assertProperty(
        claimGen,
        (tupleValues: any[]) => {
          const env = new Map<string, unknown>();
          for (let i = 0; i < claim.binders.length; i++) {
            env.set(claim.binders[i].name, tupleValues[i]);
          }
          const claimCtx: EvaluatorContext = {
            ...baseCtx,
            env,
            depth: 0,
            budget: { count: 0, limit: 10_000 },
          };

          const actual = evaluateWarrantExpr(claim.lhs, claimCtx);
          const expected = evaluateWarrantExpr(claim.rhs, claimCtx);
          return areWarrantValuesEqual(actual, expected);
        },
        {
          numTests: samples,
          seed: options.seed ?? 42,
        }
      );

      results.push({
        name: claim.name,
        kind: "claim",
        passed: true,
        verdict: pass(),
      });
    } catch (err) {
      results.push({
        name: claim.name,
        kind: "claim",
        passed: false,
        verdict: fail("claim-counterexample", String(err)),
      });
    }
  }

  return results;
}

// --- Helpers ----------------------------------------------------------------

function decodeHex(hex: string): Uint8Array {
  if (hex.length % 2 !== 0) {
    throw new Error(`Invalid hex string length: ${hex.length}`);
  }
  if (!/^(?:[0-9a-f]{2})*$/.test(hex)) {
    throw new Error("Invalid hexadecimal characters in bytes literal");
  }
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}
