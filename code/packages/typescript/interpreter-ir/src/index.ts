export type LangType = "u8" | "u16" | "u32" | "u64" | "bool" | "str" | "nil" | "void" | "any" | "polymorphic" | (string & {});

export const Types = {
  U8: "u8",
  U16: "u16",
  U32: "u32",
  U64: "u64",
  Bool: "bool",
  Str: "str",
  Nil: "nil",
  Void: "void",
  Any: "any",
  Polymorphic: "polymorphic",
  isRef(type: string): boolean {
    return type.startsWith("ref<") && type.endsWith(">");
  },
  unwrapRef(type: string): string {
    return this.isRef(type) ? type.slice(4, -1) : type;
  },
  ref(type: string): string {
    return `ref<${type}>`;
  },
  isConcrete(type: string | null | undefined): boolean {
    return type !== null && type !== undefined && type !== this.Any && type !== this.Polymorphic;
  },
} as const;

export const Opcodes = {
  Arithmetic: ["add", "sub", "mul", "div", "mod", "neg"] as const,
  Bitwise: ["and", "or", "xor", "not", "shl", "shr"] as const,
  Cmp: ["cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge"] as const,
  Branch: ["jmp", "jmp_if_true", "jmp_if_false"] as const,
  Control: ["label", "ret", "ret_void"] as const,
  Memory: ["load_reg", "store_reg", "load_mem", "store_mem"] as const,
  Call: ["call", "call_builtin"] as const,
  Io: ["io_in", "io_out"] as const,
  Coercion: ["cast", "type_assert"] as const,
  Heap: ["alloc", "box", "unbox", "field_load", "field_store", "is_null", "safepoint"] as const,
};

export const ValueOpcodes: readonly string[] = [
  ...Opcodes.Arithmetic,
  ...Opcodes.Bitwise,
  ...Opcodes.Cmp,
  "const",
  "load_reg",
  "load_mem",
  "call",
  "call_builtin",
  "io_in",
  "cast",
  "alloc",
  "box",
  "unbox",
  "field_load",
  "is_null",
  "tetrad.move",
  "move",
];

export const SideEffectOpcodes: readonly string[] = [
  ...Opcodes.Branch,
  ...Opcodes.Control,
  "store_reg",
  "store_mem",
  "io_out",
  "type_assert",
  "field_store",
  "safepoint",
];

export const AllOpcodes: readonly string[] = [
  ...new Set([
    ...ValueOpcodes,
    ...SideEffectOpcodes,
    ...Opcodes.Memory,
    ...Opcodes.Call,
    ...Opcodes.Io,
    ...Opcodes.Coercion,
    ...Opcodes.Heap,
  ]),
];

export enum SlotKind {
  Uninitialized = "uninitialized",
  Monomorphic = "monomorphic",
  Polymorphic = "polymorphic",
  Megamorphic = "megamorphic",
}

export class SlotState {
  readonly observations = new Map<string, number>();
  kind = SlotKind.Uninitialized;
  count = 0;

  record(runtimeType: string): this {
    this.count += 1;
    this.observations.set(runtimeType, (this.observations.get(runtimeType) ?? 0) + 1);
    const unique = this.observations.size;
    this.kind = unique === 1 ? SlotKind.Monomorphic : unique <= 4 ? SlotKind.Polymorphic : SlotKind.Megamorphic;
    return this;
  }

  get observedTypes(): string[] {
    return [...this.observations.keys()];
  }

  isMonomorphic(): boolean {
    return this.kind === SlotKind.Monomorphic;
  }

  isPolymorphic(): boolean {
    return this.kind === SlotKind.Polymorphic;
  }
}

export type IirValue = string | number | boolean | null | readonly IirValue[] | { readonly [key: string]: unknown };

export interface IirInstrOptions {
  op: string;
  dest?: string | null;
  srcs?: IirValue[];
  typeHint?: string | null;
  observedType?: string | null;
  observationCount?: number;
  observedSlot?: SlotState | null;
  deoptAnchor?: string | null;
  mayAlloc?: boolean;
}

export class IirInstr {
  op: string;
  dest: string | null;
  srcs: IirValue[];
  typeHint: string | null;
  observedType: string | null;
  observationCount: number;
  observedSlot: SlotState | null;
  deoptAnchor: string | null;
  mayAlloc: boolean;

  constructor(options: IirInstrOptions) {
    this.op = options.op;
    this.dest = options.dest ?? null;
    this.srcs = options.srcs ?? [];
    this.typeHint = options.typeHint ?? null;
    this.observedType = options.observedType ?? null;
    this.observationCount = options.observationCount ?? 0;
    this.observedSlot = options.observedSlot ?? null;
    this.deoptAnchor = options.deoptAnchor ?? null;
    this.mayAlloc = options.mayAlloc ?? false;
  }

  static of(op: string, options: Omit<IirInstrOptions, "op"> = {}): IirInstr {
    return new IirInstr({ op, ...options });
  }

  get typed(): boolean {
    return Types.isConcrete(this.typeHint);
  }

  get hasObservation(): boolean {
    return this.observedType !== null || this.observationCount > 0 || this.observedSlot !== null;
  }

  get polymorphic(): boolean {
    return this.observedSlot?.isPolymorphic() ?? false;
  }

  get effectiveType(): string | null {
    return this.typeHint ?? this.observedType;
  }

  recordObservation(runtimeType: string, slot?: SlotState): this {
    this.observedType = runtimeType;
    this.observationCount += 1;
    if (slot !== undefined) {
      this.observedSlot = slot.record(runtimeType);
    }
    return this;
  }

  toString(): string {
    const dest = this.dest === null ? "" : `${this.dest} = `;
    const args = this.srcs.map((src) => JSON.stringify(src)).join(", ");
    const type = this.effectiveType === null ? "" : ` : ${this.effectiveType}`;
    return `${dest}${this.op}(${args})${type}`;
  }
}

export interface IirParam {
  name: string;
  type: string;
}

export enum FunctionTypeStatus {
  FullyTyped = "fully_typed",
  PartiallyTyped = "partially_typed",
  Untyped = "untyped",
}

export interface IirFunctionOptions {
  name: string;
  params?: IirParam[];
  returnType?: string;
  instructions?: IirInstr[];
  registerCount?: number;
  typeStatus?: FunctionTypeStatus;
  callCount?: number;
  feedbackSlots?: Map<string, SlotState>;
  sourceMap?: Map<number, string>;
}

export class IirFunction {
  name: string;
  params: IirParam[];
  returnType: string;
  instructions: IirInstr[];
  registerCount: number;
  typeStatus: FunctionTypeStatus;
  callCount: number;
  feedbackSlots: Map<string, SlotState>;
  sourceMap: Map<number, string>;

  constructor(options: IirFunctionOptions) {
    this.name = options.name;
    this.params = options.params ?? [];
    this.returnType = options.returnType ?? Types.Any;
    this.instructions = options.instructions ?? [];
    this.registerCount = options.registerCount ?? 0;
    this.typeStatus = options.typeStatus ?? this.inferTypeStatus();
    this.callCount = options.callCount ?? 0;
    this.feedbackSlots = options.feedbackSlots ?? new Map();
    this.sourceMap = options.sourceMap ?? new Map();
  }

  paramNames(): string[] {
    return this.params.map((param) => param.name);
  }

  paramTypes(): string[] {
    return this.params.map((param) => param.type);
  }

  inferTypeStatus(): FunctionTypeStatus {
    const signatureTyped = this.params.every((param) => Types.isConcrete(param.type)) && Types.isConcrete(this.returnType);
    const values = this.instructions.filter((instr) => ValueOpcodes.includes(instr.op));
    const typedValues = values.filter((instr) => instr.typed).length;
    if (signatureTyped && typedValues === values.length) {
      return FunctionTypeStatus.FullyTyped;
    }
    if (signatureTyped || typedValues > 0) {
      return FunctionTypeStatus.PartiallyTyped;
    }
    return FunctionTypeStatus.Untyped;
  }

  labelIndex(): Map<string, number> {
    const labels = new Map<string, number>();
    this.instructions.forEach((instr, index) => {
      if (instr.op === "label") {
        const label = String(instr.srcs[0] ?? instr.dest ?? "");
        if (label.length > 0) {
          labels.set(label, index);
        }
      }
    });
    return labels;
  }
}

export interface IirModuleOptions {
  name: string;
  functions?: IirFunction[];
  entryPoint?: string;
  language?: string;
  metadata?: Record<string, unknown>;
}

export class IirModule {
  name: string;
  functions: IirFunction[];
  entryPoint: string;
  language: string;
  metadata: Record<string, unknown>;

  constructor(options: IirModuleOptions) {
    this.name = options.name;
    this.functions = options.functions ?? [];
    this.entryPoint = options.entryPoint ?? "main";
    this.language = options.language ?? "unknown";
    this.metadata = options.metadata ?? {};
  }

  getFunction(name: string): IirFunction | undefined {
    return this.functions.find((fn) => fn.name === name);
  }

  functionNames(): string[] {
    return this.functions.map((fn) => fn.name);
  }

  addOrReplace(fn: IirFunction): void {
    const index = this.functions.findIndex((existing) => existing.name === fn.name);
    if (index >= 0) {
      this.functions[index] = fn;
    } else {
      this.functions.push(fn);
    }
  }

  validate(): void {
    const seen = new Set<string>();
    for (const fn of this.functions) {
      if (seen.has(fn.name)) {
        throw new Error(`duplicate function: ${fn.name}`);
      }
      seen.add(fn.name);
    }
    if (!seen.has(this.entryPoint)) {
      throw new Error(`missing entry point: ${this.entryPoint}`);
    }
    for (const fn of this.functions) {
      const labels = fn.labelIndex();
      fn.instructions.forEach((instr, index) => {
        if (Opcodes.Branch.includes(instr.op as never)) {
          const label = instr.op === "jmp" ? String(instr.srcs[0] ?? "") : String(instr.srcs[1] ?? "");
          if (!labels.has(label)) {
            throw new Error(`${fn.name}:${index} branches to undefined label ${label}`);
          }
        }
      });
    }
  }
}

export { IirInstr as IIRInstr, IirFunction as IIRFunction, IirModule as IIRModule };
