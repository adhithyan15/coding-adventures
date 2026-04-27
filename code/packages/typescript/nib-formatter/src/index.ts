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
 *
 * This revision adds the first real trivia-preservation layer on top of that
 * stack. The lexer/parser now keep line comments and blank-line information
 * behind an opt-in flag, and this formatter interprets that trivia in a small
 * number of places:
 *
 * - anchored sequences such as top-level declarations and block statements
 * - boundaries such as `if ... else` and `fn ... {`
 * - the synthetic EOF token, which is the only place true end-of-file comments
 *   live after lexing
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
import type { Token, Trivia } from "@coding-adventures/lexer";
import { parseNib, parseNibDocument } from "@coding-adventures/nib-parser";
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

interface PrintNibDocOptions {
  readonly eofTrivia?: readonly Trivia[];
}

interface TriviaCommentSegment {
  readonly text: string;
  readonly newlinesBefore: number;
}

interface TriviaDisposition {
  readonly inlineTrailingComment: string | null;
  readonly leadingComments: readonly TriviaCommentSegment[];
  readonly trailingNewlinesBeforeAnchor: number;
}

interface SequenceEntry {
  doc: Doc;
  readonly blankLinesBefore: number;
  readonly attachable: boolean;
}

interface SequenceItem {
  readonly anchor: ASTNode;
  readonly doc: Doc;
}

type NodeChild = ASTNode | Token;

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
  return printNibDocWithOptions(ast, {});
}

/** Parse Nib source and lower it to `Doc` in one step. */
export function printNibSourceToDoc(source: string): Doc {
  const document = parseNibDocument(source, {
    preserveSourceInfo: true,
  });

  return printNibDocWithOptions(document.ast, {
    eofTrivia: getEofTrivia(document.tokens),
  });
}

/** Run the full formatter pipeline on a parsed Nib AST. */
export function formatNibAst(ast: ASTNode, options: NibFormatOptions): string {
  return renderDocToAscii(printNibDoc(ast), options);
}

/** Parse and format Nib source into canonical ASCII text. */
export function formatNib(
  source: string,
  options: Partial<NibFormatOptions> = {},
): string {
  const document = parseNibDocument(source, {
    preserveSourceInfo: true,
  });
  const doc = printNibDocWithOptions(document.ast, {
    eofTrivia: getEofTrivia(document.tokens),
  });

  return renderDocToAscii(doc, resolveOptions(options));
}

function renderDocToAscii(doc: Doc, options: Partial<NibFormatOptions> = {}): string {
  const resolved = resolveOptions(options);
  const scene = docToPaintScene(
    doc,
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

function printNibDocWithOptions(ast: ASTNode, options: PrintNibDocOptions): Doc {
  if (ast.ruleName !== "program") {
    throw new Error(`printNibDoc() expects a 'program' AST node, got '${ast.ruleName}'`);
  }

  const declarations = childRules(ast, "top_decl").map((decl) => ({
    anchor: decl,
    doc: printTopLevel(decl),
  }));

  return printAnchoredSequence(declarations, options.eofTrivia, 1);
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

function childTokens(node: ASTNode): Token[] {
  const tokens: Token[] = [];

  for (const child of node.children) {
    if (!isAstNode(child)) {
      tokens.push(child);
    }
  }

  return tokens;
}

function firstToken(node: ASTNode, tokenType?: string): Token | null {
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

function lastToken(node: ASTNode, tokenType?: string): Token | null {
  for (let index = node.children.length - 1; index >= 0; index -= 1) {
    const child = node.children[index];
    if (!child || isAstNode(child)) {
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

function requireToken(node: ASTNode, tokenType: string, context: string): Token {
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

  return group(
    joinAcrossTrivia(
      signature,
      printBlock(block),
      leadingTriviaOf(firstToken(block, "LBRACE")),
      " ",
    ),
  );
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
  const statements = childRules(node, "stmt").map((stmt) => ({
    anchor: stmt,
    doc: printStatement(unwrapSingleChild(stmt)),
  }));
  const closeBrace = lastToken(node, "RBRACE");
  const body = printAnchoredSequence(statements, leadingTriviaOf(closeBrace), 0);

  if (body.kind === "nil") {
    return concat([text("{"), text(" "), text("}")]);
  }

  return concat([
    text("{"),
    indent(concat([hardline(), body])),
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

  const header = concat([
    text("for "),
    text(loopVar),
    text(": "),
    printType(typeNode),
    text(" in "),
    printExpression(bounds[0]!),
    text(".."),
    printExpression(bounds[1]!),
  ]);

  return group(
    joinAcrossTrivia(
      header,
      printBlock(blockNode),
      leadingTriviaOf(firstToken(blockNode, "LBRACE")),
      " ",
    ),
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

  const ifHead = concat([text("if "), printExpression(condition)]);
  const thenDoc = group(
    joinAcrossTrivia(
      ifHead,
      printBlock(thenBlock),
      leadingTriviaOf(firstToken(thenBlock, "LBRACE")),
      " ",
    ),
  );

  if (!elseBlock) {
    return thenDoc;
  }

  const elseToken = childTokens(node).find((token) => token.value === "else");
  if (!elseToken) {
    throw new Error("Malformed if_stmt: expected else token before else-block");
  }

  const elseDoc = group(
    joinAcrossTrivia(
      text("else"),
      printBlock(elseBlock),
      leadingTriviaOf(firstToken(elseBlock, "LBRACE")),
      " ",
    ),
  );

  return group(joinAcrossTrivia(thenDoc, elseDoc, leadingTriviaOf(elseToken), " "));
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

function printAnchoredSequence(
  items: readonly SequenceItem[],
  boundaryTrivia: readonly Trivia[] | undefined,
  baseBlankLinesBetweenItems: number,
): Doc {
  const entries: SequenceEntry[] = [];
  let lastAttachableIndex: number | null = null;

  for (const item of items) {
    const disposition = classifyTrivia(leadingTriviaOf(item.anchor), lastAttachableIndex !== null);
    lastAttachableIndex = applyTriviaDisposition(
      entries,
      lastAttachableIndex,
      disposition,
      item.doc,
      baseBlankLinesBetweenItems,
    );
  }

  const boundaryDisposition = classifyTrivia(boundaryTrivia, lastAttachableIndex !== null);
  applyTriviaDisposition(entries, lastAttachableIndex, boundaryDisposition, null, 0);

  return renderEntries(entries);
}

function joinAcrossTrivia(
  left: Doc,
  right: Doc,
  trivia: readonly Trivia[] | undefined,
  inlineSeparator: string,
): Doc {
  if (!hasCommentTrivia(trivia)) {
    return concat([left, text(inlineSeparator), right]);
  }

  const entries: SequenceEntry[] = [
    { doc: left, blankLinesBefore: 0, attachable: true },
  ];

  applyTriviaDisposition(
    entries,
    0,
    classifyTrivia(trivia, true),
    right,
    0,
  );

  return renderEntries(entries);
}

function applyTriviaDisposition(
  entries: SequenceEntry[],
  lastAttachableIndex: number | null,
  disposition: TriviaDisposition,
  anchorDoc: Doc | null,
  baseBlankLinesBeforeAnchor: number,
): number | null {
  let effectiveLeadingComments = [...disposition.leadingComments];

  if (disposition.inlineTrailingComment !== null) {
    if (lastAttachableIndex !== null) {
      const entry = entries[lastAttachableIndex];
      entry.doc = appendTrailingComment(entry.doc, disposition.inlineTrailingComment);
    } else {
      effectiveLeadingComments = [
        { text: disposition.inlineTrailingComment, newlinesBefore: 0 },
        ...effectiveLeadingComments,
      ];
    }
  }

  let firstVisibleForAnchor = true;
  const hadPreviousAttachable = lastAttachableIndex !== null;
  let nextAttachableIndex = lastAttachableIndex;

  const pushEntry = (doc: Doc, sourceNewlinesBefore: number, attachable: boolean): void => {
    const sourceBlankLines = blankLinesFromNewlines(sourceNewlinesBefore);
    const blankLinesBefore =
      entries.length === 0
        ? 0
        : firstVisibleForAnchor && hadPreviousAttachable
          ? Math.max(baseBlankLinesBeforeAnchor, sourceBlankLines)
          : sourceBlankLines;

    entries.push({
      doc,
      blankLinesBefore,
      attachable,
    });

    firstVisibleForAnchor = false;
    if (attachable) {
      nextAttachableIndex = entries.length - 1;
    }
  };

  for (const comment of effectiveLeadingComments) {
    pushEntry(text(comment.text), comment.newlinesBefore, false);
  }

  if (anchorDoc !== null) {
    pushEntry(anchorDoc, disposition.trailingNewlinesBeforeAnchor, true);
  }

  return nextAttachableIndex;
}

function renderEntries(entries: readonly SequenceEntry[]): Doc {
  if (entries.length === 0) {
    return nil();
  }

  const out: Doc[] = [];
  for (let index = 0; index < entries.length; index += 1) {
    if (index > 0) {
      for (let lineIndex = 0; lineIndex < entries[index]!.blankLinesBefore + 1; lineIndex += 1) {
        out.push(hardline());
      }
    }
    out.push(entries[index]!.doc);
  }

  return concat(out);
}

function appendTrailingComment(doc: Doc, commentText: string): Doc {
  return concat([doc, text(" "), text(commentText)]);
}

function leadingTriviaOf(anchor: ASTNode | Token | null): readonly Trivia[] | undefined {
  return anchor?.leadingTrivia;
}

function getEofTrivia(tokens: readonly Token[]): readonly Trivia[] | undefined {
  const eof = tokens[tokens.length - 1];
  return eof?.type === "EOF" ? eof.leadingTrivia : undefined;
}

function hasCommentTrivia(trivia: readonly Trivia[] | undefined): boolean {
  return (trivia ?? []).some((item) => item.type === "LINE_COMMENT");
}

function classifyTrivia(
  trivia: readonly Trivia[] | undefined,
  canAttachInlineToPrevious: boolean,
): TriviaDisposition {
  const comments: TriviaCommentSegment[] = [];
  let pendingNewlines = 0;

  for (const item of trivia ?? []) {
    if (item.type === "LINE_COMMENT") {
      comments.push({
        text: item.value,
        newlinesBefore: pendingNewlines,
      });
      pendingNewlines = 0;
      continue;
    }

    pendingNewlines += countNewlines(item.value);
  }

  if (
    canAttachInlineToPrevious
    && comments.length > 0
    && comments[0]!.newlinesBefore === 0
  ) {
    return {
      inlineTrailingComment: comments[0]!.text,
      leadingComments: comments.slice(1),
      trailingNewlinesBeforeAnchor: pendingNewlines,
    };
  }

  return {
    inlineTrailingComment: null,
    leadingComments: comments,
    trailingNewlinesBeforeAnchor: pendingNewlines,
  };
}

function countNewlines(value: string): number {
  let count = 0;
  for (const ch of value) {
    if (ch === "\n") {
      count += 1;
    }
  }
  return count;
}

function blankLinesFromNewlines(newlines: number): number {
  return Math.max(0, newlines - 1);
}
