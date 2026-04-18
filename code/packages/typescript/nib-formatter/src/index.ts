/**
 * @coding-adventures/nib-formatter
 *
 * This package turns the generic Nib parser AST into `Doc` values, then uses
 * the shared layout and paint stack to produce canonical formatted source.
 *
 * The interesting constraint here is that `nib-parser` returns grammar-driven
 * `ASTNode` trees, not hand-shaped declaration and expression classes. That is
 * exactly the sort of language wrapper this formatter architecture is meant to
 * support: a small amount of AST extraction logic on top of a mostly-shared
 * document algebra toolkit.
 */

import {
  concat,
  group,
  hardline,
  indent,
  nil,
  text,
  type Doc,
  type LayoutOptions,
} from "@coding-adventures/format-doc";
import { docToPaintScene, type DocPaintOptions } from "@coding-adventures/format-doc-to-paint";
import { callLike, delimitedList, infixChain } from "@coding-adventures/format-doc-std";
import { parseNib } from "@coding-adventures/nib-parser";
import { renderToAscii } from "@coding-adventures/paint-vm-ascii";
import type { ASTNode } from "@coding-adventures/parser";

/** Package version, mirrored in tests as a smoke check. */
export const VERSION = "0.1.0";

/** The Nib expression rules that represent actual expressions in the grammar AST. */
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

/**
 * Public formatting options.
 *
 * The layout options belong to the `Doc -> LayoutTree` phase. Paint options are
 * carried through to the `LayoutTree -> PaintScene` bridge even though the
 * ASCII backend is the only renderer used today.
 */
export interface NibFormatOptions extends LayoutOptions {
  paint?: DocPaintOptions;
}

/** A minimal structural token shape as it appears inside the grammar AST. */
interface TokenLike {
  readonly type: string;
  readonly value: string;
}

type NodeChild = ASTNode | TokenLike;

const DEFAULT_FORMAT_OPTIONS: NibFormatOptions = {
  printWidth: 80,
  indentWidth: 2,
  lineHeight: 1,
};

/**
 * Lower a parsed Nib program into a backend-neutral `Doc`.
 *
 * This expects the root `program` AST from `nib-parser`.
 */
export function printNibDoc(ast: ASTNode): Doc {
  if (ast.ruleName !== "program") {
    throw new Error(`printNibDoc() expects a 'program' AST node, got '${ast.ruleName}'`);
  }

  const declarations = childNodes(ast).map((decl) => printTopLevel(decl));
  return joinWithBlankLines(declarations);
}

/** Parse Nib source and lower it to `Doc` in one step. */
export function printNibSourceToDoc(source: string): Doc {
  return printNibDoc(parseNib(source));
}

/** Run the full formatter pipeline on a parsed Nib AST. */
export function formatNibAst(ast: ASTNode, options: NibFormatOptions): string {
  const resolved = resolveOptions(options);
  const scene = docToPaintScene(
    printNibDoc(ast),
    {
      printWidth: resolved.printWidth,
      indentWidth: resolved.indentWidth,
      lineHeight: resolved.lineHeight,
    },
    resolved.paint,
  );

  // `DocLayoutTree` coordinates are already in monospace cell units, so the
  // ASCII backend must use unit scaling to keep one glyph per column.
  return renderToAscii(scene, { scaleX: 1, scaleY: 1 });
}

/** Parse and format Nib source into canonical ASCII text. */
export function formatNib(
  source: string,
  options: Partial<NibFormatOptions> = {},
): string {
  return formatNibAst(parseNib(source), resolveOptions(options));
}

function resolveOptions(options: Partial<NibFormatOptions>): NibFormatOptions {
  return {
    printWidth: options.printWidth ?? DEFAULT_FORMAT_OPTIONS.printWidth,
    indentWidth: options.indentWidth ?? DEFAULT_FORMAT_OPTIONS.indentWidth,
    lineHeight: options.lineHeight ?? DEFAULT_FORMAT_OPTIONS.lineHeight,
    paint: options.paint,
  };
}

function isAstNode(child: NodeChild): child is ASTNode {
  return "ruleName" in child;
}

function childNodes(node: ASTNode): ASTNode[] {
  return node.children.filter(isAstNode);
}

function childTokens(node: ASTNode): TokenLike[] {
  const tokens: TokenLike[] = [];

  for (const child of node.children) {
    if (!isAstNode(child)) {
      tokens.push(child);
    }
  }

  return tokens;
}

function firstToken(node: ASTNode, tokenType?: string): TokenLike | null {
  for (const child of node.children) {
    if (isAstNode(child)) {
      continue;
    }
    if (tokenType === undefined || child.type === tokenType) {
      return child;
    }
  }
  return null;
}

function childRule(node: ASTNode, ruleName: string): ASTNode | null {
  return childNodes(node).find((child) => child.ruleName === ruleName) ?? null;
}

function childRules(node: ASTNode, ruleName: string): ASTNode[] {
  return childNodes(node).filter((child) => child.ruleName === ruleName);
}

function expressionChildren(node: ASTNode): ASTNode[] {
  return childNodes(node).filter((child) => EXPRESSION_RULES.has(child.ruleName));
}

function unwrapSingleChild(node: ASTNode): ASTNode {
  const inner = childNodes(node)[0];
  if (!inner) {
    throw new Error(`Expected '${node.ruleName}' to wrap exactly one AST node`);
  }
  return inner;
}

function requireToken(node: ASTNode, tokenType: string, context: string): TokenLike {
  const token = firstToken(node, tokenType);
  if (!token) {
    throw new Error(`Expected token '${tokenType}' while formatting ${context}`);
  }
  return token;
}

function printTopLevel(node: ASTNode): Doc {
  const declaration = unwrapSingleChild(node);
  switch (declaration.ruleName) {
    case "const_decl":
      return printConstOrStatic(declaration, "const");
    case "static_decl":
      return printConstOrStatic(declaration, "static");
    case "fn_decl":
      return printFunctionDeclaration(declaration);
    default:
      throw new Error(`Unsupported top-level Nib rule '${declaration.ruleName}'`);
  }
}

function printConstOrStatic(node: ASTNode, keyword: "const" | "static"): Doc {
  const name = requireToken(node, "NAME", node.ruleName).value;
  const typeNode = childRule(node, "type");
  const exprNode = expressionChildren(node)[0];

  if (!typeNode || !exprNode) {
    throw new Error(`Malformed ${node.ruleName}: expected a type and initializer`);
  }

  return group(
    concat([
      text(`${keyword} ${name}: `),
      printType(typeNode),
      text(" = "),
      printExpression(exprNode),
      text(";"),
    ]),
  );
}

function printFunctionDeclaration(node: ASTNode): Doc {
  const name = requireToken(node, "NAME", "fn_decl").value;
  const params = childRule(node, "param_list");
  const block = childRule(node, "block");
  const returnType = childRules(node, "type")[0] ?? null;

  if (!block) {
    throw new Error("Malformed fn_decl: expected a block body");
  }

  const signature = concat([
    text("fn "),
    text(name),
    printParameterList(params),
    returnType ? concat([text(" -> "), printType(returnType)]) : nil(),
  ]);

  return group(concat([signature, text(" "), printBlock(block)]));
}

function printParameterList(node: ASTNode | null): Doc {
  const params = node ? childRules(node, "param").map((param) => printParam(param)) : [];
  return delimitedList({
    open: text("("),
    close: text(")"),
    items: params,
    trailingSeparator: "never",
  });
}

function printParam(node: ASTNode): Doc {
  const name = requireToken(node, "NAME", "param").value;
  const typeNode = childRule(node, "type");

  if (!typeNode) {
    throw new Error("Malformed param: expected a type annotation");
  }

  return concat([text(name), text(": "), printType(typeNode)]);
}

function printBlock(node: ASTNode): Doc {
  const statements = childRules(node, "stmt").map((stmt) => printStatement(unwrapSingleChild(stmt)));

  if (statements.length === 0) {
    return concat([text("{"), text(" "), text("}")]);
  }

  return concat([
    text("{"),
    indent(concat([hardline(), joinWithHardlines(statements)])),
    hardline(),
    text("}"),
  ]);
}

function printStatement(node: ASTNode): Doc {
  switch (node.ruleName) {
    case "let_stmt":
      return printLetStatement(node);
    case "assign_stmt":
      return printAssignStatement(node);
    case "return_stmt":
      return printReturnStatement(node);
    case "for_stmt":
      return printForStatement(node);
    case "if_stmt":
      return printIfStatement(node);
    case "expr_stmt":
      return printExpressionStatement(node);
    default:
      throw new Error(`Unsupported Nib statement rule '${node.ruleName}'`);
  }
}

function printLetStatement(node: ASTNode): Doc {
  const name = requireToken(node, "NAME", "let_stmt").value;
  const typeNode = childRule(node, "type");
  const exprNode = expressionChildren(node)[0];

  if (!typeNode || !exprNode) {
    throw new Error("Malformed let_stmt: expected type and initializer");
  }

  return group(
    concat([
      text("let "),
      text(name),
      text(": "),
      printType(typeNode),
      text(" = "),
      printExpression(exprNode),
      text(";"),
    ]),
  );
}

function printAssignStatement(node: ASTNode): Doc {
  const name = requireToken(node, "NAME", "assign_stmt").value;
  const exprNode = expressionChildren(node)[0];

  if (!exprNode) {
    throw new Error("Malformed assign_stmt: expected expression");
  }

  return group(
    concat([
      text(name),
      text(" = "),
      printExpression(exprNode),
      text(";"),
    ]),
  );
}

function printReturnStatement(node: ASTNode): Doc {
  const exprNode = expressionChildren(node)[0];
  if (!exprNode) {
    throw new Error("Malformed return_stmt: expected expression");
  }

  return group(concat([text("return "), printExpression(exprNode), text(";")]));
}

function printExpressionStatement(node: ASTNode): Doc {
  const exprNode = expressionChildren(node)[0];
  if (!exprNode) {
    throw new Error("Malformed expr_stmt: expected expression");
  }

  return group(concat([printExpression(exprNode), text(";")]));
}

function printForStatement(node: ASTNode): Doc {
  const loopVar = requireToken(node, "NAME", "for_stmt").value;
  const typeNode = childRule(node, "type");
  const blockNode = childRule(node, "block");
  const bounds = expressionChildren(node);

  if (!typeNode || !blockNode || bounds.length < 2) {
    throw new Error("Malformed for_stmt: expected loop type, bounds, and block");
  }

  return group(
    concat([
      text("for "),
      text(loopVar),
      text(": "),
      printType(typeNode),
      text(" in "),
      printExpression(bounds[0]!),
      text(".."),
      printExpression(bounds[1]!),
      text(" "),
      printBlock(blockNode),
    ]),
  );
}

function printIfStatement(node: ASTNode): Doc {
  const condition = expressionChildren(node)[0];
  const blocks = childRules(node, "block");
  const thenBlock = blocks[0];
  const elseBlock = blocks[1] ?? null;

  if (!condition || !thenBlock) {
    throw new Error("Malformed if_stmt: expected condition and then-block");
  }

  return group(
    concat([
      text("if "),
      printExpression(condition),
      text(" "),
      printBlock(thenBlock),
      elseBlock ? concat([text(" else "), printBlock(elseBlock)]) : nil(),
    ]),
  );
}

function printType(node: ASTNode): Doc {
  const token = childTokens(node)[0];
  if (!token) {
    throw new Error("Malformed type node: expected a token value");
  }
  return text(token.value);
}

function printExpression(node: ASTNode): Doc {
  switch (node.ruleName) {
    case "expr":
      return printExpression(unwrapSingleChild(node));
    case "or_expr":
    case "and_expr":
    case "eq_expr":
    case "cmp_expr":
    case "add_expr":
    case "bitwise_expr":
      return printInfixExpression(node);
    case "unary_expr":
      return printUnaryExpression(node);
    case "primary":
      return printPrimary(node);
    case "call_expr":
      return printCallExpression(node);
    default:
      throw new Error(`Unsupported Nib expression rule '${node.ruleName}'`);
  }
}

function printInfixExpression(node: ASTNode): Doc {
  const operands = expressionChildren(node);
  if (operands.length === 0) {
    throw new Error(`Malformed ${node.ruleName}: expected at least one operand`);
  }

  if (operands.length === 1) {
    return printExpression(operands[0]!);
  }

  const operators = childTokens(node).map((token) => text(token.value));
  if (operands.length === 2) {
    return group(
      concat([
        printExpression(operands[0]!),
        text(" "),
        operators[0]!,
        text(" "),
        printExpression(operands[1]!),
      ]),
    );
  }

  return infixChain({
    operands: operands.map((operand) => printExpression(operand)),
    operators,
  });
}

function printUnaryExpression(node: ASTNode): Doc {
  if (node.children.length === 1) {
    const operand = node.children[0];
    if (!operand || !isAstNode(operand)) {
      throw new Error("Malformed unary_expr: expected an AST child");
    }
    return printExpression(operand);
  }

  const operator = childTokens(node)[0];
  const operand = expressionChildren(node)[0];
  if (!operator || !operand) {
    throw new Error("Malformed unary_expr: expected operator and operand");
  }

  return concat([text(operator.value), printExpression(operand)]);
}

function printPrimary(node: ASTNode): Doc {
  const first = node.children[0];
  if (!first) {
    throw new Error("Malformed primary: expected at least one child");
  }

  if (isAstNode(first)) {
    return printExpression(first);
  }

  if (first.value === "(") {
    const inner = expressionChildren(node)[0];
    if (!inner) {
      throw new Error("Malformed parenthesized primary: expected inner expression");
    }
    return concat([text("("), printExpression(inner), text(")")]);
  }

  return text(first.value);
}

function printCallExpression(node: ASTNode): Doc {
  const callee = requireToken(node, "NAME", "call_expr").value;
  const argList = childRule(node, "arg_list");
  const args = argList ? expressionChildren(argList).map((arg) => printExpression(arg)) : [];

  return callLike(text(callee), args);
}

function joinWithHardlines(parts: readonly Doc[]): Doc {
  const out: Doc[] = [];
  for (let index = 0; index < parts.length; index += 1) {
    if (index > 0) {
      out.push(hardline());
    }
    out.push(parts[index]!);
  }
  return concat(out);
}

function joinWithBlankLines(parts: readonly Doc[]): Doc {
  if (parts.length === 0) {
    return nil();
  }

  const out: Doc[] = [];
  for (let index = 0; index < parts.length; index += 1) {
    if (index > 0) {
      out.push(hardline(), hardline());
    }
    out.push(parts[index]!);
  }
  return concat(out);
}
