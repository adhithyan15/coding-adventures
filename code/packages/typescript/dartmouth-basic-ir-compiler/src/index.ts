import { BackendRegistry, type Artifact } from "@coding-adventures/codegen-core";
import { FunctionTypeStatus, IirFunction, IirInstr, IirModule, Types } from "@coding-adventures/interpreter-ir";
import { JITCore } from "@coding-adventures/jit-core";
import { VMCore } from "@coding-adventures/vm-core";

export interface BasicLine { number: number; text: string }
export interface CompileResult { module: IirModule; varNames: string[] }
type BasicExpr = { kind: "number"; value: number } | { kind: "var"; name: string } | { kind: "binary"; op: string; left: BasicExpr; right: BasicExpr };
interface ForLoop { variable: string; label: string; limit: string; step: string; descending: boolean }
const OPS = new Map([["+", "add"], ["-", "sub"], ["*", "mul"], ["/", "div"]]);
const CMPS = new Map([["=", "cmp_eq"], ["<>", "cmp_ne"], ["<", "cmp_lt"], ["<=", "cmp_le"], [">", "cmp_gt"], [">=", "cmp_ge"]]);

export function parseBasicLines(source: string): BasicLine[] {
  return source.split(/\r?\n/).map((l) => l.trim()).filter(Boolean).map((line) => {
    const space = line.search(/\s/); const numberText = space === -1 ? line : line.slice(0, space);
    if (!/^\d+$/.test(numberText)) throw new Error(`missing BASIC line number: ${line}`);
    return { number: Number(numberText), text: space === -1 ? "" : line.slice(space).trim() };
  }).sort((a, b) => a.number - b.number);
}

export function compileDartmouthBasic(source: string, moduleName = "dartmouth-basic"): CompileResult {
  const c = new Ctx();
  for (const line of parseBasicLines(source)) { c.emit("label", { srcs: [lineLabel(line.number)] }); compileLine(line, c); }
  c.emit("ret_void");
  const module = new IirModule({ name: moduleName, functions: [new IirFunction({ name: "main", returnType: Types.Void, instructions: c.instructions, registerCount: c.registerCount(), typeStatus: FunctionTypeStatus.PartiallyTyped })], entryPoint: "main", language: "dartmouth-basic" });
  module.validate();
  return { module, varNames: [...c.varNames].sort() };
}
export function runDartmouthBasic(source: string, jit = false): string {
  const { module } = compileDartmouthBasic(source); let output = ""; const vm = new VMCore();
  vm.registerBuiltin("__basic_print", (args) => { output += `${String(args[0] ?? "")}\n`; return null; });
  jit ? new JITCore(vm).executeWithJit(module) : vm.execute(module);
  return output;
}
export function emitDartmouthBasic(source: string, target: string): Artifact { return BackendRegistry.default().compile(compileDartmouthBasic(source).module, target); }

function compileLine(line: BasicLine, c: Ctx): void {
  const text = line.text.trim(); const upper = text.toUpperCase();
  if (text.length === 0 || upper.startsWith("REM")) return;
  if (upper === "END" || upper === "STOP") { c.emit("ret_void"); return; }
  if (upper.startsWith("PRINT")) { compilePrint(text.slice(5).trim(), c); return; }
  if (upper.startsWith("GOTO")) { c.emit("jmp", { srcs: [lineLabel(Number(text.slice(4).trim()))] }); return; }
  if (upper.startsWith("IF")) { compileIf(text.slice(2).trim(), c); return; }
  if (upper.startsWith("FOR")) { compileFor(line.number, text.slice(3).trim(), c); return; }
  if (upper.startsWith("NEXT")) { compileNext(text.slice(4).trim(), c); return; }
  compileAssignment(text, c);
}
function compilePrint(rest: string, c: Ctx): void {
  if (rest.length === 0) { const d = c.temp(); c.emit("const", { dest: d, srcs: [""], typeHint: Types.Str }); c.emit("call_builtin", { srcs: ["__basic_print", d], typeHint: Types.Nil }); return; }
  if (rest.startsWith("\"") && rest.endsWith("\"") && rest.length >= 2) { const d = c.temp(); c.emit("const", { dest: d, srcs: [rest.slice(1, -1)], typeHint: Types.Str }); c.emit("call_builtin", { srcs: ["__basic_print", d], typeHint: Types.Nil }); return; }
  c.emit("call_builtin", { srcs: ["__basic_print", compileExpr(parseBasicExpr(rest), c)], typeHint: Types.Nil });
}
function compileIf(rest: string, c: Ctx): void {
  const then = rest.toUpperCase().indexOf("THEN"); if (then < 0) throw new Error("IF requires THEN");
  const { left, op, right } = splitCondition(rest.slice(0, then).trim()); const target = Number(rest.slice(then + 4).trim()); const d = c.temp();
  c.emit(CMPS.get(op) ?? "cmp_eq", { dest: d, srcs: [compileExpr(parseBasicExpr(left), c), compileExpr(parseBasicExpr(right), c)], typeHint: Types.Bool });
  c.emit("jmp_if_true", { srcs: [d, lineLabel(target)] });
}
function compileFor(line: number, rest: string, c: Ctx): void {
  const eq = rest.indexOf("="); if (eq < 0) throw new Error("FOR requires =");
  const variable = validateVar(rest.slice(0, eq).trim()); const afterEq = rest.slice(eq + 1).trim(); const to = afterEq.toUpperCase().indexOf(" TO "); if (to < 0) throw new Error("FOR requires TO");
  const startText = afterEq.slice(0, to).trim(); const afterTo = afterEq.slice(to + 4).trim(); const step = afterTo.toUpperCase().indexOf(" STEP ");
  const limitText = step < 0 ? afterTo : afterTo.slice(0, step).trim(); const stepText = step < 0 ? "1" : afterTo.slice(step + 6).trim(); const varReg = c.varRegister(variable);
  c.emit("move", { dest: varReg, srcs: [compileExpr(parseBasicExpr(startText), c)], typeHint: Types.U64 });
  const label = `for_${line}_${c.loopDepth}`; c.emit("label", { srcs: [label] });
  c.forLoops.push({ variable, label, limit: compileExpr(parseBasicExpr(limitText), c), step: compileExpr(parseBasicExpr(stepText), c), descending: stepText.trim().startsWith("-") });
}
function compileNext(rest: string, c: Ctx): void {
  const expected = rest.length === 0 ? null : validateVar(rest.trim()); const loop = c.forLoops.pop(); if (!loop) throw new Error("NEXT without FOR"); if (expected !== null && expected !== loop.variable) throw new Error(`NEXT ${expected} does not match FOR ${loop.variable}`);
  const reg = c.varRegister(loop.variable); c.emit("add", { dest: reg, srcs: [reg, loop.step], typeHint: Types.U64 }); const keep = c.temp();
  c.emit(loop.descending ? "cmp_ge" : "cmp_le", { dest: keep, srcs: [reg, loop.limit], typeHint: Types.Bool }); c.emit("jmp_if_true", { srcs: [keep, loop.label] });
}
function compileAssignment(text: string, c: Ctx): void { const a = parseAssignment(text); c.emit("move", { dest: c.varRegister(a.name), srcs: [compileExpr(parseBasicExpr(a.expr), c)], typeHint: Types.U64 }); }
function parseAssignment(text: string): { name: string; expr: string } { const body = text.trim().toUpperCase().startsWith("LET ") ? text.trim().slice(4).trim() : text.trim(); const eq = body.indexOf("="); if (eq < 0) throw new Error(`expected assignment: ${text}`); return { name: validateVar(body.slice(0, eq).trim()), expr: body.slice(eq + 1).trim() }; }

export function tokenizeBasicExpr(source: string): string[] {
  const tokens: string[] = []; let i = 0;
  while (i < source.length) {
    const c = source[i] ?? ""; if (/\s/.test(c)) { i += 1; continue; }
    if (/[0-9]/.test(c)) { const s = i; while (/[0-9]/.test(source[i] ?? "")) i += 1; tokens.push(source.slice(s, i)); continue; }
    if (/[A-Za-z]/.test(c)) { const s = i; while (/[A-Za-z0-9]/.test(source[i] ?? "")) i += 1; tokens.push(validateVar(source.slice(s, i))); continue; }
    if ("()+-*/".includes(c)) { tokens.push(c); i += 1; continue; }
    throw new Error(`unexpected BASIC expression character: ${c}`);
  }
  return tokens;
}
function parseBasicExpr(source: string): BasicExpr { return new ExprParser(tokenizeBasicExpr(source)).parse(); }
class ExprParser {
  private p = 0; constructor(private readonly tokens: string[]) {}
  parse(): BasicExpr { const e = this.add(); if (this.peek() !== undefined) throw new Error(`unexpected expression token: ${this.peek()}`); return e; }
  private add(): BasicExpr { let e = this.mul(); while (this.peek() === "+" || this.peek() === "-") { const op = this.next(); e = { kind: "binary", op, left: e, right: this.mul() }; } return e; }
  private mul(): BasicExpr { let e = this.primary(); while (this.peek() === "*" || this.peek() === "/") { const op = this.next(); e = { kind: "binary", op, left: e, right: this.primary() }; } return e; }
  private primary(): BasicExpr { if (this.peek() === "-") { this.next(); return { kind: "binary", op: "-", left: { kind: "number", value: 0 }, right: this.primary() }; } if (this.peek() === "(") { this.next(); const e = this.add(); this.expect(")"); return e; } const t = this.next(); return /^\d+$/.test(t) ? { kind: "number", value: Number(t) } : { kind: "var", name: validateVar(t) }; }
  private expect(t: string): void { if (this.next() !== t) throw new Error(`expected ${t}`); }
  private next(): string { const t = this.tokens[this.p]; if (t === undefined) throw new Error("unexpected end of BASIC expression"); this.p += 1; return t; }
  private peek(): string | undefined { return this.tokens[this.p]; }
}
function compileExpr(e: BasicExpr, c: Ctx): string {
  if (e.kind === "number") { const d = c.temp(); c.emit("const", { dest: d, srcs: [e.value], typeHint: Types.U64 }); return d; }
  if (e.kind === "var") return c.varRegister(e.name);
  const d = c.temp(); c.emit(OPS.get(e.op) ?? "add", { dest: d, srcs: [compileExpr(e.left, c), compileExpr(e.right, c)], typeHint: Types.U64 }); return d;
}
function splitCondition(condition: string): { left: string; op: string; right: string } { for (const op of ["<=", ">=", "<>", "=", "<", ">"]) { const i = condition.indexOf(op); if (i >= 0) return { left: condition.slice(0, i).trim(), op, right: condition.slice(i + op.length).trim() }; } throw new Error(`missing comparison operator: ${condition}`); }
function validateVar(name: string): string { const upper = name.toUpperCase(); if (!/^[A-Z][A-Z0-9]?$/.test(upper)) throw new Error(`invalid BASIC variable: ${name}`); return upper; }
function lineLabel(n: number): string { return `_line_${n}`; }
class Ctx { readonly instructions: IirInstr[] = []; readonly varNames = new Set<string>(); readonly forLoops: ForLoop[] = []; private n = 0; get loopDepth(): number { return this.forLoops.length; } temp(): string { const t = `t${this.n}`; this.n += 1; return t; } varRegister(name: string): string { const v = validateVar(name); this.varNames.add(v); return `v_${v}`; } emit(op: string, options: Omit<ConstructorParameters<typeof IirInstr>[0], "op"> = {}): void { this.instructions.push(IirInstr.of(op, options)); } registerCount(): number { return Math.max(64, this.n + this.varNames.size); } }
