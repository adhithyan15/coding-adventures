import { FunctionTypeStatus, IirFunction, IirInstr, IirModule, Types } from "@coding-adventures/interpreter-ir";
import { JITCore } from "@coding-adventures/jit-core";
import { VMCore } from "@coding-adventures/vm-core";

import { TranslationError } from "./translator.js";

export interface BrainfuckLangVmResult {
  readonly output: string;
  readonly memory: ReadonlyMap<number, unknown>;
  readonly vm: VMCore;
  readonly module: IirModule;
}

export function compileToIir(source: string, moduleName = "brainfuck"): IirModule {
  const instructions: IirInstr[] = [IirInstr.of("const", { dest: "ptr", srcs: [0], typeHint: Types.U32 })];
  const loops: Array<{ start: string; end: string }> = [];
  let loopId = 0;
  for (const char of source) {
    switch (char) {
      case ">": instructions.push(IirInstr.of("add", { dest: "ptr", srcs: ["ptr", 1], typeHint: Types.U32 })); break;
      case "<": instructions.push(IirInstr.of("sub", { dest: "ptr", srcs: ["ptr", 1], typeHint: Types.U32 })); break;
      case "+": mutateCell(instructions, "add", 1); break;
      case "-": mutateCell(instructions, "sub", 1); break;
      case ".": instructions.push(IirInstr.of("load_mem", { dest: "cell", srcs: ["ptr"], typeHint: Types.U8 }), IirInstr.of("io_out", { srcs: ["cell"] })); break;
      case ",": instructions.push(IirInstr.of("io_in", { dest: "cell", typeHint: Types.U8 }), IirInstr.of("store_mem", { srcs: ["ptr", "cell"], typeHint: Types.U8 })); break;
      case "[": {
        const labels = { start: `loop_${loopId}_start`, end: `loop_${loopId}_end` }; loopId += 1; loops.push(labels);
        instructions.push(IirInstr.of("label", { srcs: [labels.start] }), IirInstr.of("load_mem", { dest: "cell", srcs: ["ptr"], typeHint: Types.U8 }), IirInstr.of("cmp_eq", { dest: "is_zero", srcs: ["cell", 0], typeHint: Types.Bool }), IirInstr.of("jmp_if_true", { srcs: ["is_zero", labels.end] }));
        break;
      }
      case "]": {
        const labels = loops.pop(); if (labels === undefined) throw new TranslationError("Unmatched ']' -- no matching '[' found");
        instructions.push(IirInstr.of("jmp", { srcs: [labels.start] }), IirInstr.of("label", { srcs: [labels.end] })); break;
      }
      default: break;
    }
  }
  if (loops.length > 0) throw new TranslationError(`Unmatched '[' -- ${loops.length} unclosed bracket(s)`);
  instructions.push(IirInstr.of("ret_void"));
  const mod = new IirModule({ name: moduleName, functions: [new IirFunction({ name: "main", returnType: Types.Void, instructions, registerCount: 8, typeStatus: FunctionTypeStatus.PartiallyTyped })], entryPoint: "main", language: "brainfuck" });
  mod.validate(); return mod;
}

export function executeOnLangVm(source: string, input = "", jit = false): BrainfuckLangVmResult {
  const module = compileToIir(source); const vm = new VMCore({ input, u8Wrap: true });
  jit ? new JITCore(vm).executeWithJit(module) : vm.execute(module);
  return { output: vm.output, memory: vm.memory, vm, module };
}

function mutateCell(instructions: IirInstr[], op: "add" | "sub", amount: number): void {
  instructions.push(IirInstr.of("load_mem", { dest: "cell", srcs: ["ptr"], typeHint: Types.U8 }), IirInstr.of(op, { dest: "cell", srcs: ["cell", amount], typeHint: Types.U8 }), IirInstr.of("store_mem", { srcs: ["ptr", "cell"], typeHint: Types.U8 }));
}
