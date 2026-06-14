import { BackendRegistry, type Artifact } from "@coding-adventures/codegen-core";
import { FunctionTypeStatus, IirFunction, IirInstr, IirModule, Types } from "@coding-adventures/interpreter-ir";
import { JITCore } from "@coding-adventures/jit-core";
import { VMCore, type VMValue } from "@coding-adventures/vm-core";

export type TetradExpr =
  | { kind: "number"; value: number }
  | { kind: "var"; name: string }
  | { kind: "binary"; left: TetradExpr; op: string; right: TetradExpr }
  | { kind: "call"; name: string; args: TetradExpr[] };
export type TetradStatement =
  | { kind: "let" | "assign"; name: string; expr: TetradExpr }
  | { kind: "return"; expr: TetradExpr }
  | { kind: "expr"; expr: TetradExpr };
export interface TetradFunctionDef { kind: "function"; name: string; params: string[]; body: TetradStatement[] }
export interface TetradProgram { forms: Array<TetradFunctionDef | TetradStatement> }
interface Token { type: "name" | "number" | "keyword" | "symbol" | "eof"; value: string }

const KEYWORDS = new Set(["fn", "let", "return"]);
const OPS = new Map([["+", "add"], ["-", "sub"], ["*", "mul"], ["/", "div"], ["%", "mod"]]);

export function tokenizeTetrad(source: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;
  while (i < source.length) {
    const c = source[i] ?? "";
    if (/\s/.test(c)) { i += 1; continue; }
    if (c === "#") { while (i < source.length && source[i] !== "\n") i += 1; continue; }
    if (c === ":" && source[i + 1] === "=") { tokens.push({ type: "symbol", value: ":=" }); i += 2; continue; }
    if ("+-*/%(),{}=;".includes(c)) { tokens.push({ type: "symbol", value: c }); i += 1; continue; }
    if (/\d/.test(c)) {
      const s = i; while (/\d/.test(source[i] ?? "")) i += 1;
      tokens.push({ type: "number", value: source.slice(s, i) }); continue;
    }
    if (/[A-Za-z_]/.test(c)) {
      const s = i; while (/[A-Za-z0-9_]/.test(source[i] ?? "")) i += 1;
      const value = source.slice(s, i);
      tokens.push({ type: KEYWORDS.has(value) ? "keyword" : "name", value }); continue;
    }
    throw new Error(`unexpected Tetrad character: ${c}`);
  }
  tokens.push({ type: "eof", value: "" });
  return tokens;
}

export function parseTetrad(source: string): TetradProgram { return new Parser(tokenizeTetrad(source)).parseProgram(); }

class Parser {
  private p = 0;
  constructor(private readonly tokens: Token[]) {}
  parseProgram(): TetradProgram {
    const forms: Array<TetradFunctionDef | TetradStatement> = [];
    while (this.peek().type !== "eof") { forms.push(this.peek().value === "fn" ? this.parseFunction() : this.parseStatement()); this.semis(); }
    return { forms };
  }
  private parseFunction(): TetradFunctionDef {
    this.consume("keyword", "fn"); const name = this.consume("name").value; this.consume("symbol", "(");
    const params: string[] = [];
    if (!this.match("symbol", ")")) { do params.push(this.consume("name").value); while (this.match("symbol", ",")); this.consume("symbol", ")"); }
    this.consume("symbol", "{"); const body: TetradStatement[] = [];
    while (!this.match("symbol", "}")) { body.push(this.parseStatement()); this.semis(); }
    return { kind: "function", name, params, body };
  }
  private parseStatement(): TetradStatement {
    if (this.match("keyword", "let")) { const name = this.consume("name").value; this.consume("symbol", "="); return { kind: "let", name, expr: this.expr() }; }
    if (this.match("keyword", "return")) return { kind: "return", expr: this.expr() };
    if (this.peek().type === "name" && (this.peek(1).value === "=" || this.peek(1).value === ":=")) { const name = this.consume("name").value; this.p += 1; return { kind: "assign", name, expr: this.expr() }; }
    return { kind: "expr", expr: this.expr() };
  }
  private expr(): TetradExpr { return this.add(); }
  private add(): TetradExpr {
    let e = this.mul();
    while (this.peek().value === "+" || this.peek().value === "-") { const op = this.consume("symbol").value; e = { kind: "binary", left: e, op, right: this.mul() }; }
    return e;
  }
  private mul(): TetradExpr {
    let e = this.primary();
    while (this.peek().value === "*" || this.peek().value === "/" || this.peek().value === "%") { const op = this.consume("symbol").value; e = { kind: "binary", left: e, op, right: this.primary() }; }
    return e;
  }
  private primary(): TetradExpr {
    if (this.peek().type === "number") return { kind: "number", value: Number(this.consume("number").value) };
    if (this.peek().type === "name") {
      const name = this.consume("name").value;
      if (this.match("symbol", "(")) { const args: TetradExpr[] = []; if (!this.match("symbol", ")")) { do args.push(this.expr()); while (this.match("symbol", ",")); this.consume("symbol", ")"); } return { kind: "call", name, args }; }
      return { kind: "var", name };
    }
    if (this.match("symbol", "(")) { const e = this.expr(); this.consume("symbol", ")"); return e; }
    throw new Error(`expected expression, got ${this.peek().value}`);
  }
  private consume(type: Token["type"], value?: string): Token { const t = this.peek(); if (t.type !== type || (value !== undefined && t.value !== value)) throw new Error(`expected ${value ?? type}, got ${t.value}`); this.p += 1; return t; }
  private match(type: Token["type"], value: string): boolean { if (this.peek().type === type && this.peek().value === value) { this.p += 1; return true; } return false; }
  private semis(): void { while (this.match("symbol", ";")) {} }
  private peek(offset = 0): Token { return this.tokens[this.p + offset] ?? { type: "eof", value: "" }; }
}

export function compileTetrad(source: string, moduleName = "tetrad"): IirModule {
  const program = parseTetrad(source);
  const functions: IirFunction[] = [];
  const top: TetradStatement[] = [];
  for (const form of program.forms) form.kind === "function" ? functions.push(compileFunction(form)) : top.push(form);
  if (functions.every((fn) => fn.name !== "main")) functions.push(compileFunction({ kind: "function", name: "main", params: [], body: top }));
  const mod = new IirModule({ name: moduleName, functions, entryPoint: "main", language: "tetrad" });
  mod.validate();
  return mod;
}
export function runTetrad(source: string, jit = false): VMValue { const mod = compileTetrad(source); const vm = new VMCore({ u8Wrap: true }); return jit ? new JITCore(vm).executeWithJit(mod) : vm.execute(mod); }
export function emitTetrad(source: string, target: string): Artifact { return BackendRegistry.default().compile(compileTetrad(source), target); }

function compileFunction(def: TetradFunctionDef): IirFunction {
  const c = new Ctx(); let terminated = false;
  for (const stmt of def.body) if (!terminated) terminated = compileStmt(stmt, c);
  if (!terminated) c.emit("ret_void");
  return new IirFunction({ name: def.name, params: def.params.map((name) => ({ name, type: Types.U8 })), returnType: terminated ? Types.U8 : Types.Void, instructions: c.instructions, registerCount: c.registerCount(def.params.length), typeStatus: FunctionTypeStatus.FullyTyped });
}
function compileStmt(stmt: TetradStatement, c: Ctx): boolean {
  if (stmt.kind === "let" || stmt.kind === "assign") { c.emit("tetrad.move", { dest: stmt.name, srcs: [compileExpr(stmt.expr, c)], typeHint: Types.U8 }); return false; }
  if (stmt.kind === "return") { c.emit("ret", { srcs: [compileExpr(stmt.expr, c)] }); return true; }
  compileExpr(stmt.expr, c); return false;
}
function compileExpr(expr: TetradExpr, c: Ctx): string {
  if (expr.kind === "number") { const d = c.temp(); c.emit("const", { dest: d, srcs: [expr.value & 0xff], typeHint: Types.U8 }); return d; }
  if (expr.kind === "var") return expr.name;
  if (expr.kind === "binary") { const d = c.temp(); c.emit(OPS.get(expr.op) ?? "add", { dest: d, srcs: [compileExpr(expr.left, c), compileExpr(expr.right, c)], typeHint: Types.U8 }); return d; }
  const d = c.temp(); c.emit("call", { dest: d, srcs: [expr.name, ...expr.args.map((arg) => compileExpr(arg, c))], typeHint: Types.U8 }); return d;
}
class Ctx {
  readonly instructions: IirInstr[] = []; private n = 0;
  temp(): string { const name = `t${this.n}`; this.n += 1; return name; }
  emit(op: string, options: Omit<ConstructorParameters<typeof IirInstr>[0], "op"> = {}): void { this.instructions.push(IirInstr.of(op, options)); }
  registerCount(params: number): number { return Math.max(32, this.n + params); }
}
