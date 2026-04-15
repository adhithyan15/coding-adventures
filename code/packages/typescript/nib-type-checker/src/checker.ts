import type { Token } from "@coding-adventures/lexer";
import type { ASTNode } from "@coding-adventures/parser";
import {
  GenericTypeChecker,
  type TypeCheckResult,
} from "@coding-adventures/type-checker-protocol";

import { ScopeChain, type SymbolRecord } from "./scope.js";
import {
  isBcdOpAllowed,
  isNumeric,
  NibType,
  parseTypeName,
  typesAreCompatible,
} from "./types.js";

type NodeChild = ASTNode | Token;
type TypedAstNode = ASTNode & { _nibType?: NibType };

const EXPRESSION_RULES = new Set([
  "expr",
  "or_expr",
  "and_expr",
  "eq_expr",
  "cmp_expr",
  "add_expr",
  "bitwise_expr",
  "unary_expr",
  "primary",
  "call_expr",
]);

function isAstNode(child: NodeChild): child is ASTNode {
  return "ruleName" in child;
}

function tokenType(child: NodeChild): string {
  return isAstNode(child) ? "" : child.type;
}

function tokenValue(child: NodeChild): string {
  return isAstNode(child) ? "" : child.value;
}

function firstToken(subject: ASTNode | Token): Token | null {
  if (!isAstNode(subject)) {
    return subject;
  }
  for (const child of subject.children) {
    const token = firstToken(child);
    if (token) {
      return token;
    }
  }
  return null;
}

function childNodes(node: ASTNode): ASTNode[] {
  return node.children.filter(isAstNode);
}

function expressionChildren(node: ASTNode): ASTNode[] {
  return childNodes(node).filter((child) => EXPRESSION_RULES.has(child.ruleName));
}

function isNumericLiteralExpr(node: ASTNode | Token): boolean {
  if (!isAstNode(node)) {
    return node.type === "INT_LIT" || node.type === "HEX_LIT";
  }

  if (node.children.length === 0) {
    return false;
  }

  let sawAstChild = false;
  for (const child of node.children) {
    if (isAstNode(child)) {
      sawAstChild = true;
      if (!isNumericLiteralExpr(child)) {
        return false;
      }
      continue;
    }

    if (
      child.type === "NAME" ||
      child.type === "true" ||
      child.type === "false" ||
      child.value === "true" ||
      child.value === "false"
    ) {
      return false;
    }

    if (
      child.type === "EQ_EQ" ||
      child.type === "NEQ" ||
      child.type === "LEQ" ||
      child.type === "GEQ" ||
      child.type === "LT" ||
      child.type === "GT" ||
      child.type === "LAND" ||
      child.type === "LOR"
    ) {
      return false;
    }
  }

  return sawAstChild;
}

export class NibTypeChecker extends GenericTypeChecker<ASTNode> {
  protected run(ast: ASTNode): void {
    const scope = new ScopeChain();
    this.checkProgram(ast, scope);
  }

  protected nodeKind(node: ASTNode): string | null {
    return node.ruleName;
  }

  protected override locate(subject: unknown): [number, number] {
    if (subject && typeof subject === "object") {
      const token = firstToken(subject as ASTNode | Token);
      if (token) {
        return [token.line, token.column];
      }
    }
    return [1, 1];
  }

  private checkProgram(node: ASTNode, scope: ScopeChain): void {
    const functionNodes: Array<[string, ASTNode]> = [];

    for (const child of childNodes(node)) {
      const decl = child.children[0];
      if (!decl || !isAstNode(decl)) {
        continue;
      }

      if (decl.ruleName === "const_decl") {
        this.collectConstOrStatic(decl, scope, true);
      } else if (decl.ruleName === "static_decl") {
        this.collectConstOrStatic(decl, scope, false);
      } else if (decl.ruleName === "fn_decl") {
        const fn = this.collectFunctionSignature(decl, scope);
        if (fn) {
          functionNodes.push(fn);
        }
      }
    }

    for (const [, fnNode] of functionNodes) {
      this.checkFunctionBody(fnNode, scope);
    }
  }

  private collectConstOrStatic(node: ASTNode, scope: ScopeChain, isConst: boolean): void {
    let nameToken: Token | null = null;
    let typeNode: ASTNode | null = null;
    let tokenIndex = 0;

    for (const child of node.children) {
      if (isAstNode(child)) {
        if (child.ruleName === "type") {
          typeNode = child;
          break;
        }
        continue;
      }

      if (tokenIndex === 1 && child.type === "NAME") {
        nameToken = child;
      }
      tokenIndex += 1;
    }

    if (!nameToken || !typeNode) {
      return;
    }

    const nibType = this.resolveTypeNode(typeNode);
    if (!nibType) {
      return;
    }

    scope.defineGlobal(nameToken.value, {
      name: nameToken.value,
      nibType,
      isConst,
      isStatic: !isConst,
    });
  }

  private collectFunctionSignature(node: ASTNode, scope: ScopeChain): [string, ASTNode] | null {
    let fnName: string | null = null;
    let params: Array<[string, NibType]> = [];
    let returnType: NibType | null = null;

    for (const child of node.children) {
      if (!isAstNode(child)) {
        if (child.type === "NAME" && fnName === null) {
          fnName = child.value;
        }
        continue;
      }

      if (child.ruleName === "param_list") {
        params = this.extractParams(child);
      } else if (child.ruleName === "type") {
        returnType = this.resolveTypeNode(child);
      }
    }

    if (!fnName) {
      return null;
    }

    scope.defineGlobal(fnName, {
      name: fnName,
      nibType: returnType,
      isFn: true,
      fnParams: params,
      fnReturnType: returnType,
    });

    return [fnName, node];
  }

  private extractParams(node: ASTNode): Array<[string, NibType]> {
    const params: Array<[string, NibType]> = [];

    for (const child of childNodes(node)) {
      if (child.ruleName !== "param") {
        continue;
      }

      let name: string | null = null;
      let typeNode: ASTNode | null = null;
      for (const paramChild of child.children) {
        if (isAstNode(paramChild)) {
          if (paramChild.ruleName === "type") {
            typeNode = paramChild;
          }
          continue;
        }

        if (paramChild.type === "NAME" && name === null) {
          name = paramChild.value;
        }
      }

      if (name && typeNode) {
        const resolved = this.resolveTypeNode(typeNode);
        if (resolved) {
          params.push([name, resolved]);
        }
      }
    }

    return params;
  }

  private checkFunctionBody(node: ASTNode, globalScope: ScopeChain): void {
    const fnSymbol = this.functionSymbolFor(node, globalScope);
    if (!fnSymbol) {
      return;
    }

    const functionScope = globalScope;
    functionScope.push();

    for (const [name, type] of fnSymbol.fnParams ?? []) {
      functionScope.define(name, { name, nibType: type });
    }

    for (const child of childNodes(node)) {
      if (child.ruleName === "block") {
        this.checkBlock(child, functionScope, fnSymbol.fnReturnType ?? null, false);
      }
    }

    functionScope.pop();
  }

  private functionSymbolFor(node: ASTNode, scope: ScopeChain): SymbolRecord | null {
    for (const child of node.children) {
      if (!isAstNode(child) && child.type === "NAME") {
        return scope.lookup(child.value);
      }
    }
    return null;
  }

  private checkBlock(
    node: ASTNode,
    scope: ScopeChain,
    expectedReturnType: NibType | null,
    createScope = true,
  ): void {
    if (createScope) {
      scope.push();
    }

    for (const child of childNodes(node)) {
      if (child.ruleName === "stmt") {
        const stmt = child.children[0];
        if (stmt && isAstNode(stmt)) {
          this.checkStatement(stmt, scope, expectedReturnType);
        }
      }
    }

    if (createScope) {
      scope.pop();
    }
  }

  private checkStatement(
    node: ASTNode,
    scope: ScopeChain,
    expectedReturnType: NibType | null,
  ): void {
    switch (node.ruleName) {
      case "let_stmt":
        this.checkLetStatement(node, scope);
        return;
      case "assign_stmt":
        this.checkAssignStatement(node, scope);
        return;
      case "return_stmt":
        this.checkReturnStatement(node, scope, expectedReturnType);
        return;
      case "for_stmt":
        this.checkForStatement(node, scope, expectedReturnType);
        return;
      case "if_stmt":
        this.checkIfStatement(node, scope, expectedReturnType);
        return;
      case "expr_stmt": {
        const expr = expressionChildren(node)[0];
        if (expr) {
          this.checkExpression(expr, scope);
        }
        return;
      }
      default:
        return;
    }
  }

  private checkLetStatement(node: ASTNode, scope: ScopeChain): void {
    let nameToken: Token | null = null;
    let typeNode: ASTNode | null = null;
    const expr = expressionChildren(node)[0] ?? null;

    for (const child of node.children) {
      if (isAstNode(child)) {
        if (child.ruleName === "type") {
          typeNode = child;
        }
        continue;
      }

      if (child.type === "NAME" && nameToken === null) {
        nameToken = child;
      }
    }

    if (!nameToken || !typeNode || !expr) {
      return;
    }

    const declaredType = this.resolveTypeNode(typeNode);
    const exprType = this.checkExpression(expr, scope);

    if (
      declaredType &&
      exprType &&
      !isNumericLiteralExpr(expr) &&
      !typesAreCompatible(declaredType, exprType)
    ) {
      this.error(
        `Cannot initialize '${nameToken.value}' of type '${declaredType}' with expression of type '${exprType}'.`,
        expr,
      );
    }

    if (declaredType) {
      scope.define(nameToken.value, { name: nameToken.value, nibType: declaredType });
    }
  }

  private checkAssignStatement(node: ASTNode, scope: ScopeChain): void {
    let nameToken: Token | null = null;
    const expr = expressionChildren(node)[0] ?? null;

    for (const child of node.children) {
      if (!isAstNode(child) && child.type === "NAME") {
        nameToken = child;
        break;
      }
    }

    if (!nameToken || !expr) {
      return;
    }

    const symbol = scope.lookup(nameToken.value);
    if (!symbol || !symbol.nibType) {
      this.error(`'${nameToken.value}' is not defined.`, nameToken);
      return;
    }

    const exprType = this.checkExpression(expr, scope);
    if (
      exprType &&
      !isNumericLiteralExpr(expr) &&
      !typesAreCompatible(symbol.nibType, exprType)
    ) {
      this.error(
        `Cannot assign expression of type '${exprType}' to '${nameToken.value}' of type '${symbol.nibType}'.`,
        expr,
      );
    }
  }

  private checkReturnStatement(
    node: ASTNode,
    scope: ScopeChain,
    expectedReturnType: NibType | null,
  ): void {
    const expr = expressionChildren(node)[0] ?? null;
    if (!expr) {
      return;
    }

    const exprType = this.checkExpression(expr, scope);
    if (expectedReturnType && exprType && !typesAreCompatible(expectedReturnType, exprType)) {
      this.error(
        `Return type mismatch: expected '${expectedReturnType}' but got '${exprType}'.`,
        expr,
      );
    }
  }

  private checkForStatement(
    node: ASTNode,
    scope: ScopeChain,
    expectedReturnType: NibType | null,
  ): void {
    let loopVar: Token | null = null;
    let loopTypeNode: ASTNode | null = null;
    let blockNode: ASTNode | null = null;
    const exprs = expressionChildren(node);

    for (const child of node.children) {
      if (isAstNode(child)) {
        if (child.ruleName === "type" && loopTypeNode === null) {
          loopTypeNode = child;
        } else if (child.ruleName === "block") {
          blockNode = child;
        }
        continue;
      }

      if (child.type === "NAME" && loopVar === null) {
        loopVar = child;
      }
    }

    for (const boundExpr of exprs.slice(0, 2)) {
      const boundType = this.checkExpression(boundExpr, scope);
      if (boundType && !isNumeric(boundType)) {
        this.error(`For-loop bounds must be numeric, but got '${boundType}'.`, boundExpr);
      }
    }

    if (!loopVar || !loopTypeNode || !blockNode) {
      return;
    }

    const loopType = this.resolveTypeNode(loopTypeNode);
    scope.push();
    if (loopType) {
      scope.define(loopVar.value, { name: loopVar.value, nibType: loopType });
    }
    this.checkBlock(blockNode, scope, expectedReturnType, false);
    scope.pop();
  }

  private checkIfStatement(
    node: ASTNode,
    scope: ScopeChain,
    expectedReturnType: NibType | null,
  ): void {
    const expr = expressionChildren(node)[0] ?? null;
    if (expr) {
      const exprType = this.checkExpression(expr, scope);
      if (exprType && exprType !== NibType.BOOL) {
        this.error(`The condition of 'if' must have type 'bool', but got '${exprType}'.`, expr);
      }
    }

    for (const child of childNodes(node)) {
      if (child.ruleName === "block") {
        this.checkBlock(child, scope, expectedReturnType);
      }
    }
  }

  private checkExpression(node: ASTNode | Token, scope: ScopeChain): NibType | null {
    if (!isAstNode(node)) {
      return this.checkTokenExpression(node, scope);
    }

    let result: NibType | null;
    switch (node.ruleName) {
      case "call_expr":
        result = this.checkCallExpression(node, scope);
        break;
      case "primary":
        result = this.checkPrimary(node, scope);
        break;
      case "add_expr":
        result = this.checkAddExpression(node, scope);
        break;
      case "or_expr":
      case "and_expr":
      case "eq_expr":
      case "cmp_expr":
      case "bitwise_expr":
      case "unary_expr":
      case "expr":
        result = this.checkCompoundExpression(node, scope);
        break;
      default:
        result =
          node.children.length === 1 ? this.checkExpression(node.children[0], scope) : null;
        break;
    }

    if (result) {
      (node as TypedAstNode)._nibType = result;
    }
    return result;
  }

  private checkTokenExpression(token: Token, scope: ScopeChain): NibType | null {
    if (token.type === "INT_LIT" || token.type === "HEX_LIT") {
      return NibType.U4;
    }

    if (token.type === "true" || token.type === "false" || token.value === "true" || token.value === "false") {
      return NibType.BOOL;
    }

    if (token.type !== "NAME") {
      return null;
    }

    const symbol = scope.lookup(token.value);
    if (!symbol || !symbol.nibType) {
      this.error(`'${token.value}' is not defined.`, token);
      return null;
    }

    if (symbol.isFn) {
      this.error(`'${token.value}' is a function. Use parentheses to call it.`, token);
      return null;
    }

    return symbol.nibType;
  }

  private checkCompoundExpression(node: ASTNode, scope: ScopeChain): NibType | null {
    if (node.children.length === 1) {
      return this.checkExpression(node.children[0], scope);
    }

    if (node.ruleName === "or_expr" || node.ruleName === "and_expr") {
      for (const expr of expressionChildren(node)) {
        const exprType = this.checkExpression(expr, scope);
        if (exprType && exprType !== NibType.BOOL) {
          this.error("Logical operators require bool operands.", expr);
        }
      }
      return NibType.BOOL;
    }

    if (node.ruleName === "eq_expr" || node.ruleName === "cmp_expr") {
      const exprTypes = expressionChildren(node)
        .map((expr) => this.checkExpression(expr, scope))
        .filter((value): value is NibType => value !== null);

      if (exprTypes.length >= 2 && exprTypes[0] !== exprTypes[1]) {
        this.error(
          `Comparison operands must have the same type. Got '${exprTypes[0]}' and '${exprTypes[1]}'.`,
          node,
        );
      }
      return NibType.BOOL;
    }

    if (node.ruleName === "bitwise_expr") {
      const exprTypes = expressionChildren(node)
        .map((expr) => this.checkExpression(expr, scope))
        .filter((value): value is NibType => value !== null);

      if (exprTypes.length >= 2 && exprTypes[0] !== exprTypes[1]) {
        this.error(
          `Bitwise operands must have the same type. Got '${exprTypes[0]}' and '${exprTypes[1]}'.`,
          node,
        );
      }
      return exprTypes[0] ?? null;
    }

    if (node.ruleName === "unary_expr" && node.children.length >= 2) {
      const operator = node.children[0];
      const operand = node.children[1];
      const operandType = this.checkExpression(operand, scope);

      if (!isAstNode(operator) && operator.value === "!") {
        if (operandType && operandType !== NibType.BOOL) {
          this.error(`Logical NOT requires a bool operand, but got '${operandType}'.`, operand);
        }
        return NibType.BOOL;
      }

      return operandType;
    }

    const expr = expressionChildren(node)[0];
    return expr ? this.checkExpression(expr, scope) : null;
  }

  private checkAddExpression(node: ASTNode, scope: ScopeChain): NibType | null {
    if (node.children.length === 1) {
      return this.checkExpression(node.children[0], scope);
    }

    const operandTypes = expressionChildren(node).map((expr) => this.checkExpression(expr, scope));
    const resolved = operandTypes.filter((value): value is NibType => value !== null);
    const hasBcd = resolved.includes(NibType.BCD);
    let resultType = resolved[0] ?? null;

    for (let index = 1; index < node.children.length; index += 2) {
      const operator = node.children[index];
      const left = operandTypes[Math.floor(index / 2)];
      const right = operandTypes[Math.floor(index / 2) + 1];

      if (isAstNode(operator)) {
        continue;
      }

      if (hasBcd && !isBcdOpAllowed(operator.value)) {
        this.error(
          `BCD operands only support '+%' and '-', but got '${operator.value}'.`,
          operator,
        );
      }

      if (left && right && left !== right) {
        this.error(
          `Operands of '${operator.value}' must have the same type. Got '${left}' and '${right}'.`,
          operator,
        );
      }

      resultType = left ?? right ?? resultType;
    }

    return resultType;
  }

  private checkPrimary(node: ASTNode, scope: ScopeChain): NibType | null {
    if (node.children.length === 0) {
      return null;
    }

    const first = node.children[0];
    if (!isAstNode(first)) {
      return this.checkTokenExpression(first, scope);
    }

    if (first.ruleName === "call_expr") {
      return this.checkCallExpression(first, scope);
    }

    return this.checkExpression(first, scope);
  }

  private checkCallExpression(node: ASTNode, scope: ScopeChain): NibType | null {
    let fnName: string | null = null;
    const args: ASTNode[] = [];

    for (const child of node.children) {
      if (!isAstNode(child)) {
        if (child.type === "NAME" && fnName === null) {
          fnName = child.value;
        }
        continue;
      }

      if (child.ruleName === "arg_list") {
        args.push(...expressionChildren(child));
      }
    }

    if (!fnName) {
      return null;
    }

    const symbol = scope.lookup(fnName);
    if (!symbol || !symbol.isFn) {
      this.error(`Function '${fnName}' is not defined.`, node);
      return null;
    }

    if (args.length !== (symbol.fnParams?.length ?? 0)) {
      this.error(
        `Function '${fnName}' expects ${symbol.fnParams?.length ?? 0} argument(s) but got ${args.length}.`,
        node,
      );
    }

    for (let index = 0; index < args.length; index += 1) {
      const argType = this.checkExpression(args[index], scope);
      const param = symbol.fnParams?.[index];
      if (argType && param && !typesAreCompatible(param[1], argType)) {
        this.error(
          `Argument ${index + 1} to '${fnName}' expected '${param[1]}' but got '${argType}'.`,
          args[index],
        );
      }
    }

    return symbol.fnReturnType ?? null;
  }

  private resolveTypeNode(node: ASTNode): NibType | null {
    for (const child of node.children) {
      if (!isAstNode(child)) {
        const parsed = parseTypeName(child.value);
        if (!parsed) {
          this.error(`Unknown type '${child.value}'.`, child);
        }
        return parsed;
      }
    }
    return null;
  }
}

export function checkNib(ast: ASTNode): TypeCheckResult<ASTNode> {
  return new NibTypeChecker().check(ast);
}
