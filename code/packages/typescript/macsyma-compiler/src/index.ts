import type { Token } from "@coding-adventures/lexer";
import { parseMacsyma } from "@coding-adventures/macsyma-parser";
import { isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import {
  ACOS,
  ACOSH,
  ADD,
  AND,
  ASIN,
  ASINH,
  ASSIGN,
  ATAN,
  ATANH,
  BLOCK,
  COS,
  COSH,
  D,
  DEFINE,
  DIV,
  EQUAL,
  EXP,
  FACTOR,
  FALSE,
  FOR_EACH,
  FOR_RANGE,
  FORGET,
  GREATER,
  GREATER_EQUAL,
  IF,
  INTEGRATE,
  IS,
  LESS,
  LESS_EQUAL,
  LIST,
  LOG,
  MUL,
  NEG,
  NOT,
  NOT_EQUAL,
  OR,
  POW,
  PRODUCT,
  RETURN,
  SIGN,
  SIMPLIFY,
  SIN,
  SINH,
  SOLVE,
  SQRT,
  SUB,
  SUBST,
  SUM,
  TAN,
  TANH,
  TRUE,
  WHILE,
  app,
  int,
  numberNode,
  stringNode,
  sym,
} from "@coding-adventures/symbolic-ir";
import type { IRApply, IRNode, IRSymbol } from "@coding-adventures/symbolic-ir";

export class CompileError extends Error {}

export const DISPLAY = sym("Display");
export const SUPPRESS = sym("Suppress");

const BINARY_OPS = new Map<string, IRSymbol>([
  ["PLUS", ADD],
  ["MINUS", SUB],
  ["STAR", MUL],
  ["SLASH", DIV],
]);

const COMPARISON_OPS = new Map<string, IRSymbol>([
  ["EQ", EQUAL],
  ["HASH", NOT_EQUAL],
  ["LT", LESS],
  ["GT", GREATER],
  ["LEQ", LESS_EQUAL],
  ["GEQ", GREATER_EQUAL],
]);

const STANDARD_FUNCTIONS = new Map<string, IRSymbol>([
  ["diff", D],
  ["integrate", INTEGRATE],
  ["sin", SIN],
  ["cos", COS],
  ["tan", TAN],
  ["asin", ASIN],
  ["acos", ACOS],
  ["atan", ATAN],
  ["sinh", SINH],
  ["cosh", COSH],
  ["tanh", TANH],
  ["asinh", ASINH],
  ["acosh", ACOSH],
  ["atanh", ATANH],
  ["coth", sym("Coth")],
  ["sech", sym("Sech")],
  ["csch", sym("Csch")],
  ["log", LOG],
  ["exp", EXP],
  ["sqrt", SQRT],
  ["sum", SUM],
  ["product", PRODUCT],
  ["factor", FACTOR],
  ["solve", SOLVE],
  ["simplify", SIMPLIFY],
  ["subst", SUBST],
  ["assume", sym("Assume")],
  ["forget", FORGET],
  ["is", IS],
  ["sign", SIGN],
]);

export interface CompileOptions {
  readonly wrapTerminators?: boolean;
}

export function compileMacsyma(input: string | ASTNode, options: CompileOptions = {}): IRNode[] {
  const ast = typeof input === "string" ? parseMacsyma(input) : input;
  return new Compiler(options).compileProgram(ast);
}

export class Compiler {
  constructor(private readonly options: CompileOptions = {}) {}

  compileProgram(root: ASTNode): IRNode[] {
    if (root.ruleName !== "program") {
      throw new CompileError(`expected program root, got ${root.ruleName}`);
    }
    return root.children
      .filter(isASTNode)
      .filter((child) => child.ruleName === "statement")
      .map((statement) => this.compileStatement(statement));
  }

  private compileStatement(node: ASTNode): IRNode {
    const exprNode = node.children.find(isASTNode);
    if (exprNode === undefined) throw new CompileError("statement has no expression");
    const inner = this.compileNode(exprNode);
    if (this.options.wrapTerminators !== true) return inner;

    const terminator = node.children.find(isTerminatorToken);
    return app(terminator?.type === "DOLLAR" ? SUPPRESS : DISPLAY, [inner]);
  }

  private compileNode(input: ASTNode | Token): IRNode {
    if (isToken(input)) return this.compileToken(input);
    const node = unwrap(input);
    if (isToken(node)) return this.compileToken(node);

    switch (node.ruleName) {
      case "statement":
        return this.compileStatement(node);
      case "expression":
        return this.compileFirstNode(node);
      case "assign":
        return this.compileAssign(node);
      case "logical_or":
        return this.compileLogicalChain(node, OR);
      case "logical_and":
        return this.compileLogicalChain(node, AND);
      case "logical_not":
        return this.compileLogicalNot(node);
      case "comparison":
        return this.compileComparison(node);
      case "additive":
      case "multiplicative":
        return this.compileBinaryChain(node);
      case "unary":
        return this.compileUnary(node);
      case "power":
        return this.compilePower(node);
      case "postfix":
        return this.compilePostfix(node);
      case "atom":
        return this.compileFirst(node);
      case "group":
        return this.compileDelimitedSingle(node);
      case "list":
        return this.compileList(node);
      case "arglist":
        throw new CompileError("arglist cannot be compiled as a scalar expression");
      case "if_expr":
        return this.compileIf(node);
      case "while_expr":
        return this.compileWhile(node);
      case "for_expr":
        return this.compileFirstNode(node);
      case "for_each_expr":
        return this.compileForEach(node);
      case "for_range_expr":
        return this.compileForRange(node);
      case "block_expr":
        return this.compileBlock(node);
      case "return_expr":
        return this.compileReturn(node);
      default:
        throw new CompileError(`no compiler for rule ${node.ruleName}`);
    }
  }

  private compileToken(token: Token): IRNode {
    if (token.type === "NUMBER") {
      return /[.eE]/.test(token.value) ? numberNode(Number(token.value)) : int(token.value);
    }
    if (token.type === "NAME") {
      return sym(token.value);
    }
    if (token.type === "STRING") {
      return stringNode(token.value);
    }
    if (token.type === "KEYWORD") {
      if (token.value === "true") return TRUE;
      if (token.value === "false") return FALSE;
    }
    throw new CompileError(`unexpected token ${token.type}=${JSON.stringify(token.value)}`);
  }

  private compileFirst(node: ASTNode): IRNode {
    if (node.children.length === 0) throw new CompileError(`${node.ruleName} has no children`);
    return this.compileNode(node.children[0] as ASTNode | Token);
  }

  private compileFirstNode(node: ASTNode): IRNode {
    const child = node.children.find(isASTNode);
    if (child === undefined) throw new CompileError(`${node.ruleName} has no AST child`);
    return this.compileNode(child);
  }

  private compileDelimitedSingle(node: ASTNode): IRNode {
    const child = node.children.find(isASTNode);
    if (child === undefined) throw new CompileError(`${node.ruleName} has no inner expression`);
    return this.compileNode(child);
  }

  private compileAssign(node: ASTNode): IRNode {
    const children = meaningful(node);
    const opIndex = children.findIndex((child) => isToken(child) && (child.type === "COLON" || child.type === "COLONEQ"));
    if (opIndex < 0) return this.compileFirstNode(node);

    const lhs = this.compileNode(children[opIndex - 1] as ASTNode | Token);
    const rhs = this.compileNode(children[opIndex + 1] as ASTNode | Token);
    const op = children[opIndex] as Token;
    if (op.type === "COLONEQ") {
      if (lhs.kind === "apply" && lhs.head.kind === "symbol") {
        return app(DEFINE, [lhs.head, app(LIST, lhs.args), rhs]);
      }
      return app(DEFINE, [lhs, app(LIST, []), rhs]);
    }
    return app(ASSIGN, [lhs, rhs]);
  }

  private compileLogicalChain(node: ASTNode, head: IRSymbol): IRNode {
    const operands = node.children.filter(isASTNode).map((child) => this.compileNode(child));
    return operands.length === 1 ? operands[0] : app(head, operands);
  }

  private compileLogicalNot(node: ASTNode): IRNode {
    const keyword = node.children.find((child) => isToken(child) && child.value === "not");
    if (keyword === undefined) return this.compileFirstNode(node);
    const child = node.children.find(isASTNode);
    if (child === undefined) throw new CompileError("not expression missing operand");
    return app(NOT, [this.compileNode(child)]);
  }

  private compileComparison(node: ASTNode): IRNode {
    const children = meaningful(node);
    const opIndex = children.findIndex((child) => isToken(child) && COMPARISON_OPS.has(child.type));
    if (opIndex < 0) return this.compileFirstNode(node);
    const head = COMPARISON_OPS.get((children[opIndex] as Token).type);
    if (head === undefined) throw new CompileError("unknown comparison op");
    return app(head, [
      this.compileNode(children[opIndex - 1] as ASTNode | Token),
      this.compileNode(children[opIndex + 1] as ASTNode | Token),
    ]);
  }

  private compileBinaryChain(node: ASTNode): IRNode {
    const children = meaningful(node);
    let result = this.compileNode(children[0] as ASTNode | Token);
    for (let i = 1; i < children.length; i += 2) {
      const op = children[i] as Token;
      const head = BINARY_OPS.get(op.type);
      if (head === undefined) throw new CompileError(`unknown binary op ${op.type}`);
      result = app(head, [result, this.compileNode(children[i + 1] as ASTNode | Token)]);
    }
    return result;
  }

  private compileUnary(node: ASTNode): IRNode {
    const children = meaningful(node);
    if (children.length === 1) return this.compileNode(children[0] as ASTNode | Token);
    const op = children[0] as Token;
    const value = this.compileNode(children[1] as ASTNode | Token);
    return op.type === "MINUS" ? app(NEG, [value]) : value;
  }

  private compilePower(node: ASTNode): IRNode {
    const children = meaningful(node);
    if (children.length === 1) return this.compileNode(children[0] as ASTNode | Token);
    return app(POW, [
      this.compileNode(children[0] as ASTNode | Token),
      this.compileNode(children[2] as ASTNode | Token),
    ]);
  }

  private compilePostfix(node: ASTNode): IRNode {
    const children = meaningful(node);
    let result = this.compileNode(children[0] as ASTNode | Token);
    for (let i = 1; i < children.length; i += 1) {
      const child = children[i];
      if (!isToken(child) || child.type !== "LPAREN") continue;
      const argsNode = children[i + 1];
      const args = isASTNode(argsNode) && argsNode.ruleName === "arglist"
        ? this.compileArglist(argsNode)
        : [];
      result = app(canonicalCallHead(result), args);
    }
    return result;
  }

  private compileList(node: ASTNode): IRNode {
    const args = node.children
      .filter(isASTNode)
      .filter((child) => child.ruleName === "arglist")
      .flatMap((child) => this.compileArglist(child));
    return app(LIST, args);
  }

  private compileArglist(node: ASTNode): IRNode[] {
    return node.children.filter(isASTNode).map((child) => this.compileNode(child));
  }

  private compileIf(node: ASTNode): IRNode {
    const expressions = node.children.filter(isASTNode).map((child) => this.compileNode(child));
    if (expressions.length < 2) throw new CompileError("if expression needs condition and then branch");
    let fallback: IRNode = expressions.length % 2 === 1 ? expressions[expressions.length - 1] : FALSE;
    const pairLimit = expressions.length % 2 === 1 ? expressions.length - 1 : expressions.length;
    for (let i = pairLimit - 2; i >= 0; i -= 2) {
      fallback = app(IF, [expressions[i], expressions[i + 1], fallback]);
    }
    return fallback;
  }

  private compileWhile(node: ASTNode): IRNode {
    const expressions = node.children.filter(isASTNode).map((child) => this.compileNode(child));
    if (expressions.length !== 2) throw new CompileError("while expression needs condition and body");
    return app(WHILE, expressions);
  }

  private compileForEach(node: ASTNode): IRNode {
    const variable = node.children.find(isNameToken);
    const expressions = node.children.filter(isASTNode).map((child) => this.compileNode(child));
    if (variable === undefined || expressions.length !== 2) throw new CompileError("for-each expression malformed");
    return app(FOR_EACH, [sym(variable.value), expressions[0], expressions[1]]);
  }

  private compileForRange(node: ASTNode): IRNode {
    const variable = node.children.find(isNameToken);
    const expressions = node.children.filter(isASTNode).map((child) => this.compileNode(child));
    if (variable === undefined || expressions.length < 2) throw new CompileError("for-range expression malformed");
    let start: IRNode = int(1);
    let step: IRNode = int(1);
    let end: IRNode;
    let body: IRNode;
    if (expressions.length === 2) {
      [end, body] = expressions;
    } else if (expressions.length === 3) {
      [start, end, body] = expressions;
    } else {
      [start, step, end, body] = expressions;
    }
    return app(FOR_RANGE, [sym(variable.value), start, step, end, body]);
  }

  private compileBlock(node: ASTNode): IRNode {
    const argsNode = node.children.find(isArglistNode);
    if (argsNode === undefined) return app(BLOCK, [app(LIST, [])]);
    const args = this.compileArglist(argsNode);
    if (args[0]?.kind === "apply" && isSameSymbol(args[0].head, LIST)) {
      return app(BLOCK, args);
    }
    return app(BLOCK, [app(LIST, []), ...args]);
  }

  private compileReturn(node: ASTNode): IRNode {
    const child = node.children.find(isASTNode);
    if (child === undefined) throw new CompileError("return expression missing value");
    return app(RETURN, [this.compileNode(child)]);
  }
}

function unwrap(node: ASTNode): ASTNode | Token {
  let current: ASTNode | Token = node;
  while (isASTNode(current) && current.children.length === 1) {
    current = current.children[0] as ASTNode | Token;
  }
  return current;
}

function meaningful(node: ASTNode): Array<ASTNode | Token> {
  return node.children as Array<ASTNode | Token>;
}

function isToken(child: unknown): child is Token {
  return typeof child === "object" && child !== null && "type" in child && "value" in child && !("ruleName" in child);
}

function isTerminatorToken(child: ASTNode | Token): child is Token {
  return isToken(child) && (child.type === "SEMI" || child.type === "DOLLAR");
}

function isNameToken(child: ASTNode | Token): child is Token {
  return isToken(child) && child.type === "NAME";
}

function isArglistNode(child: ASTNode | Token): child is ASTNode {
  return isASTNode(child) && child.ruleName === "arglist";
}

function canonicalCallHead(head: IRNode): IRNode {
  if (head.kind === "symbol") {
    return STANDARD_FUNCTIONS.get(head.name) ?? head;
  }
  return head;
}

function isSameSymbol(node: IRNode, symbol: IRSymbol): boolean {
  return node.kind === "symbol" && node.name === symbol.name;
}
