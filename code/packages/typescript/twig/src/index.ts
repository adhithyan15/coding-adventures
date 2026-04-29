import { BackendRegistry, type Artifact } from "@coding-adventures/codegen-core";
import { FunctionTypeStatus, IirFunction, IirInstr, IirModule, Types } from "@coding-adventures/interpreter-ir";
import { JITCore } from "@coding-adventures/jit-core";
import { VMCore, VMError, type VMValue } from "@coding-adventures/vm-core";

export class SymbolRef { constructor(readonly name: string) {} }
export type TwigExpr = number | boolean | null | SymbolRef | TwigExpr[];
const BUILTINS = new Set(["+", "-", "*", "/", "=", "<", ">", "cons", "car", "cdr", "null?", "pair?", "number?", "print"]);

export function tokenizeTwig(source: string): string[] {
  const tokens: string[] = []; let i = 0;
  while (i < source.length) {
    const c = source[i] ?? "";
    if (/\s/.test(c)) { i += 1; continue; }
    if (c === ";") { while (i < source.length && source[i] !== "\n") i += 1; continue; }
    if (c === "(" || c === ")") { tokens.push(c); i += 1; continue; }
    const s = i; while (i < source.length && !/\s/.test(source[i] ?? "") && source[i] !== "(" && source[i] !== ")") i += 1;
    tokens.push(source.slice(s, i));
  }
  return tokens;
}
export function parseTwig(source: string): TwigExpr[] { return new Parser(tokenizeTwig(source)).forms(); }
class Parser {
  private p = 0; constructor(private readonly tokens: string[]) {}
  forms(): TwigExpr[] { const forms: TwigExpr[] = []; while (this.p < this.tokens.length) forms.push(this.expr()); return forms; }
  private expr(): TwigExpr {
    const t = this.next();
    if (t === "(") { const list: TwigExpr[] = []; while (this.peek() !== ")") { if (this.peek() === undefined) throw new Error("unterminated Twig list"); list.push(this.expr()); } this.next(); return list; }
    if (t === ")") throw new Error("unexpected )");
    if (/^-?\d+$/.test(t)) return Number(t);
    if (t === "#t") return true;
    if (t === "#f") return false;
    if (t === "nil") return null;
    return new SymbolRef(t);
  }
  private next(): string { const t = this.tokens[this.p]; if (t === undefined) throw new Error("unexpected end of Twig source"); this.p += 1; return t; }
  private peek(): string | undefined { return this.tokens[this.p]; }
}

export class CompileError extends Error { constructor(message: string) { super(message); this.name = "CompileError"; } }

export function compileTwig(source: string, moduleName = "twig"): IirModule {
  const forms = parseTwig(source); const functions: IirFunction[] = []; const main = new Ctx(); const body: TwigExpr[] = [];
  for (const form of forms) {
    if (isFnDefine(form)) functions.push(compileFn(form));
    else if (isValueDefine(form)) main.emit("call_builtin", { srcs: ["global_set", symbolName(form[1]), compileExpr(form[2] ?? null, main, new Set())], typeHint: Types.Any });
    else body.push(form);
  }
  let last: string | null = null; for (const form of body) last = compileExpr(form, main, new Set());
  if (last === null) { last = main.temp(); main.emit("const", { dest: last, srcs: [null], typeHint: Types.Nil }); }
  main.emit("ret", { srcs: [last] });
  functions.push(new IirFunction({ name: "main", returnType: Types.Any, instructions: main.instructions, registerCount: main.registerCount(), typeStatus: FunctionTypeStatus.Untyped }));
  const mod = new IirModule({ name: moduleName, functions, entryPoint: "main", language: "twig" });
  mod.validate(); return mod;
}

export function runTwig(source: string, jit = false): [string, VMValue] { const r = runTwigDetailed(source, jit); return [r.stdout, r.value]; }
export function runTwigDetailed(source: string, jit = false): { stdout: string; value: VMValue; module: IirModule; vm: VMCore } {
  const module = compileTwig(source); const globals = new Map<string, VMValue>(); let stdout = ""; const vm = new VMCore();
  installTwigBuiltins(vm, globals, (text) => { stdout += text; });
  const value = jit ? new JITCore(vm).executeWithJit(module) : vm.execute(module);
  return { stdout, value, module, vm };
}
export function emitTwig(source: string, target: string): Artifact { return BackendRegistry.default().compile(compileTwig(source), target); }

export function installTwigBuiltins(vm: VMCore, globals: Map<string, VMValue>, write: (text: string) => void): void {
  vm.registerBuiltin("+", (args) => args.reduce((s, v) => toNumber(s) + toNumber(v), 0) as VMValue);
  vm.registerBuiltin("-", (args) => args.length === 1 ? -toNumber(args[0] ?? 0) : args.slice(1).reduce((l, v) => toNumber(l) - toNumber(v), toNumber(args[0] ?? 0)));
  vm.registerBuiltin("*", (args) => args.reduce((p, v) => toNumber(p) * toNumber(v), 1) as VMValue);
  vm.registerBuiltin("/", (args) => args.slice(1).reduce((l, v) => Math.trunc(toNumber(l) / toNumber(v)), toNumber(args[0] ?? 0)));
  vm.registerBuiltin("=", (args) => args[0] === args[1]);
  vm.registerBuiltin("<", (args) => toNumber(args[0] ?? 0) < toNumber(args[1] ?? 0));
  vm.registerBuiltin(">", (args) => toNumber(args[0] ?? 0) > toNumber(args[1] ?? 0));
  vm.registerBuiltin("cons", (args) => ["cons", args[0] ?? null, args[1] ?? null]);
  vm.registerBuiltin("car", (args) => asPair(args[0])[1]);
  vm.registerBuiltin("cdr", (args) => asPair(args[0])[2]);
  vm.registerBuiltin("null?", (args) => args[0] === null);
  vm.registerBuiltin("pair?", (args) => isPair(args[0]));
  vm.registerBuiltin("number?", (args) => typeof args[0] === "number");
  vm.registerBuiltin("print", (args) => { write(`${formatTwigValue(args[0] ?? null)}\n`); return null; });
  vm.registerBuiltin("global_get", (args) => { const name = String(args[0]); if (!globals.has(name)) throw new VMError(`undefined global: ${name}`); return globals.get(name) ?? null; });
  vm.registerBuiltin("global_set", (args) => { const value = args[1] ?? null; globals.set(String(args[0]), value); return value; });
  vm.registerBuiltin("_move", (args) => args[0] ?? null);
}

function compileFn(form: TwigExpr[]): IirFunction {
  const sig = form[1]; if (!Array.isArray(sig) || sig.length === 0) throw new CompileError("function define requires a signature list");
  const name = symbolName(sig[0]); const params = sig.slice(1).map(symbolName); const c = new Ctx(); const locals = new Set(params);
  let result: string | null = null; for (const expr of form.slice(2)) result = compileExpr(expr, c, locals);
  if (result === null) { result = c.temp(); c.emit("const", { dest: result, srcs: [null], typeHint: Types.Nil }); }
  c.emit("ret", { srcs: [result] });
  return new IirFunction({ name, params: params.map((p) => ({ name: p, type: Types.Any })), returnType: Types.Any, instructions: c.instructions, registerCount: c.registerCount(), typeStatus: FunctionTypeStatus.Untyped });
}
function compileExpr(expr: TwigExpr, c: Ctx, locals: Set<string>): string {
  if (expr instanceof SymbolRef) { if (locals.has(expr.name)) return expr.name; const d = c.temp(); c.emit("call_builtin", { dest: d, srcs: ["global_get", expr.name], typeHint: Types.Any }); return d; }
  if (typeof expr === "number" || typeof expr === "boolean" || expr === null) { const d = c.temp(); c.emit("const", { dest: d, srcs: [expr], typeHint: expr === null ? Types.Nil : typeof expr === "boolean" ? Types.Bool : Types.U64 }); return d; }
  if (expr.length === 0) { const d = c.temp(); c.emit("const", { dest: d, srcs: [null], typeHint: Types.Nil }); return d; }
  const head = expr[0];
  if (head instanceof SymbolRef && head.name === "if") return compileIf(expr, c, locals);
  if (head instanceof SymbolRef && head.name === "begin") return compileBegin(expr.slice(1), c, locals);
  if (head instanceof SymbolRef && head.name === "let") return compileLet(expr, c, locals);
  if (!(head instanceof SymbolRef)) throw new CompileError("Twig applications require a symbol in operator position");
  const args = expr.slice(1).map((arg) => compileExpr(arg, c, locals)); const d = c.temp();
  c.emit(BUILTINS.has(head.name) ? "call_builtin" : "call", { dest: d, srcs: [head.name, ...args], typeHint: Types.Any }); return d;
}
function compileIf(expr: TwigExpr[], c: Ctx, locals: Set<string>): string {
  const cond = compileExpr(expr[1] ?? null, c, locals); const elseLabel = c.label("else"); const end = c.label("endif"); const d = c.temp();
  c.emit("jmp_if_false", { srcs: [cond, elseLabel] }); c.emit("move", { dest: d, srcs: [compileExpr(expr[2] ?? null, c, locals)], typeHint: Types.Any }); c.emit("jmp", { srcs: [end] });
  c.emit("label", { srcs: [elseLabel] }); c.emit("move", { dest: d, srcs: [compileExpr(expr[3] ?? null, c, locals)], typeHint: Types.Any }); c.emit("label", { srcs: [end] }); return d;
}
function compileBegin(exprs: TwigExpr[], c: Ctx, locals: Set<string>): string { let r: string | null = null; for (const e of exprs) r = compileExpr(e, c, locals); if (r !== null) return r; const d = c.temp(); c.emit("const", { dest: d, srcs: [null], typeHint: Types.Nil }); return d; }
function compileLet(expr: TwigExpr[], c: Ctx, locals: Set<string>): string {
  const bindings = expr[1]; if (!Array.isArray(bindings)) throw new CompileError("let requires a binding list");
  const next = new Set(locals);
  for (const b of bindings) { if (!Array.isArray(b) || b.length !== 2) throw new CompileError("let binding must be a pair"); const name = symbolName(b[0]); c.emit("move", { dest: name, srcs: [compileExpr(b[1], c, next)], typeHint: Types.Any }); next.add(name); }
  return compileBegin(expr.slice(2), c, next);
}
function isFnDefine(expr: TwigExpr): expr is TwigExpr[] { return Array.isArray(expr) && symbolNameOrNull(expr[0]) === "define" && Array.isArray(expr[1]); }
function isValueDefine(expr: TwigExpr): expr is TwigExpr[] { return Array.isArray(expr) && symbolNameOrNull(expr[0]) === "define" && expr[1] instanceof SymbolRef; }
function symbolName(expr: TwigExpr | undefined): string { if (!(expr instanceof SymbolRef)) throw new CompileError("expected symbol"); return expr.name; }
function symbolNameOrNull(expr: TwigExpr | undefined): string | null { return expr instanceof SymbolRef ? expr.name : null; }
function toNumber(value: VMValue): number { if (typeof value !== "number") throw new VMError(`expected number, got ${formatTwigValue(value)}`); return value; }
function isPair(value: VMValue | undefined): value is readonly ["cons", VMValue, VMValue] { return Array.isArray(value) && value[0] === "cons" && value.length === 3; }
function asPair(value: VMValue | undefined): readonly ["cons", VMValue, VMValue] { if (!isPair(value)) throw new VMError(`expected pair, got ${formatTwigValue(value ?? null)}`); return value; }
export function formatTwigValue(value: VMValue): string { if (value === null) return "nil"; if (value === true) return "#t"; if (value === false) return "#f"; if (isPair(value)) return `(${formatTwigValue(value[1])} . ${formatTwigValue(value[2])})`; return String(value); }
class Ctx { readonly instructions: IirInstr[] = []; private n = 0; private labels = 0; temp(): string { const t = `t${this.n}`; this.n += 1; return t; } label(prefix: string): string { const l = `${prefix}_${this.labels}`; this.labels += 1; return l; } emit(op: string, options: Omit<ConstructorParameters<typeof IirInstr>[0], "op"> = {}): void { this.instructions.push(IirInstr.of(op, options)); } registerCount(): number { return Math.max(64, this.n); } }
