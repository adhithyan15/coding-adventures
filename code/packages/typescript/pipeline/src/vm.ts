/**
 * Virtual Machine — A General-Purpose Stack-Based Bytecode Interpreter.
 * ======================================================================
 *
 * Chapter 1: What Is a Virtual Machine?
 * ======================================================================
 *
 * Imagine you've written a program in Python, Ruby, or some custom language you
 * invented. Before your computer can actually *run* that program, it needs to be
 * translated into something a processor understands. But real CPUs are messy —
 * there are dozens of different architectures (x86, ARM, RISC-V), each with its
 * own instruction set.
 *
 * A **virtual machine** (VM) solves this by providing a *fake* processor that
 * runs everywhere. Instead of compiling your language to x86 or ARM, you compile
 * it to the VM's **bytecode** — a simple, portable instruction set. Then the VM
 * interprets that bytecode on whatever real hardware you happen to have.
 *
 * This is exactly how Java works:
 *     Java source -> javac -> .class file (bytecode) -> JVM interprets it
 *
 * And how .NET works:
 *     C# source -> csc -> .dll (CIL bytecode) -> CLR interprets/JITs it
 *
 * Chapter 2: Stack-Based Architecture
 * ======================================================================
 *
 * Our VM uses a **stack-based** architecture (like the JVM, .NET CLR, and
 * Python's CPython). All operations work by pushing values onto a stack
 * and popping them off:
 *
 *     To compute 1 + 2:
 *         LOAD_CONST 1    // stack: [1]
 *         LOAD_CONST 2    // stack: [1, 2]
 *         ADD             // pops 2 and 1, pushes 3 -> stack: [3]
 *
 * The alternative is a **register-based** architecture (like Lua's VM or
 * real CPUs), where operations specify source and destination registers:
 *
 *     ADD R1, R2, R3   -> "Put R2 + R3 into R1"
 *
 * Stack machines are simpler to implement (no register allocation needed)
 * and produce more compact bytecode, which is why they're more common in
 * educational and production VMs alike.
 */

import type { CodeObject, Instruction, VMTrace } from "./vm-types.js";
import { OpCode, OpCodeName } from "./vm-types.js";

// ===========================================================================
// VirtualMachine
// ===========================================================================

/**
 * A stack-based virtual machine that executes CodeObjects.
 *
 * The VM is the final stage of the pipeline. It takes the bytecode produced
 * by the compiler and actually *runs* it — reading instructions one by one,
 * manipulating the stack, and storing variable values.
 *
 * State:
 * - **stack**: The operand stack. All computations happen here.
 * - **variables**: Named variable storage (like global variables).
 * - **output**: Captured print output from PRINT instructions.
 * - **pc**: The program counter — which instruction we're about to execute.
 *
 * The VM records a `VMTrace` after each instruction for visualization.
 *
 * @example
 *     const vm = new VirtualMachine();
 *     const traces = vm.execute(codeObject);
 *     console.log(vm.variables); // { x: 3 }
 */
export class VirtualMachine {
  /** The operand stack — all computations happen here. */
  private stack: unknown[] = [];

  /** Named variable storage — maps variable names to their current values. */
  variables: Record<string, unknown> = {};

  /** Captured print output from PRINT instructions. */
  output: string[] = [];

  /** The program counter — index of the next instruction to execute. */
  private pc = 0;

  // -----------------------------------------------------------------
  // Public API
  // -----------------------------------------------------------------

  /**
   * Execute a CodeObject and return a list of execution traces.
   *
   * This is the main entry point. It runs the bytecode instruction by
   * instruction until it hits a HALT instruction. At each step, it
   * captures a VMTrace snapshot for the visualizer.
   *
   * @param code - The compiled bytecode to execute.
   * @returns An array of VMTrace snapshots, one per executed instruction.
   *
   * @example
   *     const vm = new VirtualMachine();
   *     const traces = vm.execute(code);
   *     // traces[0] shows what happened when the first instruction ran
   */
  execute(code: CodeObject): VMTrace[] {
    const traces: VMTrace[] = [];
    this.pc = 0;

    // The fetch-decode-execute cycle — the fundamental loop of every
    // processor, real or virtual. We fetch the next instruction, decode
    // what it means, execute it, and repeat until we hit HALT.
    while (this.pc < code.instructions.length) {
      const instr = code.instructions[this.pc];
      const currentPc = this.pc;

      // Capture the stack state BEFORE execution for the trace.
      const stackBefore = [...this.stack];

      // Execute the instruction (this may modify stack, variables, pc).
      const description = this.executeInstruction(instr, code);

      // If we just executed HALT, record the final trace and stop.
      if (instr.opcode === OpCode.HALT) {
        traces.push({
          pc: currentPc,
          instruction: instr,
          stackBefore,
          stackAfter: [...this.stack],
          variables: { ...this.variables },
          description,
        });
        break;
      }

      // Record the trace snapshot after execution.
      traces.push({
        pc: currentPc,
        instruction: instr,
        stackBefore,
        stackAfter: [...this.stack],
        variables: { ...this.variables },
        description,
      });
    }

    return traces;
  }

  // -----------------------------------------------------------------
  // Instruction execution — the decode/execute phase
  // -----------------------------------------------------------------

  /**
   * Execute a single instruction and return a human-readable description.
   *
   * This is the "decode and execute" phase of the fetch-decode-execute cycle.
   * Based on the opcode, we perform the appropriate operation on the stack
   * and/or variables.
   *
   * @param instr - The instruction to execute.
   * @param code - The CodeObject (needed for constant/name pool lookups).
   * @returns A human-readable description of what happened.
   */
  private executeInstruction(instr: Instruction, code: CodeObject): string {
    switch (instr.opcode) {
      // -- LOAD_CONST: Push a constant onto the stack ----------------------
      case OpCode.LOAD_CONST: {
        const value = code.constants[instr.operand!];
        this.stack.push(value);
        this.pc++;
        return `Push constant ${JSON.stringify(value)} onto the stack`;
      }

      // -- POP: Discard the top of stack -----------------------------------
      case OpCode.POP: {
        const value = this.stack.pop();
        this.pc++;
        return `Pop ${JSON.stringify(value)} from the stack`;
      }

      // -- DUP: Duplicate the top of stack ---------------------------------
      case OpCode.DUP: {
        const value = this.stack[this.stack.length - 1];
        this.stack.push(value);
        this.pc++;
        return `Duplicate top of stack: ${JSON.stringify(value)}`;
      }

      // -- STORE_NAME: Pop and store in a named variable -------------------
      case OpCode.STORE_NAME: {
        const name = code.names[instr.operand!];
        const value = this.stack.pop();
        this.variables[name] = value;
        this.pc++;
        return `Store ${JSON.stringify(value)} into variable '${name}'`;
      }

      // -- LOAD_NAME: Push the value of a named variable -------------------
      case OpCode.LOAD_NAME: {
        const name = code.names[instr.operand!];
        const value = this.variables[name];
        if (value === undefined) {
          throw new Error(`Undefined variable: ${name}`);
        }
        this.stack.push(value);
        this.pc++;
        return `Load variable '${name}' (value: ${JSON.stringify(value)})`;
      }

      // -- ADD: Pop two values, push their sum -----------------------------
      case OpCode.ADD: {
        const b = this.stack.pop() as number;
        const a = this.stack.pop() as number;
        const result = (a as number) + (b as number);
        this.stack.push(result);
        this.pc++;
        return `Pop ${JSON.stringify(a)} and ${JSON.stringify(b)}, push sum ${JSON.stringify(result)}`;
      }

      // -- SUB: Pop two values, push their difference ----------------------
      case OpCode.SUB: {
        const b = this.stack.pop() as number;
        const a = this.stack.pop() as number;
        const result = a - b;
        this.stack.push(result);
        this.pc++;
        return `Pop ${a} and ${b}, push difference ${result}`;
      }

      // -- MUL: Pop two values, push their product -------------------------
      case OpCode.MUL: {
        const b = this.stack.pop() as number;
        const a = this.stack.pop() as number;
        const result = a * b;
        this.stack.push(result);
        this.pc++;
        return `Pop ${a} and ${b}, push product ${result}`;
      }

      // -- DIV: Pop two values, push their quotient ------------------------
      case OpCode.DIV: {
        const b = this.stack.pop() as number;
        const a = this.stack.pop() as number;
        if (b === 0) {
          throw new Error("Division by zero");
        }
        const result = a / b;
        this.stack.push(result);
        this.pc++;
        return `Pop ${a} and ${b}, push quotient ${result}`;
      }

      // -- PRINT: Pop top of stack and add to output -----------------------
      case OpCode.PRINT: {
        const value = this.stack.pop();
        const text = String(value);
        this.output.push(text);
        this.pc++;
        return `Print ${JSON.stringify(value)}`;
      }

      // -- HALT: Stop execution -------------------------------------------
      case OpCode.HALT: {
        return "Halt execution";
      }

      default: {
        const opName = OpCodeName[instr.opcode] ?? `0x${instr.opcode.toString(16)}`;
        throw new Error(`Unknown opcode: ${opName}`);
      }
    }
  }
}
