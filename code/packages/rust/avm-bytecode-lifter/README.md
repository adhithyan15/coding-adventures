# avm-bytecode-lifter

Deterministic ActionScript AVM1 and AVM2 binary bytecode lifter targeting InterpreterIR and vm-core for Rust.

Part of the **Coding Adventures** Flash Player suite (`SWF04`, see `code/specs/SWF00-flash-runtime-roadmap.md`).

---

## Overview

`avm-bytecode-lifter` translates compiled Flash bytecode from `DoAction` / `DoInitAction` tags (AVM1) and `DoABC` tags (AVM2) into `interpreter-ir` (`LANG01`) modules executable on `vm-core` (`LANG02`):

1. **AVM1 Action Lifter**:
   - Decodes AVM1 action byte streams into verified instruction sequences.
   - Stack and Arithmetic Actions: `ActionPush`, `ActionPop`, `ActionAdd2`, `ActionSubtract`, `ActionMultiply`, `ActionDivide`, `ActionEquals2`, `ActionLess2`, `ActionAnd`, `ActionOr`, `ActionNot`.
   - Object and Member Actions: `ActionGetVariable`, `ActionSetVariable`, `ActionGetMember`, `ActionSetMember`, `ActionCallFunction`, `ActionCallMethod`, `ActionNewObject`.
   - Control Flow Actions: `ActionBranchAlways`, `ActionBranchIfTrue`, `ActionDefineFunction`, `ActionDefineFunction2`, `ActionConstantPool`, `ActionTrace`, `ActionStop`, `ActionPlay`, `ActionGotoFrame`.
   - **Mandatory CFG & Stack Verification**:
     - Validates all branch offsets land strictly on decoded instruction boundaries (`LifterError::InvalidBranchTarget`).
     - Simulates abstract operand stack depths across all basic blocks, rejecting underflow (`LifterError::StackUnderflow`) or overflow (`LifterError::StackOverflow`).
2. **AVM2 ABC Lifter**:
   - Bounded ABC bytecode parser for `DoABC` constant pools, method signatures, metadata, instances, classes, scripts, and method bodies.
   - Opcodes: `pushbyte`, `pushshort`, `pushint`, `pushstring`, `getlocal`, `setlocal`, `add`, `subtract`, `multiply`, `divide`, `iflt`, `ifeq`, `jump`, `callproperty`, `callpropvoid`, `returnvalue`, `returnvoid`, etc.
   - CFG & operand stack verification prior to emission.
3. **CWE-1321 Prototype Pollution Defense**:
   - Isolated prototype registry per movie execution environment.
   - Frozen native built-ins (`Object.prototype`, `Array.prototype`, `String.prototype`, `Function.prototype`, `Math`, `MovieClip`).
   - Rejection/isolation of attempts to overwrite frozen prototype properties or inject untrusted descriptors.
4. **Deterministic VM Execution**:
   - Executes lifted `IIRModule`s on `vm-core::VMCore` with strict step limit enforcement (`max_steps <= 2_000_000`).
   - Captures trace output and timeline action events.
5. **Security & Bounds Guarantees**:
   - Bytecode size bound: `MAX_BYTECODE_SIZE = 16 MB` (CWE-400 defense).
   - Constant pool bound: `MAX_CONSTANTS = 65_536` (CWE-400 defense).
   - Operand stack depth: `MAX_OPERAND_STACK = 1_024` (CWE-400 defense).
   - Instruction emission cap: `MAX_INSTRUCTIONS = 100_000` (CWE-400 defense).
   - Zero `unsafe`: `#![deny(unsafe_code)]`.
   - Zero ambient authority: `"capabilities": []` in `required_capabilities.json`.
