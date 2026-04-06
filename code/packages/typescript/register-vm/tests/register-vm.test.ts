/**
 * register-vm.test.ts — Unit tests for @coding-adventures/register-vm.
 *
 * Each test builds a CodeObject by hand (the way a compiler would) and
 * exercises specific VM behaviours. The tests are deliberately verbose
 * and descriptive — they serve as runnable documentation.
 *
 * ## Organisation
 *
 * 1.  LDA_CONSTANT + HALT   → returnValue === 42
 * 2.  STAR + LDAR           → register round-trip
 * 3.  ADD (same types)      → monomorphic feedback
 * 4.  ADD (mixed types)     → feedback transitions through polymorphic to megamorphic
 * 5.  JUMP_IF_FALSE         → conditional skip
 * 6.  LDA_GLOBAL / STA_GLOBAL
 * 7.  CALL_ANY_RECEIVER     → closure invocation
 * 8.  HALT                  → returns accumulator immediately
 * 9.  LDA_NAMED_PROPERTY    → monomorphic hidden-class feedback
 * 10. STACK_CHECK           → overflow detection
 *
 * Plus additional tests for broader opcode coverage.
 */

import { describe, expect, it } from 'vitest';
import {
  Opcode,
  RegisterVM,
  newObject,
  objectWithHiddenClass,
  newVector,
  opcodeName,
  recordBinaryOp,
  recordCallSite,
  recordPropertyLoad,
  valueType,
  newContext,
  getSlot,
  setSlot,
} from '../src/index.js';
import type { CodeObject, RegisterInstruction } from '../src/index.js';

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/** Shorthand: create an instruction with no feedback slot. */
function instr(opcode: Opcode, ...operands: number[]): RegisterInstruction {
  return { opcode, operands, feedbackSlot: null };
}

/** Shorthand: create an instruction with a feedback slot. */
function instrFb(opcode: Opcode, feedbackSlot: number, ...operands: number[]): RegisterInstruction {
  return { opcode, operands, feedbackSlot };
}

/** Build a minimal CodeObject from an instruction list. */
function code(
  instructions: RegisterInstruction[],
  opts: Partial<Omit<CodeObject, 'instructions'>> = {},
): CodeObject {
  return {
    name: opts.name ?? 'test',
    instructions,
    constants: opts.constants ?? [],
    names: opts.names ?? [],
    registerCount: opts.registerCount ?? 8,
    feedbackSlotCount: opts.feedbackSlotCount ?? 4,
    parameterCount: opts.parameterCount ?? 0,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: LDA_CONSTANT + HALT → returnValue === 42
// ─────────────────────────────────────────────────────────────────────────────

describe('LDA_CONSTANT + HALT', () => {
  it('loads a constant and halts, returning the accumulator', () => {
    const vm = new RegisterVM();

    // Load 42 from constant pool (index 0) into accumulator, then stop.
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_CONSTANT, 0), // acc = constants[0] = 42
          instr(Opcode.HALT),            // stop; return acc
        ],
        { constants: [42] },
      ),
    );

    expect(result.error).toBeNull();
    expect(result.returnValue).toBe(42);
  });

  it('loads a string constant', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [instr(Opcode.LDA_CONSTANT, 0), instr(Opcode.HALT)],
        { constants: ['hello'] },
      ),
    );
    expect(result.returnValue).toBe('hello');
  });

  it('loads zero with LDA_ZERO', () => {
    const vm = new RegisterVM();
    const result = vm.execute(code([instr(Opcode.LDA_ZERO), instr(Opcode.HALT)]));
    expect(result.returnValue).toBe(0);
  });

  it('loads a small integer with LDA_SMI', () => {
    const vm = new RegisterVM();
    const result = vm.execute(code([instr(Opcode.LDA_SMI, 99), instr(Opcode.HALT)]));
    expect(result.returnValue).toBe(99);
  });

  it('loads undefined', () => {
    const vm = new RegisterVM();
    const result = vm.execute(code([instr(Opcode.LDA_UNDEFINED), instr(Opcode.HALT)]));
    expect(result.returnValue).toBeUndefined();
  });

  it('loads null', () => {
    const vm = new RegisterVM();
    const result = vm.execute(code([instr(Opcode.LDA_NULL), instr(Opcode.HALT)]));
    expect(result.returnValue).toBeNull();
  });

  it('loads true', () => {
    const vm = new RegisterVM();
    const result = vm.execute(code([instr(Opcode.LDA_TRUE), instr(Opcode.HALT)]));
    expect(result.returnValue).toBe(true);
  });

  it('loads false', () => {
    const vm = new RegisterVM();
    const result = vm.execute(code([instr(Opcode.LDA_FALSE), instr(Opcode.HALT)]));
    expect(result.returnValue).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: STAR + LDAR — register round-trip
// ─────────────────────────────────────────────────────────────────────────────

describe('STAR + LDAR register round-trip', () => {
  it('stores the accumulator to a register and reads it back', () => {
    const vm = new RegisterVM();

    // Load 7 → store to r0 → load 99 → load from r0 → halt.
    // Expected: accumulator is 7 (from r0), not 99.
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 7),   // acc = 7
        instr(Opcode.STAR, 0),      // r0  = acc (7)
        instr(Opcode.LDA_SMI, 99),  // acc = 99  (clobber accumulator)
        instr(Opcode.LDAR, 0),      // acc = r0  (7)
        instr(Opcode.HALT),
      ]),
    );

    expect(result.error).toBeNull();
    expect(result.returnValue).toBe(7);
  });

  it('MOV copies one register to another', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 55),  // acc = 55
        instr(Opcode.STAR, 0),      // r0 = 55
        instr(Opcode.MOV, 0, 1),    // r1 = r0
        instr(Opcode.LDAR, 1),      // acc = r1
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(55);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: ADD same types → monomorphic feedback
// ─────────────────────────────────────────────────────────────────────────────

describe('ADD with monomorphic feedback', () => {
  it('adds two numbers and records monomorphic feedback', () => {
    // Program:
    //   acc = 10          (LDA_SMI)
    //   r0  = acc         (STAR)
    //   acc = 20          (LDA_SMI)
    //   acc = acc + r0    (ADD with feedback slot 0)
    //   HALT              → acc = 30
    const vm = new RegisterVM();
    const { result, trace } = vm.executeWithTrace(
      code(
        [
          instr(Opcode.LDA_SMI, 10),
          instr(Opcode.STAR, 0),
          instr(Opcode.LDA_SMI, 20),
          // ADD: operands[0]=r0 (right), operands[1]=feedback slot 0
          { opcode: Opcode.ADD, operands: [0, 0], feedbackSlot: 0 },
          instr(Opcode.HALT),
        ],
        { feedbackSlotCount: 2 },
      ),
    );

    expect(result.error).toBeNull();
    expect(result.returnValue).toBe(30);

    // Find the ADD step in the trace and inspect its feedback delta.
    const addStep = trace.find(s => s.instruction.opcode === Opcode.ADD);
    expect(addStep).toBeDefined();
    expect(addStep!.feedbackDelta).toHaveLength(1);
    expect(addStep!.feedbackDelta[0].after.kind).toBe('monomorphic');
  });

  it('stays monomorphic when the same types are seen repeatedly', () => {
    const vector = newVector(1);
    recordBinaryOp(vector, 0, 5, 10);     // uninitialized → monomorphic
    recordBinaryOp(vector, 0, 3, 7);      // monomorphic (same: number, number)
    recordBinaryOp(vector, 0, 1, 100);    // monomorphic (same)

    expect(vector[0].kind).toBe('monomorphic');
    if (vector[0].kind === 'monomorphic') {
      expect(vector[0].types).toHaveLength(1);
      expect(vector[0].types[0]).toEqual(['number', 'number']);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: ADD mixed types → feedback transitions to megamorphic
// ─────────────────────────────────────────────────────────────────────────────

describe('ADD mixed types → feedback state transitions', () => {
  it('transitions uninitialized → mono → poly → mega as new types arrive', () => {
    const vector = newVector(1);

    // Step 1: uninitialized → monomorphic
    recordBinaryOp(vector, 0, 1, 2);
    expect(vector[0].kind).toBe('monomorphic');

    // Step 2: monomorphic → polymorphic (new type pair: string+string)
    recordBinaryOp(vector, 0, 'a', 'b');
    expect(vector[0].kind).toBe('polymorphic');

    // Step 3: polymorphic grows (boolean+number)
    recordBinaryOp(vector, 0, true, 1);
    expect(vector[0].kind).toBe('polymorphic');

    // Step 4: polymorphic grows (null+undefined)
    recordBinaryOp(vector, 0, null, undefined);
    expect(vector[0].kind).toBe('polymorphic');
    if (vector[0].kind === 'polymorphic') {
      expect(vector[0].types).toHaveLength(4);
    }

    // Step 5: 5th distinct pair → megamorphic
    recordBinaryOp(vector, 0, 'x', 99);
    expect(vector[0].kind).toBe('megamorphic');

    // Step 6: megamorphic stays megamorphic
    recordBinaryOp(vector, 0, 1, 1);
    expect(vector[0].kind).toBe('megamorphic');
  });

  it('valueType correctly classifies all VMValue kinds', () => {
    expect(valueType(42)).toBe('number');
    expect(valueType('hi')).toBe('string');
    expect(valueType(true)).toBe('boolean');
    expect(valueType(null)).toBe('null');
    expect(valueType(undefined)).toBe('undefined');
    expect(valueType([])).toBe('array');
    expect(valueType(newObject())).toBe('object');
    const fn = { kind: 'function' as const, code: null as unknown as CodeObject, context: null };
    expect(valueType(fn)).toBe('function');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Test 5: JUMP_IF_FALSE skips instruction
// ─────────────────────────────────────────────────────────────────────────────

describe('JUMP_IF_FALSE conditional branch', () => {
  it('skips the next instruction when accumulator is false', () => {
    // Program:
    //   acc = false
    //   JUMP_IF_FALSE +1   (skip next instruction)
    //   acc = 99           (should be skipped)
    //   acc = 42           (should run)
    //   HALT
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_FALSE),
        instr(Opcode.JUMP_IF_FALSE, 1),  // offset +1 → skip 1 instruction
        instr(Opcode.LDA_SMI, 99),       // skipped
        instr(Opcode.LDA_SMI, 42),       // executed
        instr(Opcode.HALT),
      ]),
    );

    expect(result.error).toBeNull();
    expect(result.returnValue).toBe(42);
  });

  it('does NOT skip when accumulator is true', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_TRUE),
        instr(Opcode.JUMP_IF_FALSE, 1),  // condition is true → don't jump
        instr(Opcode.LDA_SMI, 99),       // executed
        instr(Opcode.LDA_SMI, 42),       // also executed
        instr(Opcode.HALT),
      ]),
    );
    // Both LDA_SMI run; last one wins.
    expect(result.returnValue).toBe(42);
  });

  it('JUMP_IF_TRUE jumps on truthy accumulator', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_TRUE),
        instr(Opcode.JUMP_IF_TRUE, 1),   // jump over next instruction
        instr(Opcode.LDA_SMI, 1),        // skipped
        instr(Opcode.LDA_SMI, 2),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(2);
  });

  it('JUMP skips unconditionally', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.JUMP, 1),           // skip next
        instr(Opcode.LDA_SMI, 1),        // skipped
        instr(Opcode.LDA_SMI, 7),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(7);
  });

  it('JUMP_IF_NULL jumps when accumulator is null', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_NULL),
        instr(Opcode.JUMP_IF_NULL, 1),
        instr(Opcode.LDA_SMI, 1),
        instr(Opcode.LDA_SMI, 5),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(5);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Test 6: LDA_GLOBAL / STA_GLOBAL
// ─────────────────────────────────────────────────────────────────────────────

describe('Global variable access (LDA_GLOBAL / STA_GLOBAL)', () => {
  it('stores to a global and loads it back', () => {
    // Program:
    //   acc = 123
    //   globals['counter'] = acc   (STA_GLOBAL)
    //   acc = 0
    //   acc = globals['counter']   (LDA_GLOBAL) → 123
    //   HALT
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_SMI, 123),
          instr(Opcode.STA_GLOBAL, 0),   // names[0] = 'counter'
          instr(Opcode.LDA_ZERO),
          instr(Opcode.LDA_GLOBAL, 0),   // acc = globals['counter']
          instr(Opcode.HALT),
        ],
        { names: ['counter'] },
      ),
    );

    expect(result.error).toBeNull();
    expect(result.returnValue).toBe(123);
  });

  it('returns undefined for an unset global', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [instr(Opcode.LDA_GLOBAL, 0), instr(Opcode.HALT)],
        { names: ['notSet'] },
      ),
    );
    expect(result.returnValue).toBeUndefined();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Test 7: CALL_ANY_RECEIVER with closure
// ─────────────────────────────────────────────────────────────────────────────

describe('CALL_ANY_RECEIVER with closure', () => {
  it('invokes a closure that returns a constant', () => {
    // Inner code: load 777 from its own constant pool, return.
    const innerCode: CodeObject = {
      name: 'getNumber',
      instructions: [
        instr(Opcode.LDA_CONSTANT, 0),  // acc = 777
        instr(Opcode.RETURN),
      ],
      constants: [777],
      names: [],
      registerCount: 0,
      feedbackSlotCount: 0,
      parameterCount: 0,
    };

    // Outer code:
    //   r0 = closure(innerCode)    via CREATE_CLOSURE
    //   call r0 with 0 args        via CALL_ANY_RECEIVER
    //   HALT                       → acc = 777
    const outerCode = code(
      [
        instr(Opcode.CREATE_CLOSURE, 0),  // acc = closure wrapping innerCode
        instr(Opcode.STAR, 0),            // r0 = closure
        // CALL_ANY_RECEIVER: callableReg=0, firstArgReg=2, argc=0, feedbackSlot=0
        { opcode: Opcode.CALL_ANY_RECEIVER, operands: [0, 2, 0, 0], feedbackSlot: 0 },
        instr(Opcode.HALT),
      ],
      { constants: [innerCode], feedbackSlotCount: 2 },
    );

    const vm = new RegisterVM();
    const result = vm.execute(outerCode);

    expect(result.error).toBeNull();
    expect(result.returnValue).toBe(777);
  });

  it('passes arguments to a closure', () => {
    // Inner: load param 0 (r0), add smi 1, return.
    const adderCode: CodeObject = {
      name: 'addOne',
      instructions: [
        instr(Opcode.LDAR, 0),     // acc = first parameter (r0)
        instr(Opcode.ADD_SMI, 1),  // acc = acc + 1
        instr(Opcode.RETURN),
      ],
      constants: [],
      names: [],
      registerCount: 2,
      feedbackSlotCount: 0,
      parameterCount: 1,
    };

    const outerCode = code(
      [
        instr(Opcode.CREATE_CLOSURE, 0),  // acc = addOne closure
        instr(Opcode.STAR, 0),            // r0 = addOne
        instr(Opcode.LDA_SMI, 41),
        instr(Opcode.STAR, 1),            // r1 = 41 (argument)
        // CALL: callableReg=0, firstArgReg=1, argc=1
        { opcode: Opcode.CALL_ANY_RECEIVER, operands: [0, 1, 1, 0], feedbackSlot: null },
        instr(Opcode.HALT),
      ],
      { constants: [adderCode], feedbackSlotCount: 2 },
    );

    const vm = new RegisterVM();
    const result = vm.execute(outerCode);

    expect(result.error).toBeNull();
    expect(result.returnValue).toBe(42);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Test 8: HALT returns accumulator immediately
// ─────────────────────────────────────────────────────────────────────────────

describe('HALT terminates immediately', () => {
  it('stops before later instructions execute', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 10),
        instr(Opcode.HALT),
        instr(Opcode.LDA_SMI, 99),  // never reached
      ]),
    );
    expect(result.returnValue).toBe(10);
  });

  it('RETURN also stops the current frame', () => {
    const innerCode: CodeObject = {
      name: 'returner',
      instructions: [
        instr(Opcode.LDA_SMI, 5),
        instr(Opcode.RETURN),
        instr(Opcode.LDA_SMI, 9),  // never reached
      ],
      constants: [],
      names: [],
      registerCount: 0,
      feedbackSlotCount: 0,
      parameterCount: 0,
    };

    const outerCode = code(
      [
        instr(Opcode.CREATE_CLOSURE, 0),
        instr(Opcode.STAR, 0),
        { opcode: Opcode.CALL_ANY_RECEIVER, operands: [0, 1, 0, 0], feedbackSlot: null },
        instr(Opcode.HALT),
      ],
      { constants: [innerCode] },
    );

    const vm = new RegisterVM();
    const result = vm.execute(outerCode);
    expect(result.returnValue).toBe(5);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Test 9: LDA_NAMED_PROPERTY — monomorphic hidden-class feedback
// ─────────────────────────────────────────────────────────────────────────────

describe('LDA_NAMED_PROPERTY with hidden-class feedback', () => {
  it('loads a property and records monomorphic feedback', () => {
    // Build an object { x: 42 } manually and put it in the constant pool.
    const obj = newObject();
    obj.properties.set('x', 42);
    const originalClassId = obj.hiddenClassId;

    // Program:
    //   acc = obj (from constants)
    //   r0 = acc
    //   LDA_NAMED_PROPERTY r0, 'x', feedbackSlot=0
    //   HALT
    const vm = new RegisterVM();
    const { result, trace } = vm.executeWithTrace(
      code(
        [
          instr(Opcode.LDA_CONSTANT, 0),         // acc = obj
          instr(Opcode.STAR, 0),                  // r0 = obj
          { opcode: Opcode.LDA_NAMED_PROPERTY, operands: [0, 0, 0], feedbackSlot: 0 },
          instr(Opcode.HALT),
        ],
        { constants: [obj], names: ['x'], feedbackSlotCount: 2 },
      ),
    );

    expect(result.error).toBeNull();
    expect(result.returnValue).toBe(42);

    // The LDA_NAMED_PROPERTY step should have recorded the hidden class.
    const loadStep = trace.find(s => s.instruction.opcode === Opcode.LDA_NAMED_PROPERTY);
    expect(loadStep).toBeDefined();
    expect(loadStep!.feedbackDelta).toHaveLength(1);
    expect(loadStep!.feedbackDelta[0].after.kind).toBe('monomorphic');

    // The type string encodes the hidden class id.
    if (loadStep!.feedbackDelta[0].after.kind === 'monomorphic') {
      expect(loadStep!.feedbackDelta[0].after.types[0][0]).toBe(`hc:${originalClassId}`);
    }
  });

  it('transitions to polymorphic when two different objects are accessed', () => {
    // Two objects with different hidden class IDs.
    const vector = newVector(1);
    const obj1 = newObject();
    const obj2 = newObject();
    recordPropertyLoad(vector, 0, obj1.hiddenClassId);   // monomorphic
    recordPropertyLoad(vector, 0, obj2.hiddenClassId);   // polymorphic

    expect(vector[0].kind).toBe('polymorphic');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Test 10: STACK_CHECK throws on overflow
// ─────────────────────────────────────────────────────────────────────────────

describe('STACK_CHECK — stack overflow detection', () => {
  it('throws a VMError when call depth exceeds maxDepth', () => {
    // Build a self-calling function that recurses forever.
    // The inner code: STACK_CHECK, CREATE_CLOSURE(self), CALL_ANY_RECEIVER, RETURN
    //
    // We use a tiny maxDepth so the test runs quickly.

    // Create the recursive code object (it references itself via constant pool).
    const recurseCode: CodeObject = {
      name: 'recurse',
      instructions: [
        instr(Opcode.STACK_CHECK),
        instr(Opcode.CREATE_CLOSURE, 0),   // acc = closure(recurseCode)
        instr(Opcode.STAR, 0),             // r0 = closure
        { opcode: Opcode.CALL_ANY_RECEIVER, operands: [0, 1, 0, 0], feedbackSlot: null },
        instr(Opcode.RETURN),
      ],
      constants: [] as unknown[],  // filled below
      names: [],
      registerCount: 4,
      feedbackSlotCount: 1,
      parameterCount: 0,
    } as CodeObject;
    // Point the constant pool back to itself.
    (recurseCode.constants as unknown[]).push(recurseCode);

    const outerCode = code(
      [
        instr(Opcode.CREATE_CLOSURE, 0),
        instr(Opcode.STAR, 0),
        { opcode: Opcode.CALL_ANY_RECEIVER, operands: [0, 1, 0, 0], feedbackSlot: null },
        instr(Opcode.HALT),
      ],
      { constants: [recurseCode] },
    );

    const vm = new RegisterVM({ maxDepth: 5 });
    const result = vm.execute(outerCode);

    expect(result.error).not.toBeNull();
    expect(result.error!.message).toMatch(/maximum call stack/i);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Additional opcode coverage tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Arithmetic opcodes', () => {
  const vm = new RegisterVM();

  it('SUB subtracts', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 10),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 3),
        instr(Opcode.SUB, 0),    // 3 - 10 = -7
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(-7);
  });

  it('MUL multiplies', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 6),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 7),
        instr(Opcode.MUL, 0),   // 7 * 6 = 42
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(42);
  });

  it('DIV divides', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 4),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 20),
        instr(Opcode.DIV, 0),   // 20 / 4 = 5
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(5);
  });

  it('MOD returns remainder', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 3),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 10),
        instr(Opcode.MOD, 0),   // 10 % 3 = 1
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(1);
  });

  it('POW raises to power', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 3),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 2),
        instr(Opcode.POW, 0),   // 2 ** 3 = 8
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(8);
  });

  it('ADD_SMI adds a literal', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 40),
        instr(Opcode.ADD_SMI, 2),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(42);
  });

  it('SUB_SMI subtracts a literal', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 45),
        instr(Opcode.SUB_SMI, 3),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(42);
  });

  it('NEGATE negates the accumulator', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 42),
        instr(Opcode.NEGATE),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(-42);
  });

  it('ADD returns NaN for incompatible non-string types', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_NULL),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_TRUE),
        instr(Opcode.ADD, 0),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBeNaN();
  });

  it('ADD concatenates strings', () => {
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_CONSTANT, 0),  // 'hello'
          instr(Opcode.STAR, 0),
          instr(Opcode.LDA_CONSTANT, 1),  // ' world'
          instr(Opcode.ADD, 0),           // ' world' + 'hello' = ' worldhello'
          instr(Opcode.HALT),
        ],
        { constants: ['hello', ' world'] },
      ),
    );
    // acc = ' world', right = r0 = 'hello'
    // doAdd(' world', 'hello') = ' worldhello'
    expect(result.returnValue).toBe(' worldhello');
  });
});

describe('Bitwise opcodes', () => {
  const vm = new RegisterVM();

  it('BITWISE_AND', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 0b1100),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 0b1010),
        instr(Opcode.BITWISE_AND, 0),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(0b1000);
  });

  it('BITWISE_OR', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 0b1100),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 0b1010),
        instr(Opcode.BITWISE_OR, 0),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(0b1110);
  });

  it('BITWISE_XOR', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 0b1100),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 0b1010),
        instr(Opcode.BITWISE_XOR, 0),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(0b0110);
  });

  it('BITWISE_NOT', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 0),
        instr(Opcode.BITWISE_NOT),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(-1);
  });

  it('SHIFT_LEFT', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 1),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 1),
        instr(Opcode.SHIFT_LEFT, 0),   // 1 << 1 = 2
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(2);
  });

  it('SHIFT_RIGHT', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 1),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 8),
        instr(Opcode.SHIFT_RIGHT, 0),  // 8 >> 1 = 4
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(4);
  });
});

describe('Comparison opcodes', () => {
  const vm = new RegisterVM();

  it('TEST_EQUAL', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 5),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 5),
        instr(Opcode.TEST_EQUAL, 0),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(true);
  });

  it('TEST_LESS_THAN', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 10),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 3),
        instr(Opcode.TEST_LESS_THAN, 0),  // 3 < 10
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(true);
  });

  it('LOGICAL_NOT', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_TRUE),
        instr(Opcode.LOGICAL_NOT),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(false);
  });

  it('TYPEOF returns number for integer', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 1),
        instr(Opcode.TYPEOF),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe('number');
  });

  it('TYPEOF returns undefined for undefined', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_UNDEFINED),
        instr(Opcode.TYPEOF),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe('undefined');
  });

  it('TYPEOF returns object for null', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_NULL),
        instr(Opcode.TYPEOF),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe('object');
  });

  it('TEST_UNDETECTABLE returns true for null', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_NULL),
        instr(Opcode.TEST_UNDETECTABLE),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(true);
  });

  it('TEST_NOT_EQUAL', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 1),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 2),
        instr(Opcode.TEST_NOT_EQUAL, 0),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(true);
  });
});

describe('Object creation and property access', () => {
  it('CREATE_OBJECT_LITERAL creates an empty object', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.CREATE_OBJECT_LITERAL),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBeDefined();
    expect(typeof result.returnValue).toBe('object');
  });

  it('STA_NAMED_PROPERTY sets a property', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.CREATE_OBJECT_LITERAL),   // acc = {}
          instr(Opcode.STAR, 0),                  // r0 = obj
          instr(Opcode.LDA_SMI, 99),
          instr(Opcode.STA_NAMED_PROPERTY, 0, 0), // r0['value'] = 99
          instr(Opcode.LDA_NAMED_PROPERTY_NO_FEEDBACK, 0, 0),  // acc = r0['value']
          instr(Opcode.HALT),
        ],
        { names: ['value'] },
      ),
    );
    expect(result.returnValue).toBe(99);
  });

  it('CREATE_ARRAY_LITERAL creates an array', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([instr(Opcode.CREATE_ARRAY_LITERAL), instr(Opcode.HALT)]),
    );
    expect(Array.isArray(result.returnValue)).toBe(true);
  });

  it('CLONE_OBJECT shallow-clones an object', () => {
    const vm = new RegisterVM();
    const { result } = vm.executeWithTrace(
      code(
        [
          instr(Opcode.CREATE_OBJECT_LITERAL),
          instr(Opcode.STAR, 0),
          instr(Opcode.LDA_SMI, 5),
          instr(Opcode.STA_NAMED_PROPERTY, 0, 0),  // r0.n = 5
          instr(Opcode.LDAR, 0),
          instr(Opcode.CLONE_OBJECT),               // acc = clone of r0
          instr(Opcode.HALT),
        ],
        { names: ['n'] },
      ),
    );
    expect(result.error).toBeNull();
  });
});

describe('Context / scope chain', () => {
  it('newContext creates a context with undefined slots', () => {
    const ctx = newContext(null, 3);
    expect(ctx.slots).toHaveLength(3);
    expect(ctx.slots.every(v => v === undefined)).toBe(true);
    expect(ctx.parent).toBeNull();
  });

  it('getSlot / setSlot at depth 0', () => {
    const ctx = newContext(null, 2);
    setSlot(ctx, 0, 1, 42);
    expect(getSlot(ctx, 0, 1)).toBe(42);
  });

  it('getSlot walks the parent chain', () => {
    const outer = newContext(null, 1);
    setSlot(outer, 0, 0, 99);
    const inner = newContext(outer, 1);
    expect(getSlot(inner, 1, 0)).toBe(99);
  });

  it('getSlot returns undefined for out-of-bounds depth', () => {
    const ctx = newContext(null, 1);
    expect(getSlot(ctx, 5, 0)).toBeUndefined();
  });

  it('LDA_CURRENT_CONTEXT_SLOT reads from the current context', () => {
    const vm = new RegisterVM();
    // Program: push context, store 77 to slot 0, read it back, halt.
    const result = vm.execute(
      code([
        instr(Opcode.PUSH_CONTEXT, 2),              // push new ctx with 2 slots
        instr(Opcode.LDA_SMI, 77),
        instr(Opcode.STA_CURRENT_CONTEXT_SLOT, 0),  // ctx[0] = 77
        instr(Opcode.LDA_ZERO),
        instr(Opcode.LDA_CURRENT_CONTEXT_SLOT, 0),  // acc = ctx[0]
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(77);
  });

  it('POP_CONTEXT restores the parent', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.PUSH_CONTEXT, 1),
        instr(Opcode.LDA_SMI, 5),
        instr(Opcode.STA_CURRENT_CONTEXT_SLOT, 0),
        instr(Opcode.POP_CONTEXT),
        // After pop, reading slot 0 of null context returns undefined
        instr(Opcode.LDA_CURRENT_CONTEXT_SLOT, 0),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBeUndefined();
  });
});

describe('opcodeName utility', () => {
  it('returns the correct name for known opcodes', () => {
    expect(opcodeName(0x00)).toBe('LDA_CONSTANT');
    expect(opcodeName(0xff)).toBe('HALT');
    expect(opcodeName(0x30)).toBe('ADD');
    expect(opcodeName(0x50)).toBe('JUMP');
  });

  it('returns UNKNOWN(...) for unrecognized opcodes', () => {
    expect(opcodeName(0xee)).toBe('UNKNOWN(0xEE)');
  });
});

describe('Feedback utilities', () => {
  it('recordCallSite records function type', () => {
    const vector = newVector(1);
    recordCallSite(vector, 0, 'function');
    expect(vector[0].kind).toBe('monomorphic');
  });

  it('newVector returns all-uninitialized slots', () => {
    const v = newVector(5);
    expect(v).toHaveLength(5);
    expect(v.every(s => s.kind === 'uninitialized')).toBe(true);
  });

  it('ignores slot index out of range', () => {
    const vector = newVector(1);
    // Should not throw
    recordBinaryOp(vector, 99, 1, 2);
    expect(vector[0].kind).toBe('uninitialized');
  });
});

describe('Error handling', () => {
  it('returns an error for calling a non-function', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 42),
        instr(Opcode.STAR, 0),
        // Try to call 42 as a function
        { opcode: Opcode.CALL_ANY_RECEIVER, operands: [0, 1, 0, 0], feedbackSlot: null },
        instr(Opcode.HALT),
      ]),
    );
    expect(result.error).not.toBeNull();
    expect(result.error!.message).toMatch(/not a function/i);
  });

  it('THROW produces a VM error', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_CONSTANT, 0),
          instr(Opcode.THROW),
        ],
        { constants: ['oops'] },
      ),
    );
    expect(result.error).not.toBeNull();
    expect(result.error!.message).toContain('oops');
  });

  it('unknown opcode produces an error', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([{ opcode: 0xde, operands: [], feedbackSlot: null }]),
    );
    expect(result.error).not.toBeNull();
    expect(result.error!.message).toMatch(/unknown opcode/i);
  });
});

describe('executeWithTrace', () => {
  it('trace has one entry per instruction', () => {
    const vm = new RegisterVM();
    const { trace } = vm.executeWithTrace(
      code([
        instr(Opcode.LDA_SMI, 1),
        instr(Opcode.LDA_SMI, 2),
        instr(Opcode.HALT),
      ]),
    );
    // 3 instructions → 3 trace steps (HALT is recorded before stopping)
    expect(trace).toHaveLength(3);
  });

  it('trace captures accumulator before and after', () => {
    const vm = new RegisterVM();
    const { trace } = vm.executeWithTrace(
      code([
        instr(Opcode.LDA_SMI, 7),
        instr(Opcode.HALT),
      ]),
    );
    const ldaStep = trace[0];
    expect(ldaStep.accBefore).toBeUndefined();
    expect(ldaStep.accAfter).toBe(7);
  });
});

describe('newObject / objectWithHiddenClass', () => {
  it('newObject assigns distinct hiddenClassIds', () => {
    const a = newObject();
    const b = newObject();
    expect(a.hiddenClassId).not.toBe(b.hiddenClassId);
  });
});

describe('Keyed property access', () => {
  it('LDA_KEYED_PROPERTY reads by key in accumulator', () => {
    const vm = new RegisterVM();
    // Build code: create obj, set 'y'=55 via STA_NAMED_PROPERTY, then LDA_KEYED_PROPERTY
    const result = vm.execute(
      code(
        [
          instr(Opcode.CREATE_OBJECT_LITERAL),         // acc = {}
          instr(Opcode.STAR, 0),                        // r0 = obj
          instr(Opcode.LDA_SMI, 55),
          instr(Opcode.STA_NAMED_PROPERTY, 0, 0),      // r0['y'] = 55
          instr(Opcode.LDA_CONSTANT, 0),               // acc = 'y' (key)
          instr(Opcode.LDA_KEYED_PROPERTY, 0),         // acc = r0['y']
          instr(Opcode.HALT),
        ],
        { names: ['y'], constants: ['y'] },
      ),
    );
    expect(result.returnValue).toBe(55);
  });
});

describe('TEST_IN and TEST_GREATER_THAN', () => {
  it('TEST_IN checks property existence', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.CREATE_OBJECT_LITERAL),
          instr(Opcode.STAR, 0),                       // r0 = {}
          instr(Opcode.LDA_SMI, 1),
          instr(Opcode.STA_NAMED_PROPERTY, 0, 0),      // r0['a'] = 1
          instr(Opcode.LDA_CONSTANT, 0),               // acc = 'a'
          instr(Opcode.TEST_IN, 0),                    // acc = 'a' in r0
          instr(Opcode.HALT),
        ],
        { names: ['a'], constants: ['a'] },
      ),
    );
    expect(result.returnValue).toBe(true);
  });

  it('TEST_GREATER_THAN compares numbers', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 3),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 10),
        instr(Opcode.TEST_GREATER_THAN, 0),   // 10 > 3
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Additional coverage tests
// ─────────────────────────────────────────────────────────────────────────────

describe('CONSTRUCT opcode', () => {
  it('creates an object and runs the constructor body', () => {
    const ctorCode: CodeObject = {
      name: 'MyClass',
      instructions: [
        instr(Opcode.RETURN),
      ],
      constants: [],
      names: [],
      registerCount: 2,
      feedbackSlotCount: 0,
      parameterCount: 0,
    };

    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.CREATE_CLOSURE, 0),
          instr(Opcode.STAR, 0),
          { opcode: Opcode.CONSTRUCT, operands: [0, 1, 0], feedbackSlot: null },
          instr(Opcode.HALT),
        ],
        { constants: [ctorCode] },
      ),
    );
    expect(result.error).toBeNull();
    expect(typeof result.returnValue).toBe('object');
    expect(result.returnValue).not.toBeNull();
  });

  it('CONSTRUCT throws when callee is not a function', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 5),
        instr(Opcode.STAR, 0),
        { opcode: Opcode.CONSTRUCT, operands: [0, 1, 0], feedbackSlot: null },
        instr(Opcode.HALT),
      ]),
    );
    expect(result.error).not.toBeNull();
    expect(result.error!.message).toMatch(/not a constructor/i);
  });
});

describe('CALL_WITH_SPREAD and CONSTRUCT_WITH_SPREAD', () => {
  it('CALL_WITH_SPREAD calls a function', () => {
    const innerCode: CodeObject = {
      name: 'spreadFn',
      instructions: [instr(Opcode.LDA_SMI, 77), instr(Opcode.RETURN)],
      constants: [],
      names: [],
      registerCount: 0,
      feedbackSlotCount: 0,
      parameterCount: 0,
    };

    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.CREATE_CLOSURE, 0),
          instr(Opcode.STAR, 0),
          { opcode: Opcode.CALL_WITH_SPREAD, operands: [0, 1, 0], feedbackSlot: null },
          instr(Opcode.HALT),
        ],
        { constants: [innerCode] },
      ),
    );
    expect(result.error).toBeNull();
    expect(result.returnValue).toBe(77);
  });

  it('CONSTRUCT_WITH_SPREAD calls a constructor', () => {
    const innerCode: CodeObject = {
      name: 'CtorSpread',
      instructions: [instr(Opcode.RETURN)],
      constants: [],
      names: [],
      registerCount: 2,
      feedbackSlotCount: 0,
      parameterCount: 0,
    };

    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.CREATE_CLOSURE, 0),
          instr(Opcode.STAR, 0),
          { opcode: Opcode.CONSTRUCT_WITH_SPREAD, operands: [0, 1, 0], feedbackSlot: null },
          instr(Opcode.HALT),
        ],
        { constants: [innerCode] },
      ),
    );
    expect(result.error).toBeNull();
  });

  it('CALL_WITH_SPREAD throws for non-function', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 1),
        instr(Opcode.STAR, 0),
        { opcode: Opcode.CALL_WITH_SPREAD, operands: [0, 1, 0], feedbackSlot: null },
        instr(Opcode.HALT),
      ]),
    );
    expect(result.error).not.toBeNull();
  });
});

describe('Generator opcodes (no-ops)', () => {
  it('SUSPEND_GENERATOR and RESUME_GENERATOR are no-ops', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 10),
        instr(Opcode.SUSPEND_GENERATOR),
        instr(Opcode.RESUME_GENERATOR),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(10);
    expect(result.error).toBeNull();
  });
});

describe('Array LDA_NAMED_PROPERTY', () => {
  it('reads .length from an array stored in a register', () => {
    const arr = [1, 2, 3];
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_CONSTANT, 0),
          instr(Opcode.STAR, 0),
          { opcode: Opcode.LDA_NAMED_PROPERTY, operands: [0, 0, 0], feedbackSlot: null },
          instr(Opcode.HALT),
        ],
        { constants: [arr], names: ['length'], feedbackSlotCount: 2 },
      ),
    );
    expect(result.returnValue).toBe(3);
  });

  it('returns undefined for non-length array property', () => {
    const arr = [1, 2, 3];
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_CONSTANT, 0),
          instr(Opcode.STAR, 0),
          { opcode: Opcode.LDA_NAMED_PROPERTY, operands: [0, 0, 0], feedbackSlot: null },
          instr(Opcode.HALT),
        ],
        { constants: [arr], names: ['foo'], feedbackSlotCount: 1 },
      ),
    );
    expect(result.returnValue).toBeUndefined();
  });

  it('returns undefined when register holds a primitive', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_SMI, 5),
          instr(Opcode.STAR, 0),
          { opcode: Opcode.LDA_NAMED_PROPERTY, operands: [0, 0, 0], feedbackSlot: null },
          instr(Opcode.HALT),
        ],
        { names: ['x'], feedbackSlotCount: 1 },
      ),
    );
    expect(result.returnValue).toBeUndefined();
  });
});

describe('DELETE_PROPERTY opcodes', () => {
  it('DELETE_PROPERTY_STRICT removes a property', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.CREATE_OBJECT_LITERAL),
          instr(Opcode.STAR, 0),
          instr(Opcode.LDA_SMI, 1),
          instr(Opcode.STA_NAMED_PROPERTY, 0, 0),
          instr(Opcode.LDA_CONSTANT, 0),
          instr(Opcode.STAR, 1),
          { opcode: Opcode.DELETE_PROPERTY_STRICT, operands: [0, 1], feedbackSlot: null },
          instr(Opcode.HALT),
        ],
        { names: ['x'], constants: ['x'] },
      ),
    );
    expect(result.error).toBeNull();
    expect(result.returnValue).toBe(true);
  });

  it('DELETE_PROPERTY_SLOPPY removes a property', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.CREATE_OBJECT_LITERAL),
          instr(Opcode.STAR, 0),
          instr(Opcode.LDA_SMI, 9),
          instr(Opcode.STA_NAMED_PROPERTY, 0, 0),
          instr(Opcode.LDA_CONSTANT, 0),
          instr(Opcode.STAR, 1),
          { opcode: Opcode.DELETE_PROPERTY_SLOPPY, operands: [0, 1], feedbackSlot: null },
          instr(Opcode.HALT),
        ],
        { names: ['a'], constants: ['a'] },
      ),
    );
    expect(result.returnValue).toBe(true);
  });

  it('DELETE_PROPERTY_STRICT returns true for non-objects', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_SMI, 5),
          instr(Opcode.STAR, 0),
          instr(Opcode.LDA_CONSTANT, 0),
          instr(Opcode.STAR, 1),
          { opcode: Opcode.DELETE_PROPERTY_STRICT, operands: [0, 1], feedbackSlot: null },
          instr(Opcode.HALT),
        ],
        { constants: ['x'] },
      ),
    );
    expect(result.returnValue).toBe(true);
  });
});

describe('CREATE_REGEXP_LITERAL', () => {
  it('creates an object from a regexp literal in the constant pool', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          { opcode: Opcode.CREATE_REGEXP_LITERAL, operands: [0], feedbackSlot: null },
          instr(Opcode.HALT),
        ],
        { constants: ['^hello$'] },
      ),
    );
    expect(result.error).toBeNull();
    expect(typeof result.returnValue).toBe('object');
  });
});

describe('Module variable opcodes', () => {
  it('LDA_MODULE_VARIABLE reads from globals', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_SMI, 55),
          instr(Opcode.STA_GLOBAL, 0),
          instr(Opcode.LDA_ZERO),
          { opcode: Opcode.LDA_MODULE_VARIABLE, operands: [0], feedbackSlot: null },
          instr(Opcode.HALT),
        ],
        { names: ['mod'] },
      ),
    );
    expect(result.returnValue).toBe(55);
  });

  it('STA_MODULE_VARIABLE writes to globals', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_SMI, 88),
          { opcode: Opcode.STA_MODULE_VARIABLE, operands: [0], feedbackSlot: null },
          instr(Opcode.LDA_ZERO),
          instr(Opcode.LDA_GLOBAL, 0),
          instr(Opcode.HALT),
        ],
        { names: ['mod2'] },
      ),
    );
    expect(result.returnValue).toBe(88);
  });
});

describe('STA_KEYED_PROPERTY', () => {
  it('writes to a keyed property on an array', () => {
    const vm = new RegisterVM();
    const arr: number[] = [];
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_CONSTANT, 0),
          instr(Opcode.STAR, 0),
          instr(Opcode.LDA_ZERO),
          instr(Opcode.STAR, 1),
          instr(Opcode.LDA_SMI, 42),
          { opcode: Opcode.STA_KEYED_PROPERTY, operands: [0, 1], feedbackSlot: null },
          instr(Opcode.LDA_ZERO),
          instr(Opcode.LDA_KEYED_PROPERTY, 0),
          instr(Opcode.HALT),
        ],
        { constants: [arr] },
      ),
    );
    expect(result.returnValue).toBe(42);
  });

  it('writes to a keyed property on an object', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.CREATE_OBJECT_LITERAL),
          instr(Opcode.STAR, 0),
          instr(Opcode.LDA_CONSTANT, 0),
          instr(Opcode.STAR, 1),
          instr(Opcode.LDA_SMI, 99),
          { opcode: Opcode.STA_KEYED_PROPERTY, operands: [0, 1], feedbackSlot: null },
          instr(Opcode.LDA_CONSTANT, 0),
          instr(Opcode.LDA_KEYED_PROPERTY, 0),
          instr(Opcode.HALT),
        ],
        { constants: ['foo'] },
      ),
    );
    expect(result.returnValue).toBe(99);
  });
});

describe('Iterator protocol', () => {
  it('GET_ITERATOR wraps an array', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_CONSTANT, 0),
          instr(Opcode.GET_ITERATOR),
          instr(Opcode.HALT),
        ],
        { constants: [[10, 20]] },
      ),
    );
    expect(result.error).toBeNull();
    expect(typeof result.returnValue).toBe('object');
  });

  it('CALL_ITERATOR_STEP advances the iterator', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_CONSTANT, 0),
          instr(Opcode.GET_ITERATOR),
          instr(Opcode.CALL_ITERATOR_STEP),
          instr(Opcode.GET_ITERATOR_VALUE),
          instr(Opcode.HALT),
        ],
        { constants: [[10, 20]] },
      ),
    );
    expect(result.error).toBeNull();
    expect(result.returnValue).toBe(10);
  });

  it('GET_ITERATOR_DONE returns true when exhausted', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_CONSTANT, 0),
          instr(Opcode.GET_ITERATOR),
          instr(Opcode.CALL_ITERATOR_STEP),
          instr(Opcode.GET_ITERATOR_DONE),
          instr(Opcode.HALT),
        ],
        { constants: [[]] },
      ),
    );
    expect(result.returnValue).toBe(true);
  });

  it('GET_ITERATOR throws for non-array values', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 5),
        instr(Opcode.GET_ITERATOR),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.error).not.toBeNull();
    expect(result.error!.message).toMatch(/not iterable/i);
  });

  it('CALL_ITERATOR_STEP with non-iterator is a no-op', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 5),
        instr(Opcode.CALL_ITERATOR_STEP),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(5);
    expect(result.error).toBeNull();
  });

  it('GET_ITERATOR_DONE returns true for non-object', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_NULL),
        instr(Opcode.GET_ITERATOR_DONE),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(true);
  });

  it('GET_ITERATOR_VALUE returns undefined for non-object', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_NULL),
        instr(Opcode.GET_ITERATOR_VALUE),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBeUndefined();
  });
});

describe('RETHROW', () => {
  it('RETHROW produces a VM error', () => {
    const vm = new RegisterVM();
    const result = vm.execute(code([instr(Opcode.RETHROW)]));
    expect(result.error).not.toBeNull();
    expect(result.error!.message).toContain('rethrow');
  });
});

describe('LDA_CONTEXT_SLOT / STA_CONTEXT_SLOT', () => {
  it('stores and loads a context slot at depth 0 explicitly', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.PUSH_CONTEXT, 2),
        instr(Opcode.LDA_SMI, 33),
        { opcode: Opcode.STA_CONTEXT_SLOT, operands: [0, 1], feedbackSlot: null },
        instr(Opcode.LDA_ZERO),
        { opcode: Opcode.LDA_CONTEXT_SLOT, operands: [0, 1], feedbackSlot: null },
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(33);
  });

  it('LDA_CONTEXT_SLOT returns undefined when no context', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        { opcode: Opcode.LDA_CONTEXT_SLOT, operands: [0, 0], feedbackSlot: null },
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBeUndefined();
  });

  it('STA_CONTEXT_SLOT with no context is a no-op', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 5),
        { opcode: Opcode.STA_CONTEXT_SLOT, operands: [0, 0], feedbackSlot: null },
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(5);
  });
});

describe('LDA_LOCAL / STA_LOCAL', () => {
  it('STA_LOCAL and LDA_LOCAL round-trip', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 77),
        { opcode: Opcode.STA_LOCAL, operands: [2], feedbackSlot: null },
        instr(Opcode.LDA_ZERO),
        { opcode: Opcode.LDA_LOCAL, operands: [2], feedbackSlot: null },
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(77);
  });
});

describe('objectWithHiddenClass', () => {
  it('creates a new object with a different hiddenClassId but same properties', () => {
    const obj = newObject();
    obj.properties.set('x', 1);
    const transitioned = objectWithHiddenClass(obj);
    expect(transitioned.hiddenClassId).not.toBe(obj.hiddenClassId);
    expect(transitioned.properties).toBe(obj.properties);
  });
});

describe('Additional comparison opcodes', () => {
  const vm = new RegisterVM();

  it('TEST_STRICT_EQUAL', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 3),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 3),
        instr(Opcode.TEST_STRICT_EQUAL, 0),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(true);
  });

  it('TEST_STRICT_NOT_EQUAL', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 3),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 4),
        instr(Opcode.TEST_STRICT_NOT_EQUAL, 0),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(true);
  });

  it('TEST_LESS_THAN_OR_EQUAL', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 5),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 5),
        instr(Opcode.TEST_LESS_THAN_OR_EQUAL, 0),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(true);
  });

  it('TEST_GREATER_THAN_OR_EQUAL', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 2),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 5),
        instr(Opcode.TEST_GREATER_THAN_OR_EQUAL, 0),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(true);
  });

  it('TEST_INSTANCEOF returns true for objects', () => {
    const result = vm.execute(
      code([
        instr(Opcode.CREATE_OBJECT_LITERAL),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDAR, 0),
        instr(Opcode.TEST_INSTANCEOF, 0),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(true);
  });

  it('JUMP_IF_UNDEFINED jumps when accumulator is undefined', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_UNDEFINED),
        instr(Opcode.JUMP_IF_UNDEFINED, 1),
        instr(Opcode.LDA_SMI, 1),
        instr(Opcode.LDA_SMI, 7),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(7);
  });

  it('JUMP_IF_NULL_OR_UNDEFINED jumps for undefined', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_UNDEFINED),
        instr(Opcode.JUMP_IF_NULL_OR_UNDEFINED, 1),
        instr(Opcode.LDA_SMI, 1),
        instr(Opcode.LDA_SMI, 9),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(9);
  });

  it('JUMP_IF_TO_BOOLEAN_TRUE and JUMP_IF_TO_BOOLEAN_FALSE', () => {
    const r1 = vm.execute(
      code([
        instr(Opcode.LDA_TRUE),
        instr(Opcode.JUMP_IF_TO_BOOLEAN_TRUE, 1),
        instr(Opcode.LDA_SMI, 1),
        instr(Opcode.LDA_SMI, 5),
        instr(Opcode.HALT),
      ]),
    );
    expect(r1.returnValue).toBe(5);

    const r2 = vm.execute(
      code([
        instr(Opcode.LDA_FALSE),
        instr(Opcode.JUMP_IF_TO_BOOLEAN_FALSE, 1),
        instr(Opcode.LDA_SMI, 1),
        instr(Opcode.LDA_SMI, 6),
        instr(Opcode.HALT),
      ]),
    );
    expect(r2.returnValue).toBe(6);
  });

  it('JUMP_LOOP jumps forward (positive offset)', () => {
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 5),
        instr(Opcode.JUMP_LOOP, 1),
        instr(Opcode.LDA_SMI, 99),
        instr(Opcode.ADD_SMI, 1),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(6);
  });
});

describe('Additional arithmetic edge cases', () => {
  it('ADD_SMI with non-number accumulator returns NaN', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([instr(Opcode.LDA_NULL), instr(Opcode.ADD_SMI, 1), instr(Opcode.HALT)]),
    );
    expect(result.returnValue).toBeNaN();
  });

  it('SUB_SMI with non-number accumulator returns NaN', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([instr(Opcode.LDA_NULL), instr(Opcode.SUB_SMI, 1), instr(Opcode.HALT)]),
    );
    expect(result.returnValue).toBeNaN();
  });

  it('NEGATE of non-number returns NaN', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([instr(Opcode.LDA_NULL), instr(Opcode.NEGATE), instr(Opcode.HALT)]),
    );
    expect(result.returnValue).toBeNaN();
  });

  it('SHIFT_RIGHT_LOGICAL', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.LDA_SMI, 1),
        instr(Opcode.STAR, 0),
        instr(Opcode.LDA_SMI, 16),
        instr(Opcode.SHIFT_RIGHT_LOGICAL, 0),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(8);
  });
});

describe('executeWithTrace error handling', () => {
  it('catches VMError and returns it in result', () => {
    const vm = new RegisterVM();
    const { result, trace } = vm.executeWithTrace(
      code(
        [instr(Opcode.LDA_CONSTANT, 0), instr(Opcode.THROW)],
        { constants: ['boom'] },
      ),
    );
    expect(result.error).not.toBeNull();
    expect(trace.length).toBeGreaterThan(0);
  });
});

describe('DEBUGGER opcode', () => {
  it('DEBUGGER is a no-op', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([instr(Opcode.LDA_SMI, 42), instr(Opcode.DEBUGGER), instr(Opcode.HALT)]),
    );
    expect(result.returnValue).toBe(42);
  });
});

describe('TYPEOF for string and boolean', () => {
  it('TYPEOF returns string for string value', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [instr(Opcode.LDA_CONSTANT, 0), instr(Opcode.TYPEOF), instr(Opcode.HALT)],
        { constants: ['hello'] },
      ),
    );
    expect(result.returnValue).toBe('string');
  });

  it('TYPEOF returns boolean for boolean value', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([instr(Opcode.LDA_TRUE), instr(Opcode.TYPEOF), instr(Opcode.HALT)]),
    );
    expect(result.returnValue).toBe('boolean');
  });

  it('TYPEOF returns function for a closure', () => {
    const innerCode: CodeObject = {
      name: 'fn',
      instructions: [instr(Opcode.RETURN)],
      constants: [],
      names: [],
      registerCount: 0,
      feedbackSlotCount: 0,
      parameterCount: 0,
    };
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [instr(Opcode.CREATE_CLOSURE, 0), instr(Opcode.TYPEOF), instr(Opcode.HALT)],
        { constants: [innerCode] },
      ),
    );
    expect(result.returnValue).toBe('function');
  });
});

describe('SUB / MUL / DIV with feedback slots', () => {
  it('SUB records feedback', () => {
    const vm = new RegisterVM();
    const { result, trace } = vm.executeWithTrace(
      code(
        [
          instr(Opcode.LDA_SMI, 10),
          instr(Opcode.STAR, 0),
          instr(Opcode.LDA_SMI, 3),
          { opcode: Opcode.SUB, operands: [0, 0], feedbackSlot: 0 },
          instr(Opcode.HALT),
        ],
        { feedbackSlotCount: 1 },
      ),
    );
    expect(result.returnValue).toBe(-7);
    const subStep = trace.find(s => s.instruction.opcode === Opcode.SUB);
    expect(subStep!.feedbackDelta.length).toBe(1);
  });

  it('MUL records feedback', () => {
    const vm = new RegisterVM();
    const { result } = vm.executeWithTrace(
      code(
        [
          instr(Opcode.LDA_SMI, 6),
          instr(Opcode.STAR, 0),
          instr(Opcode.LDA_SMI, 7),
          { opcode: Opcode.MUL, operands: [0, 0], feedbackSlot: 0 },
          instr(Opcode.HALT),
        ],
        { feedbackSlotCount: 1 },
      ),
    );
    expect(result.returnValue).toBe(42);
  });

  it('DIV records feedback', () => {
    const vm = new RegisterVM();
    const { result } = vm.executeWithTrace(
      code(
        [
          instr(Opcode.LDA_SMI, 2),
          instr(Opcode.STAR, 0),
          instr(Opcode.LDA_SMI, 10),
          { opcode: Opcode.DIV, operands: [0, 0], feedbackSlot: 0 },
          instr(Opcode.HALT),
        ],
        { feedbackSlotCount: 1 },
      ),
    );
    expect(result.returnValue).toBe(5);
  });
});

describe('toBoolean edge cases via JUMP_IF_FALSE', () => {
  it('empty string is falsy', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_CONSTANT, 0),
          instr(Opcode.JUMP_IF_FALSE, 1),
          instr(Opcode.LDA_SMI, 1),
          instr(Opcode.LDA_SMI, 99),
          instr(Opcode.HALT),
        ],
        { constants: [''] },
      ),
    );
    expect(result.returnValue).toBe(99);
  });

  it('NaN is falsy', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code(
        [
          instr(Opcode.LDA_CONSTANT, 0),
          instr(Opcode.JUMP_IF_FALSE, 1),
          instr(Opcode.LDA_SMI, 1),
          instr(Opcode.LDA_SMI, 88),
          instr(Opcode.HALT),
        ],
        { constants: [NaN] },
      ),
    );
    expect(result.returnValue).toBe(88);
  });

  it('object is truthy', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        instr(Opcode.CREATE_OBJECT_LITERAL),
        instr(Opcode.JUMP_IF_TRUE, 1),
        instr(Opcode.LDA_SMI, 1),
        instr(Opcode.LDA_SMI, 77),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(77);
  });
});

describe('CALL_ANY_RECEIVER feedback slot recording', () => {
  it('records call-site feedback when slot is defined', () => {
    const innerCode: CodeObject = {
      name: 'inner',
      instructions: [instr(Opcode.LDA_SMI, 1), instr(Opcode.RETURN)],
      constants: [],
      names: [],
      registerCount: 0,
      feedbackSlotCount: 0,
      parameterCount: 0,
    };

    const vm = new RegisterVM();
    const { trace } = vm.executeWithTrace(
      code(
        [
          instr(Opcode.CREATE_CLOSURE, 0),
          instr(Opcode.STAR, 0),
          { opcode: Opcode.CALL_ANY_RECEIVER, operands: [0, 1, 0, 0], feedbackSlot: null },
          instr(Opcode.HALT),
        ],
        { constants: [innerCode], feedbackSlotCount: 1 },
      ),
    );

    const callStep = trace.find(s => s.instruction.opcode === Opcode.CALL_ANY_RECEIVER);
    expect(callStep).toBeDefined();
    expect(callStep!.feedbackDelta.length).toBe(1);
  });
});

describe('CREATE_CONTEXT opcode', () => {
  it('pushes a new context with given slot count', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([
        { opcode: Opcode.CREATE_CONTEXT, operands: [3], feedbackSlot: null },
        instr(Opcode.LDA_SMI, 7),
        instr(Opcode.STA_CURRENT_CONTEXT_SLOT, 2),
        instr(Opcode.LDA_CURRENT_CONTEXT_SLOT, 2),
        instr(Opcode.HALT),
      ]),
    );
    expect(result.returnValue).toBe(7);
  });
});

describe('CLONE_OBJECT on non-object', () => {
  it('leaves accumulator unchanged for a non-object value', () => {
    const vm = new RegisterVM();
    const result = vm.execute(
      code([instr(Opcode.LDA_SMI, 42), instr(Opcode.CLONE_OBJECT), instr(Opcode.HALT)]),
    );
    expect(result.returnValue).toBe(42);
  });
});
