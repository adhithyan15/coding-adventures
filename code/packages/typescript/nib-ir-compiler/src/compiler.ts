import {
  IDGenerator,
  IrOp,
  IrProgram,
  imm,
  lbl,
  reg,
  type IrOperand,
} from "@coding-adventures/compiler-ir";
import type { SourceMapChain } from "@coding-adventures/compiler-source-map";
import type { Token } from "@coding-adventures/lexer";
import type { ASTNode } from "@coding-adventures/parser";
import { NibType } from "@coding-adventures/nib-type-checker";

import { type BuildConfig, debugConfig } from "./build-config.js";

const REG_ZERO = 0;
const REG_SCRATCH = 1;
const REG_VAR_BASE = 2;

type TypedAstNode = ASTNode & { _nibType?: NibType };
type NodeChild = ASTNode | Token;

export interface CompileResult {
  readonly program: IrProgram;
  readonly sourceMap: SourceMapChain | null;
}

function isAstNode(child: NodeChild): child is ASTNode {
  return "ruleName" in child;
}

function tokenType(token: Token): string {
  return token.type;
}

function unwrapTopDecl(child: ASTNode | Token): ASTNode | null {
  if (!isAstNode(child)) {
    return null;
  }
  for (const grandchild of child.children) {
    if (isAstNode(grandchild)) {
      return grandchild;
    }
  }
  return null;
}

function parseLiteral(value: string, type: string): number {
  try {
    return type === "HEX_LIT" ? Number.parseInt(value, 16) : Number.parseInt(value, 10);
  } catch {
    return 0;
  }
}

function firstTokenValue(subject: ASTNode | Token): string {
  if (!isAstNode(subject)) {
    return subject.value;
  }
  for (const child of subject.children) {
    const value = firstTokenValue(child);
    if (value) {
      return value;
    }
  }
  return "";
}

function resolveTypeNode(node: ASTNode | null): NibType | null {
  if (!node) {
    return null;
  }
  for (const child of node.children) {
    if (!isAstNode(child)) {
      switch (child.value) {
        case "u4":
          return NibType.U4;
        case "u8":
          return NibType.U8;
        case "bcd":
          return NibType.BCD;
        case "bool":
          return NibType.BOOL;
        default:
          return null;
      }
    }
  }
  return null;
}

function extractParams(paramListNode: ASTNode): Array<[string, NibType]> {
  const params: Array<[string, NibType]> = [];
  for (const child of paramListNode.children) {
    if (!isAstNode(child) || child.ruleName !== "param") {
      continue;
    }
    let name: string | null = null;
    let typeNode: ASTNode | null = null;
    for (const part of child.children) {
      if (isAstNode(part)) {
        if (part.ruleName === "type") {
          typeNode = part;
        }
        continue;
      }
      if (tokenType(part) === "NAME" && name === null) {
        name = part.value;
      }
    }
    const resolved = resolveTypeNode(typeNode);
    if (name && resolved) {
      params.push([name, resolved]);
    }
  }
  return params;
}

function isExpressionNode(node: ASTNode): boolean {
  return [
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
  ].includes(node.ruleName);
}

function extractConstInt(subject: ASTNode | Token): number {
  if (!isAstNode(subject)) {
    if (tokenType(subject) === "INT_LIT" || tokenType(subject) === "HEX_LIT") {
      return parseLiteral(subject.value, tokenType(subject));
    }
    return 0;
  }

  for (const child of subject.children) {
    const value = extractConstInt(child);
    if (value !== 0) {
      return value;
    }
  }
  return 0;
}

function hasFunctionNamed(ast: ASTNode, name: string): boolean {
  for (const child of ast.children) {
    const inner = unwrapTopDecl(child);
    if (!inner || inner.ruleName !== "fn_decl") {
      continue;
    }
    for (const part of inner.children) {
      if (!isAstNode(part) && tokenType(part) === "NAME" && part.value === name) {
        return true;
      }
    }
  }
  return false;
}

function typeSizeBytes(type: NibType): number {
  return type === NibType.U8 ? 2 : 1;
}

class Compiler {
  private readonly idGenerator = new IDGenerator();
  private readonly program = new IrProgram("_start");
  private loopCount = 0;
  private ifCount = 0;
  private readonly constValues = new Map<string, number>();

  constructor(private readonly config: BuildConfig) {}

  compile(ast: ASTNode): CompileResult {
    for (const child of ast.children) {
      const inner = unwrapTopDecl(child);
      if (!inner) {
        continue;
      }
      if (inner.ruleName === "const_decl") {
        const info = this.extractDeclInfo(inner);
        if (info.name !== null) {
          this.constValues.set(info.name, info.initValue);
        }
      } else if (inner.ruleName === "static_decl") {
        this.emitStaticData(inner);
      }
    }

    this.emitEntryPoint(ast);

    for (const child of ast.children) {
      const inner = unwrapTopDecl(child);
      if (inner?.ruleName === "fn_decl") {
        this.compileFunction(inner);
      }
    }

    return { program: this.program, sourceMap: null };
  }

  private emit(opcode: IrOp, ...operands: IrOperand[]): number {
    const id = this.idGenerator.next();
    this.program.addInstruction({ opcode, operands, id });
    return id;
  }

  private emitLabel(name: string): void {
    this.program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl(name)], id: -1 });
  }

  private emitComment(text: string): void {
    if (this.config.insertDebugComments) {
      this.program.addInstruction({ opcode: IrOp.COMMENT, operands: [lbl(text)], id: -1 });
    }
  }

  private extractDeclInfo(node: ASTNode): { name: string | null; type: NibType | null; initValue: number } {
    let name: string | null = null;
    let typeNode: ASTNode | null = null;
    let initValue = 0;

    for (const child of node.children) {
      if (isAstNode(child)) {
        if (child.ruleName === "type" && typeNode === null) {
          typeNode = child;
        } else if (isExpressionNode(child)) {
          initValue = extractConstInt(child);
        }
        continue;
      }

      if (name === null && tokenType(child) === "NAME") {
        name = child.value;
      } else if (tokenType(child) === "INT_LIT" || tokenType(child) === "HEX_LIT") {
        initValue = parseLiteral(child.value, tokenType(child));
      }
    }

    return { name, type: resolveTypeNode(typeNode), initValue };
  }

  private emitStaticData(node: ASTNode): void {
    const info = this.extractDeclInfo(node);
    if (!info.name || !info.type) {
      return;
    }
    this.emitComment(`static ${info.name}: ${info.type} = ${info.initValue}`);
    this.program.addData({
      label: info.name,
      size: typeSizeBytes(info.type),
      init: info.initValue,
    });
  }

  private emitEntryPoint(ast: ASTNode): void {
    this.emitLabel("_start");
    this.emitComment("program entry point: initialize v0=0, call main, halt");
    this.emit(IrOp.LOAD_IMM, reg(REG_ZERO), imm(0));
    if (hasFunctionNamed(ast, "main")) {
      this.emit(IrOp.CALL, lbl("_fn_main"));
    }
    this.emit(IrOp.HALT);
  }

  private compileFunction(node: ASTNode): void {
    let functionName: string | null = null;
    let blockNode: ASTNode | null = null;
    let params: Array<[string, NibType]> = [];

    for (const child of node.children) {
      if (isAstNode(child)) {
        if (child.ruleName === "param_list") {
          params = extractParams(child);
        } else if (child.ruleName === "block") {
          blockNode = child;
        }
        continue;
      }
      if (functionName === null && tokenType(child) === "NAME") {
        functionName = child.value;
      }
    }

    if (!functionName || !blockNode) {
      return;
    }

    this.emitComment(
      `function: ${functionName}(${params.map(([name, type]) => `${name}: ${type}`).join(", ")})`,
    );
    this.emitLabel(`_fn_${functionName}`);

    const registers = new Map<string, number>();
    let nextRegister = REG_VAR_BASE;
    for (const [name] of params) {
      registers.set(name, nextRegister);
      nextRegister += 1;
    }

    this.compileBlock(blockNode, registers, nextRegister);
    this.emit(IrOp.RET);
  }

  private compileBlock(block: ASTNode, registers: Map<string, number>, nextRegister: number): number {
    let current = nextRegister;
    for (const child of block.children) {
      if (isAstNode(child) && child.ruleName === "stmt") {
        const inner = child.children[0];
        if (inner && isAstNode(inner)) {
          current = this.compileStatement(inner, registers, current);
        }
      }
    }
    return current;
  }

  private compileStatement(node: ASTNode, registers: Map<string, number>, nextRegister: number): number {
    switch (node.ruleName) {
      case "let_stmt":
        return this.compileLet(node, registers, nextRegister);
      case "assign_stmt":
        this.compileAssign(node, registers);
        return nextRegister;
      case "return_stmt":
        this.compileReturn(node, registers);
        return nextRegister;
      case "for_stmt":
        return this.compileFor(node, registers, nextRegister);
      case "if_stmt":
        this.compileIf(node, registers, nextRegister);
        return nextRegister;
      case "expr_stmt": {
        const expr = node.children.find((child) => isAstNode(child) && isExpressionNode(child));
        if (expr && isAstNode(expr)) {
          this.compileExpr(expr, registers);
        }
        return nextRegister;
      }
      default:
        return nextRegister;
    }
  }

  private compileLet(node: ASTNode, registers: Map<string, number>, nextRegister: number): number {
    let name: string | null = null;
    let typeNode: ASTNode | null = null;
    let expression: ASTNode | null = null;

    for (const child of node.children) {
      if (isAstNode(child)) {
        if (child.ruleName === "type") {
          typeNode = child;
        } else if (isExpressionNode(child)) {
          expression = child;
        }
        continue;
      }
      if (name === null && tokenType(child) === "NAME") {
        name = child.value;
      }
    }

    if (!name || !expression) {
      return nextRegister;
    }

    const destination = nextRegister;
    registers.set(name, destination);
    const resultRegister = this.compileExpr(expression, registers);
    if (resultRegister !== destination) {
      this.emit(IrOp.ADD_IMM, reg(destination), reg(resultRegister), imm(0));
    }
    this.emitComment(`let ${name}: ${resolveTypeNode(typeNode) ?? "?"}`);
    return nextRegister + 1;
  }

  private compileAssign(node: ASTNode, registers: Map<string, number>): void {
    let name: string | null = null;
    let expression: ASTNode | null = null;
    for (const child of node.children) {
      if (isAstNode(child) && isExpressionNode(child)) {
        expression = child;
      } else if (!isAstNode(child) && name === null && tokenType(child) === "NAME") {
        name = child.value;
      }
    }
    if (!name || !expression) {
      return;
    }
    const target = registers.get(name);
    if (target === undefined) {
      return;
    }
    const valueRegister = this.compileExpr(expression, registers);
    this.emit(IrOp.ADD_IMM, reg(target), reg(valueRegister), imm(0));
  }

  private compileReturn(node: ASTNode, registers: Map<string, number>): void {
    const expression = node.children.find((child) => isAstNode(child) && isExpressionNode(child));
    if (expression && isAstNode(expression)) {
      const valueRegister = this.compileExpr(expression, registers);
      if (valueRegister !== REG_SCRATCH) {
        this.emit(IrOp.ADD_IMM, reg(REG_SCRATCH), reg(valueRegister), imm(0));
      }
    }
    this.emit(IrOp.RET);
  }

  private compileFor(node: ASTNode, registers: Map<string, number>, nextRegister: number): number {
    let loopVar: string | null = null;
    let blockNode: ASTNode | null = null;
    const exprs = node.children.filter((child) => isAstNode(child) && isExpressionNode(child)) as ASTNode[];

    for (const child of node.children) {
      if (isAstNode(child)) {
        if (child.ruleName === "block") {
          blockNode = child;
        }
        continue;
      }
      if (loopVar === null && tokenType(child) === "NAME") {
        loopVar = child.value;
      }
    }

    if (!loopVar || exprs.length < 2 || !blockNode) {
      return nextRegister;
    }

    const loopRegister = nextRegister;
    const limitRegister = nextRegister + 1;
    const startLabel = `loop_${this.loopCount}_start`;
    const endLabel = `loop_${this.loopCount}_end`;
    this.loopCount += 1;

    registers.set(loopVar, loopRegister);
    const startValueRegister = this.compileExpr(exprs[0], registers);
    if (startValueRegister !== loopRegister) {
      this.emit(IrOp.ADD_IMM, reg(loopRegister), reg(startValueRegister), imm(0));
    }
    const limitValueRegister = this.compileExpr(exprs[1], registers);
    if (limitValueRegister !== limitRegister) {
      this.emit(IrOp.ADD_IMM, reg(limitRegister), reg(limitValueRegister), imm(0));
    }

    this.emitLabel(startLabel);
    const compareRegister = REG_SCRATCH;
    this.emit(IrOp.CMP_LT, reg(compareRegister), reg(loopRegister), reg(limitRegister));
    this.emit(IrOp.BRANCH_Z, reg(compareRegister), lbl(endLabel));

    const nested = new Map(registers);
    this.compileBlock(blockNode, nested, nextRegister + 2);
    this.emit(IrOp.ADD_IMM, reg(loopRegister), reg(loopRegister), imm(1));
    this.emit(IrOp.JUMP, lbl(startLabel));
    this.emitLabel(endLabel);
    return nextRegister + 2;
  }

  private compileIf(node: ASTNode, registers: Map<string, number>, nextRegister: number): void {
    const expr = node.children.find((child) => isAstNode(child) && isExpressionNode(child));
    if (!expr || !isAstNode(expr)) {
      return;
    }

    const conditionRegister = this.compileExpr(expr, registers);
    const elseLabel = `if_${this.ifCount}_else`;
    const endLabel = `if_${this.ifCount}_end`;
    this.ifCount += 1;

    this.emit(IrOp.BRANCH_Z, reg(conditionRegister), lbl(elseLabel));

    const blocks = node.children.filter((child) => isAstNode(child) && child.ruleName === "block") as ASTNode[];
    if (blocks[0]) {
      this.compileBlock(blocks[0], new Map(registers), nextRegister);
    }
    this.emit(IrOp.JUMP, lbl(endLabel));
    this.emitLabel(elseLabel);
    if (blocks[1]) {
      this.compileBlock(blocks[1], new Map(registers), nextRegister);
    }
    this.emitLabel(endLabel);
  }

  private compileExpr(node: ASTNode | Token, registers: Map<string, number>): number {
    if (!isAstNode(node)) {
      return this.compileTokenExpr(node, registers);
    }

    switch (node.ruleName) {
      case "call_expr":
        return this.compileCallExpr(node, registers);
      case "primary":
        return this.compilePrimary(node, registers);
      case "add_expr":
        return this.compileAddExpr(node, registers);
      case "or_expr":
      case "and_expr":
      case "eq_expr":
      case "cmp_expr":
      case "bitwise_expr":
      case "unary_expr":
      case "expr":
        return this.compileCompoundExpr(node, registers);
      default:
        if (node.children.length === 1) {
          return this.compileExpr(node.children[0], registers);
        }
        return REG_SCRATCH;
    }
  }

  private compileTokenExpr(token: Token, registers: Map<string, number>): number {
    if (tokenType(token) === "INT_LIT" || tokenType(token) === "HEX_LIT") {
      this.emit(IrOp.LOAD_IMM, reg(REG_SCRATCH), imm(parseLiteral(token.value, tokenType(token))));
      return REG_SCRATCH;
    }

    if (token.value === "true" || token.value === "false") {
      this.emit(IrOp.LOAD_IMM, reg(REG_SCRATCH), imm(token.value === "true" ? 1 : 0));
      return REG_SCRATCH;
    }

    const registerIndex = registers.get(token.value);
    if (registerIndex !== undefined) {
      return registerIndex;
    }

    if (this.constValues.has(token.value)) {
      this.emit(IrOp.LOAD_IMM, reg(REG_SCRATCH), imm(this.constValues.get(token.value) ?? 0));
      return REG_SCRATCH;
    }

    return REG_SCRATCH;
  }

  private compilePrimary(node: ASTNode, registers: Map<string, number>): number {
    const first = node.children[0];
    if (!first) {
      return REG_SCRATCH;
    }
    return this.compileExpr(first, registers);
  }

  private compileCallExpr(node: ASTNode, registers: Map<string, number>): number {
    let functionName: string | null = null;
    const args: ASTNode[] = [];

    for (const child of node.children) {
      if (isAstNode(child)) {
        if (child.ruleName === "arg_list") {
          for (const argChild of child.children) {
            if (isAstNode(argChild) && isExpressionNode(argChild)) {
              args.push(argChild);
            }
          }
        }
        continue;
      }
      if (functionName === null && tokenType(child) === "NAME") {
        functionName = child.value;
      }
    }

    if (!functionName) {
      return REG_SCRATCH;
    }

    args.forEach((arg, index) => {
      const valueRegister = this.compileExpr(arg, registers);
      const destination = REG_VAR_BASE + index;
      if (valueRegister !== destination) {
        this.emit(IrOp.ADD_IMM, reg(destination), reg(valueRegister), imm(0));
      }
    });
    this.emit(IrOp.CALL, lbl(`_fn_${functionName}`));
    return REG_SCRATCH;
  }

  private compileCompoundExpr(node: ASTNode, registers: Map<string, number>): number {
    const children = node.children;
    if (children.length === 1) {
      return this.compileExpr(children[0], registers);
    }

    if (
      node.ruleName === "unary_expr" &&
      children.length >= 2 &&
      !isAstNode(children[0]) &&
      (children[0].value === "!" || children[0].value === "~")
    ) {
      const operandRegister = this.compileExpr(children[1], registers);
      return this.emitUnary(children[0].value, operandRegister, node);
    }

    let leftRegister = this.compileExpr(children[0], registers);
    for (let index = 1; index < children.length - 1; index += 2) {
      const operator = children[index];
      const rightNode = children[index + 1];
      const operatorValue = isAstNode(operator) ? firstTokenValue(operator) : operator.value;
      const rightRegister = this.compileExpr(rightNode, registers);
      leftRegister = this.emitBinary(operatorValue, leftRegister, rightRegister);
    }
    return leftRegister;
  }

  private compileAddExpr(node: ASTNode, registers: Map<string, number>): number {
    const children = node.children;
    if (children.length === 1) {
      return this.compileExpr(children[0], registers);
    }

    let leftRegister = this.compileExpr(children[0], registers);
    for (let index = 1; index < children.length - 1; index += 2) {
      const operator = children[index];
      const rightNode = children[index + 1];
      const operatorValue = isAstNode(operator) ? firstTokenValue(operator) : operator.value;
      const rightRegister = this.compileExpr(rightNode, registers);
      const nibType = (node as TypedAstNode)._nibType ?? null;
      leftRegister = this.emitAddOp(operatorValue, leftRegister, rightRegister, nibType);
    }
    return leftRegister;
  }

  private emitUnary(operator: string, operandRegister: number, node: ASTNode): number {
    if (operator === "!") {
      this.emit(IrOp.CMP_EQ, reg(REG_SCRATCH), reg(operandRegister), reg(REG_ZERO));
      return REG_SCRATCH;
    }

    if (operator === "~") {
      const nibType = (node as TypedAstNode)._nibType ?? null;
      const mask = nibType === NibType.U8 ? 0xff : 0x0f;
      this.emit(IrOp.LOAD_IMM, reg(REG_SCRATCH), imm(mask));
      this.emit(IrOp.SUB, reg(REG_SCRATCH), reg(REG_SCRATCH), reg(operandRegister));
      return REG_SCRATCH;
    }

    return operandRegister;
  }

  private emitBinary(operator: string, leftRegister: number, rightRegister: number): number {
    switch (operator) {
      case "==":
        this.emit(IrOp.CMP_EQ, reg(REG_SCRATCH), reg(leftRegister), reg(rightRegister));
        return REG_SCRATCH;
      case "!=":
        this.emit(IrOp.CMP_NE, reg(REG_SCRATCH), reg(leftRegister), reg(rightRegister));
        return REG_SCRATCH;
      case "<":
        this.emit(IrOp.CMP_LT, reg(REG_SCRATCH), reg(leftRegister), reg(rightRegister));
        return REG_SCRATCH;
      case ">":
        this.emit(IrOp.CMP_GT, reg(REG_SCRATCH), reg(leftRegister), reg(rightRegister));
        return REG_SCRATCH;
      case "<=":
        this.emit(IrOp.CMP_GT, reg(REG_SCRATCH), reg(rightRegister), reg(leftRegister));
        return REG_SCRATCH;
      case ">=":
        this.emit(IrOp.CMP_LT, reg(REG_SCRATCH), reg(rightRegister), reg(leftRegister));
        return REG_SCRATCH;
      case "&&":
        this.emit(IrOp.AND, reg(REG_SCRATCH), reg(leftRegister), reg(rightRegister));
        return REG_SCRATCH;
      case "||":
        this.emit(IrOp.ADD, reg(REG_SCRATCH), reg(leftRegister), reg(rightRegister));
        this.emit(IrOp.CMP_NE, reg(REG_SCRATCH), reg(REG_SCRATCH), reg(REG_ZERO));
        return REG_SCRATCH;
      case "&":
        this.emit(IrOp.AND, reg(REG_SCRATCH), reg(leftRegister), reg(rightRegister));
        return REG_SCRATCH;
      default:
        return leftRegister;
    }
  }

  private emitAddOp(operator: string, leftRegister: number, rightRegister: number, nibType: NibType | null): number {
    switch (operator) {
      case "+%":
        this.emit(IrOp.ADD, reg(REG_SCRATCH), reg(leftRegister), reg(rightRegister));
        if (nibType === NibType.BCD) {
          this.emitComment("bcd +%: backend should emit DAA after ADD");
          this.emit(IrOp.AND_IMM, reg(REG_SCRATCH), reg(REG_SCRATCH), imm(255));
        } else if (nibType === NibType.U4) {
          this.emit(IrOp.AND_IMM, reg(REG_SCRATCH), reg(REG_SCRATCH), imm(15));
        } else {
          this.emit(IrOp.AND_IMM, reg(REG_SCRATCH), reg(REG_SCRATCH), imm(255));
        }
        return REG_SCRATCH;
      case "-":
        this.emit(IrOp.SUB, reg(REG_SCRATCH), reg(leftRegister), reg(rightRegister));
        return REG_SCRATCH;
      case "+":
      case "+?":
        this.emit(IrOp.ADD, reg(REG_SCRATCH), reg(leftRegister), reg(rightRegister));
        return REG_SCRATCH;
      default:
        return leftRegister;
    }
  }
}

export function compileNib(typedAst: ASTNode, config: BuildConfig = debugConfig()): CompileResult {
  if (typedAst.ruleName !== "program") {
    throw new Error(`expected 'program' AST node, got '${typedAst.ruleName}'`);
  }
  return new Compiler(config).compile(typedAst);
}
