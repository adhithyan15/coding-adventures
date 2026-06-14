import { IirFunction, IirInstr, IirModule, Types } from "@coding-adventures/interpreter-ir";
import type { IirValue } from "@coding-adventures/interpreter-ir";

export type VMValue = string | number | boolean | null | readonly VMValue[] | { readonly [key: string]: unknown };

export class VMError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "VMError";
  }
}

export class UnknownOpcodeError extends VMError {
  constructor(op: string) {
    super(`unknown opcode: ${op}`);
    this.name = "UnknownOpcodeError";
  }
}

export class FrameOverflowError extends VMError {
  constructor(maxFrames: number) {
    super(`maximum frame depth exceeded: ${maxFrames}`);
    this.name = "FrameOverflowError";
  }
}

export class VMInterrupt extends VMError {
  constructor() {
    super("VM interrupted");
    this.name = "VMInterrupt";
  }
}

export type BuiltinHandler = (args: VMValue[]) => VMValue;

export class BuiltinRegistry {
  private readonly handlers = new Map<string, BuiltinHandler>();

  constructor(registerDefaults = true) {
    if (registerDefaults) {
      this.register("noop", () => null);
      this.register("assert_eq", (args) => {
        if (args[0] !== args[1]) {
          throw new VMError(`assert_eq failed: ${String(args[0])} != ${String(args[1])}`);
        }
        return null;
      });
    }
  }

  register(name: string, handler: BuiltinHandler): void {
    this.handlers.set(name, handler);
  }

  call(name: string, args: VMValue[]): VMValue {
    const handler = this.handlers.get(name);
    if (handler === undefined) {
      throw new VMError(`unknown builtin: ${name}`);
    }
    return handler(args);
  }

  names(): string[] {
    return [...this.handlers.keys()];
  }

  entries(): [string, BuiltinHandler][] {
    return [...this.handlers.entries()];
  }
}

export class VMFrame {
  readonly fn: IirFunction;
  ip = 0;
  readonly registers = new Map<string, VMValue>();
  readonly slots = new Map<string, VMValue>();

  constructor(fn: IirFunction, args: VMValue[] = []) {
    this.fn = fn;
    fn.params.forEach((param, index) => {
      const value = args[index] ?? null;
      this.registers.set(param.name, value);
      this.slots.set(param.name, value);
    });
  }

  resolve(value: IirValue | VMValue | undefined): VMValue {
    if (value === undefined) return null;
    if (typeof value === "string" && this.registers.has(value)) return this.registers.get(value) ?? null;
    if (Array.isArray(value)) return value.map((entry) => this.resolve(entry)) as VMValue[];
    return value as VMValue;
  }

  write(name: string | null, value: VMValue): void {
    if (name !== null) this.registers.set(name, value);
  }

  loadSlot(name: string): VMValue {
    return this.slots.get(name) ?? null;
  }

  storeSlot(name: string, value: VMValue): void {
    this.slots.set(name, value);
  }
}

export class BranchStats {
  takenCount = 0;
  notTakenCount = 0;
  record(taken: boolean): void {
    if (taken) this.takenCount += 1;
    else this.notTakenCount += 1;
  }
}

export class VMMetrics {
  readonly functionCallCounts = new Map<string, number>();
  totalInstructionsExecuted = 0;
  totalFramesPushed = 0;
  totalJitHits = 0;
  readonly branchStats = new Map<string, BranchStats>();
  readonly loopBackEdgeCounts = new Map<string, number>();
}

export interface VMCoreOptions {
  maxFrames?: number;
  builtins?: BuiltinRegistry;
  profilerEnabled?: boolean;
  u8Wrap?: boolean;
  input?: string;
}

export type JitHandler = (args: VMValue[]) => VMValue;
export interface TraceEvent { functionName: string; ip: number; instruction: string }

export class VMCore {
  readonly builtins: BuiltinRegistry;
  readonly memory = new Map<number, VMValue>();
  readonly ioPorts = new Map<string, VMValue>();
  output = "";
  private readonly maxFrames: number;
  private readonly profilerEnabled: boolean;
  private readonly u8Wrap: boolean;
  private readonly frames: VMFrame[] = [];
  private readonly jitHandlers = new Map<string, JitHandler>();
  private readonly metricData = new VMMetrics();
  private readonly coverage = new Map<string, Set<number>>();
  private module: IirModule | null = null;
  private interrupted = false;
  private inputBuffer: number[];
  private coverageEnabled = false;
  private trace: TraceEvent[] | null = null;

  constructor(options: VMCoreOptions = {}) {
    this.maxFrames = options.maxFrames ?? 64;
    this.builtins = options.builtins ?? new BuiltinRegistry();
    this.profilerEnabled = options.profilerEnabled ?? true;
    this.u8Wrap = options.u8Wrap ?? false;
    this.inputBuffer = [...(options.input ?? "")].map((char) => char.charCodeAt(0) & 0xff);
  }

  execute(mod: IirModule, fn = mod.entryPoint, args: VMValue[] = []): VMValue {
    mod.validate();
    this.module = mod;
    this.interrupted = false;
    return this.invokeFunction(fn, args);
  }

  executeTraced(mod: IirModule, fn = mod.entryPoint, args: VMValue[] = []): { result: VMValue; trace: TraceEvent[] } {
    this.trace = [];
    try {
      const result = this.execute(mod, fn, args);
      return { result, trace: this.trace };
    } finally {
      this.trace = null;
    }
  }

  metrics(): VMMetrics { return this.metricData; }
  resetMetrics(): void {
    this.metricData.functionCallCounts.clear();
    this.metricData.totalInstructionsExecuted = 0;
    this.metricData.totalFramesPushed = 0;
    this.metricData.totalJitHits = 0;
    this.metricData.branchStats.clear();
    this.metricData.loopBackEdgeCounts.clear();
  }
  registerBuiltin(name: string, handler: BuiltinHandler): void { this.builtins.register(name, handler); }
  registerJitHandler(name: string, handler: JitHandler): void { this.jitHandlers.set(name, handler); }
  unregisterJitHandler(name: string): void { this.jitHandlers.delete(name); }
  hotFunctions(minCalls = 1): string[] {
    return [...this.metricData.functionCallCounts.entries()].filter(([, calls]) => calls >= minCalls).sort((a, b) => b[1] - a[1]).map(([name]) => name);
  }
  branchProfile(functionName: string, ip: number): BranchStats | undefined { return this.metricData.branchStats.get(`${functionName}:${ip}`); }
  loopIterations(functionName: string, label: string): number { return this.metricData.loopBackEdgeCounts.get(`${functionName}:${label}`) ?? 0; }
  enableCoverage(): void { this.coverageEnabled = true; }
  disableCoverage(): void { this.coverageEnabled = false; }
  coverageData(): Map<string, number[]> { return new Map([...this.coverage.entries()].map(([fn, ips]) => [fn, [...ips].sort((a, b) => a - b)])); }
  resetCoverage(): void { this.coverage.clear(); }
  interrupt(): void { this.interrupted = true; }

  private invokeFunction(name: string, args: VMValue[]): VMValue {
    const mod = this.module;
    if (mod === null) throw new VMError("no module loaded");
    const fn = mod.getFunction(name);
    if (fn === undefined) throw new VMError(`unknown function: ${name}`);
    fn.callCount += 1;
    this.recordFunctionCall(name);
    const jitHandler = this.jitHandlers.get(name);
    if (jitHandler !== undefined) {
      this.metricData.totalJitHits += 1;
      return jitHandler(args);
    }
    if (this.frames.length >= this.maxFrames) throw new FrameOverflowError(this.maxFrames);
    const frame = new VMFrame(fn, args);
    this.frames.push(frame);
    this.metricData.totalFramesPushed += 1;
    try {
      return this.runFrame(frame);
    } finally {
      this.frames.pop();
    }
  }

  private runFrame(frame: VMFrame): VMValue {
    const labels = frame.fn.labelIndex();
    while (frame.ip < frame.fn.instructions.length) {
      if (this.interrupted) throw new VMInterrupt();
      const instr = frame.fn.instructions[frame.ip];
      this.recordInstruction(frame.fn.name, frame.ip, instr);
      const result = this.dispatch(frame, instr, labels);
      if (result.kind === "return") return result.value;
      frame.ip = result.kind === "jump" ? result.ip : frame.ip + 1;
    }
    return null;
  }

  private dispatch(frame: VMFrame, instr: IirInstr, labels: Map<string, number>): { kind: "next" } | { kind: "jump"; ip: number } | { kind: "return"; value: VMValue } {
    switch (instr.op) {
      case "const":
      case "move":
      case "tetrad.move":
        this.writeObserved(frame, instr, frame.resolve(instr.srcs[0])); return { kind: "next" };
      case "add": case "sub": case "mul": case "div": case "mod": case "and": case "or": case "xor": case "shl": case "shr": case "cmp_eq": case "cmp_ne": case "cmp_lt": case "cmp_le": case "cmp_gt": case "cmp_ge":
        this.writeObserved(frame, instr, this.binaryOp(instr.op, frame.resolve(instr.srcs[0]), frame.resolve(instr.srcs[1]))); return { kind: "next" };
      case "neg":
        this.writeObserved(frame, instr, -this.toNumber(frame.resolve(instr.srcs[0]))); return { kind: "next" };
      case "not":
        this.writeObserved(frame, instr, ~this.toNumber(frame.resolve(instr.srcs[0]))); return { kind: "next" };
      case "cast":
        this.writeObserved(frame, instr, this.cast(frame.resolve(instr.srcs[0]), instr.typeHint ?? String(instr.srcs[1] ?? Types.Any))); return { kind: "next" };
      case "type_assert":
        this.assertType(frame.resolve(instr.srcs[0]), instr.typeHint ?? String(instr.srcs[1] ?? Types.Any)); return { kind: "next" };
      case "label":
        return { kind: "next" };
      case "jmp":
        return { kind: "jump", ip: this.jumpTarget(frame, labels, String(instr.srcs[0] ?? "")) };
      case "jmp_if_true": {
        const taken = this.truthy(frame.resolve(instr.srcs[0]));
        this.recordBranch(frame.fn.name, frame.ip, taken);
        return taken ? { kind: "jump", ip: this.jumpTarget(frame, labels, String(instr.srcs[1] ?? "")) } : { kind: "next" };
      }
      case "jmp_if_false": {
        const taken = !this.truthy(frame.resolve(instr.srcs[0]));
        this.recordBranch(frame.fn.name, frame.ip, taken);
        return taken ? { kind: "jump", ip: this.jumpTarget(frame, labels, String(instr.srcs[1] ?? "")) } : { kind: "next" };
      }
      case "ret":
        return { kind: "return", value: frame.resolve(instr.srcs[0]) };
      case "ret_void":
        return { kind: "return", value: null };
      case "call": {
        const args = instr.srcs.slice(1).map((src) => frame.resolve(src));
        this.writeObserved(frame, instr, this.invokeFunction(String(instr.srcs[0] ?? ""), args)); return { kind: "next" };
      }
      case "call_builtin": {
        const args = instr.srcs.slice(1).map((src) => frame.resolve(src));
        this.writeObserved(frame, instr, this.builtins.call(String(instr.srcs[0] ?? ""), args)); return { kind: "next" };
      }
      case "load_reg":
        this.writeObserved(frame, instr, frame.resolve(instr.srcs[0])); return { kind: "next" };
      case "store_reg": {
        const target = instr.dest ?? String(instr.srcs[0] ?? "");
        frame.write(target, instr.dest === null ? frame.resolve(instr.srcs[1]) : frame.resolve(instr.srcs[0])); return { kind: "next" };
      }
      case "load_mem":
        this.writeObserved(frame, instr, this.memory.get(this.toNumber(frame.resolve(instr.srcs[0]))) ?? 0); return { kind: "next" };
      case "store_mem":
        this.memory.set(this.toNumber(frame.resolve(instr.srcs[0])), this.wrapValue(frame.resolve(instr.srcs[1]), instr.typeHint)); return { kind: "next" };
      case "io_in":
        this.writeObserved(frame, instr, this.inputBuffer.shift() ?? 0); return { kind: "next" };
      case "io_out": {
        const value = frame.resolve(instr.srcs[0]);
        this.output += typeof value === "string" ? value : String.fromCharCode(this.toNumber(value) & 0xff); return { kind: "next" };
      }
      case "is_null":
        this.writeObserved(frame, instr, frame.resolve(instr.srcs[0]) === null); return { kind: "next" };
      case "safepoint":
        return { kind: "next" };
      default:
        throw new UnknownOpcodeError(instr.op);
    }
  }

  private recordInstruction(functionName: string, ip: number, instr: IirInstr): void {
    this.metricData.totalInstructionsExecuted += 1;
    if (this.coverageEnabled) {
      const ips = this.coverage.get(functionName) ?? new Set<number>();
      ips.add(ip);
      this.coverage.set(functionName, ips);
    }
    this.trace?.push({ functionName, ip, instruction: instr.toString() });
  }
  private recordFunctionCall(name: string): void {
    if (this.profilerEnabled) this.metricData.functionCallCounts.set(name, (this.metricData.functionCallCounts.get(name) ?? 0) + 1);
  }
  private recordBranch(functionName: string, ip: number, taken: boolean): void {
    const key = `${functionName}:${ip}`;
    const stats = this.metricData.branchStats.get(key) ?? new BranchStats();
    stats.record(taken);
    this.metricData.branchStats.set(key, stats);
  }
  private jumpTarget(frame: VMFrame, labels: Map<string, number>, label: string): number {
    const target = labels.get(label);
    if (target === undefined) throw new VMError(`${frame.fn.name} branches to undefined label ${label}`);
    if (target < frame.ip) {
      const key = `${frame.fn.name}:${label}`;
      this.metricData.loopBackEdgeCounts.set(key, (this.metricData.loopBackEdgeCounts.get(key) ?? 0) + 1);
    }
    return target;
  }
  private writeObserved(frame: VMFrame, instr: IirInstr, value: VMValue): void {
    const wrapped = this.wrapValue(value, instr.typeHint);
    frame.write(instr.dest, wrapped);
    instr.recordObservation(instr.typeHint ?? this.runtimeType(wrapped));
  }
  private binaryOp(op: string, left: VMValue, right: VMValue): VMValue {
    switch (op) {
      case "add": return this.toNumber(left) + this.toNumber(right);
      case "sub": return this.toNumber(left) - this.toNumber(right);
      case "mul": return this.toNumber(left) * this.toNumber(right);
      case "div": return Math.trunc(this.toNumber(left) / this.toNumber(right));
      case "mod": return this.toNumber(left) % this.toNumber(right);
      case "and": return this.toNumber(left) & this.toNumber(right);
      case "or": return this.toNumber(left) | this.toNumber(right);
      case "xor": return this.toNumber(left) ^ this.toNumber(right);
      case "shl": return this.toNumber(left) << this.toNumber(right);
      case "shr": return this.toNumber(left) >>> this.toNumber(right);
      case "cmp_eq": return left === right;
      case "cmp_ne": return left !== right;
      case "cmp_lt": return this.toNumber(left) < this.toNumber(right);
      case "cmp_le": return this.toNumber(left) <= this.toNumber(right);
      case "cmp_gt": return this.toNumber(left) > this.toNumber(right);
      case "cmp_ge": return this.toNumber(left) >= this.toNumber(right);
      default: throw new UnknownOpcodeError(op);
    }
  }
  private cast(value: VMValue, type: string): VMValue {
    switch (type) {
      case Types.U8: return this.toNumber(value) & 0xff;
      case Types.U16: return this.toNumber(value) & 0xffff;
      case Types.U32: return this.toNumber(value) >>> 0;
      case Types.Bool: return this.truthy(value);
      case Types.Str: return String(value);
      case Types.Nil: return null;
      default: return value;
    }
  }
  private assertType(value: VMValue, type: string): void {
    if (type !== Types.Any && this.runtimeType(value) !== type) throw new VMError(`type assertion failed: expected ${type}, got ${this.runtimeType(value)}`);
  }
  private runtimeType(value: VMValue): string {
    if (value === null) return Types.Nil;
    if (typeof value === "boolean") return Types.Bool;
    if (typeof value === "string") return Types.Str;
    if (typeof value === "number") return Number.isInteger(value) && value >= 0 && value <= 0xff ? Types.U8 : Types.U64;
    return Types.Any;
  }
  private wrapValue(value: VMValue, typeHint: string | null): VMValue {
    return this.u8Wrap && typeHint === Types.U8 && typeof value === "number" ? value & 0xff : value;
  }
  private truthy(value: VMValue): boolean { return !(value === null || value === false || value === 0); }
  private toNumber(value: VMValue): number {
    if (typeof value === "number") return value;
    if (typeof value === "boolean") return value ? 1 : 0;
    if (typeof value === "string") {
      const parsed = Number(value);
      if (!Number.isNaN(parsed)) return parsed;
    }
    throw new VMError(`expected number, got ${String(value)}`);
  }
}
