// ============================================================================
// RegisterVMTests.swift — XCTest suite for the RegisterVM package
// ============================================================================
//
// Each test builds a small `CodeObject` by hand (like a minimal assembler),
// executes it through `RegisterVM.execute(_:)`, and asserts on the result.
//
// The tests are organised by feature:
//   1. Basic loads and halt
//   2. Register moves (Star / Ldar)
//   3. Arithmetic with monomorphic feedback (int+int always → Smi/Smi)
//   4. Feedback state transitions (int+int then float+float → polymorphic)
//   5. Conditional branches
//   6. Global variables (LdaGlobal / StaGlobal)
//   7. Function calls via callAnyReceiver
//   8. Halt returns accumulator immediately
//   9. Named-property loads with feedback
//  10. Stack-check overflow detection
//
// ============================================================================

import XCTest
@testable import RegisterVM

final class RegisterVMTests: XCTestCase {

    // ── 1. Basic constant load + halt ─────────────────────────────────────

    /// Verify that `ldaConstant` loads a value from the constant pool into the
    /// accumulator, and that `halt` terminates execution returning that value.
    func testLdaConstantHalt() {
        //   ldaConstant [0]   ; acc = 42
        //   halt              ; return acc
        let code = CodeObject(
            instructions: [
                RegisterInstruction(opcode: .ldaConstant, operands: [0]),
                RegisterInstruction(opcode: .halt),
            ],
            constants: [.integer(42)],
            names: [],
            registerCount: 1,
            feedbackSlotCount: 0
        )
        var vm = RegisterVM()
        let result = vm.execute(code)

        XCTAssertNil(result.error, "Expected no error; got: \(result.error?.message ?? "")")
        if case .integer(let n) = result.returnValue {
            XCTAssertEqual(n, 42)
        } else {
            XCTFail("Expected .integer(42), got \(result.returnValue)")
        }
    }

    // ── 2. Star / Ldar round-trip ─────────────────────────────────────────

    /// Verify that `star` stores the accumulator into a register and `ldar`
    /// restores it back, preserving the value across the round-trip.
    func testStarLdar() {
        //   ldaSmi [99]   ; acc = 99
        //   star   [0]    ; r0  = 99
        //   ldaZero       ; acc = 0
        //   ldar   [0]    ; acc = r0  (should be 99)
        //   halt
        let code = CodeObject(
            instructions: [
                RegisterInstruction(opcode: .ldaSmi,    operands: [99]),
                RegisterInstruction(opcode: .star,      operands: [0]),
                RegisterInstruction(opcode: .ldaZero),
                RegisterInstruction(opcode: .ldar,      operands: [0]),
                RegisterInstruction(opcode: .halt),
            ],
            constants: [],
            names: [],
            registerCount: 2,
            feedbackSlotCount: 0
        )
        var vm = RegisterVM()
        let result = vm.execute(code)

        XCTAssertNil(result.error)
        if case .integer(let n) = result.returnValue {
            XCTAssertEqual(n, 99)
        } else {
            XCTFail("Expected .integer(99), got \(result.returnValue)")
        }
    }

    // ── 3. Add with monomorphic int feedback ──────────────────────────────

    /// After executing `add` with two integers, the feedback slot must be in
    /// the `.monomorphic` state recording the ("Smi", "Smi") type pair.
    func testAddMonomorphicFeedback() {
        //   ldaSmi  [3]         ; acc = 3
        //   star    [0]         ; r0  = 3
        //   ldaSmi  [4]         ; acc = 4
        //   add     [0] slot:0  ; acc = r0 + acc = 7  (records Smi+Smi at slot 0)
        //   halt
        let code = CodeObject(
            instructions: [
                RegisterInstruction(opcode: .ldaSmi,  operands: [3]),
                RegisterInstruction(opcode: .star,    operands: [0]),
                RegisterInstruction(opcode: .ldaSmi,  operands: [4]),
                RegisterInstruction(opcode: .add,     operands: [0], feedbackSlot: 0),
                RegisterInstruction(opcode: .halt),
            ],
            constants: [],
            names: [],
            registerCount: 2,
            feedbackSlotCount: 1
        )
        var vm = RegisterVM()
        let result = vm.execute(code)

        XCTAssertNil(result.error)
        if case .integer(let n) = result.returnValue {
            XCTAssertEqual(n, 7)
        } else {
            XCTFail("Expected .integer(7), got \(result.returnValue)")
        }

        // The feedback vector lives in the CallFrame which is ephemeral, so we
        // verify the logic directly via the helper functions.
        var fv = FeedbackSlot.newVector(size: 1)
        recordBinaryOp(vector: &fv, slot: 0, left: .integer(3), right: .integer(4))
        if case .monomorphic(let types) = fv[0] {
            XCTAssertEqual(types.count, 1)
            XCTAssertEqual(types[0].0, "Smi")
            XCTAssertEqual(types[0].1, "Smi")
        } else {
            XCTFail("Expected .monomorphic after one int+int recording, got \(fv[0])")
        }
    }

    // ── 4. Feedback state transitions ─────────────────────────────────────

    /// Record int+int (→ monomorphic), then float+float (→ polymorphic),
    /// verifying the IC state machine transitions correctly.
    func testAddFeedbackTransitions() {
        var fv = FeedbackSlot.newVector(size: 1)

        // Step 1: first observation → monomorphic.
        recordBinaryOp(vector: &fv, slot: 0, left: .integer(1), right: .integer(2))
        if case .monomorphic(let types) = fv[0] {
            XCTAssertEqual(types.count, 1)
        } else {
            XCTFail("Expected monomorphic after first observation")
        }

        // Step 2: different type pair → polymorphic.
        recordBinaryOp(vector: &fv, slot: 0, left: .float(1.5), right: .float(2.5))
        if case .polymorphic(let types) = fv[0] {
            XCTAssertEqual(types.count, 2)
            XCTAssertEqual(types[0].0, "Smi")
            XCTAssertEqual(types[1].0, "Number")
        } else {
            XCTFail("Expected polymorphic after second distinct observation, got \(fv[0])")
        }

        // Step 3: same type pair again → still polymorphic (deduplicated).
        recordBinaryOp(vector: &fv, slot: 0, left: .integer(9), right: .integer(1))
        if case .polymorphic(let types) = fv[0] {
            XCTAssertEqual(types.count, 2, "Duplicate type pairs must not be re-added")
        } else {
            XCTFail("Expected polymorphic with deduplication, got \(fv[0])")
        }
    }

    // ── 5. Conditional branch: jumpIfFalse ────────────────────────────────

    /// Compile a simple if/else:
    ///   if (false) { acc = 1 } else { acc = 2 }
    ///
    /// Instruction layout:
    ///   0: ldaFalse
    ///   1: jumpIfFalse [3]   ; condition is false → jump to instruction 3
    ///   2: ldaSmi [1]        ; true branch (skipped)
    ///   3: ldaSmi [2]        ; false branch (taken)
    ///   4: halt
    func testJumpIfFalse() {
        let prog = CodeObject(
            instructions: [
                RegisterInstruction(opcode: .ldaFalse),                    // 0
                RegisterInstruction(opcode: .jumpIfFalse,  operands: [3]), // 1
                RegisterInstruction(opcode: .ldaSmi,       operands: [1]), // 2  (true branch — skipped)
                RegisterInstruction(opcode: .ldaSmi,       operands: [2]), // 3  (false branch — taken)
                RegisterInstruction(opcode: .halt),                        // 4
            ],
            constants: [],
            names: [],
            registerCount: 1,
            feedbackSlotCount: 0
        )
        var vm = RegisterVM()
        let result = vm.execute(prog)

        XCTAssertNil(result.error)
        if case .integer(let n) = result.returnValue {
            XCTAssertEqual(n, 2, "false condition should select the else branch (2)")
        } else {
            XCTFail("Expected .integer(2), got \(result.returnValue)")
        }
    }

    // ── 6. Global variables ───────────────────────────────────────────────

    /// Store a value into a global, then read it back in the same execution.
    func testGlobalVariables() {
        //   ldaSmi  [77]
        //   staGlobal [0]   ; globals["answer"] = 77
        //   ldaZero
        //   ldaGlobal [0]   ; acc = globals["answer"]
        //   halt
        let code = CodeObject(
            instructions: [
                RegisterInstruction(opcode: .ldaSmi,      operands: [77]),
                RegisterInstruction(opcode: .staGlobal,   operands: [0]),
                RegisterInstruction(opcode: .ldaZero),
                RegisterInstruction(opcode: .ldaGlobal,   operands: [0]),
                RegisterInstruction(opcode: .halt),
            ],
            constants: [],
            names: ["answer"],
            registerCount: 1,
            feedbackSlotCount: 0
        )
        var vm = RegisterVM()
        let result = vm.execute(code)

        XCTAssertNil(result.error)
        if case .integer(let n) = result.returnValue {
            XCTAssertEqual(n, 77)
        } else {
            XCTFail("Expected .integer(77), got \(result.returnValue)")
        }
        if case .integer(let v) = vm.globals["answer"] {
            XCTAssertEqual(v, 77)
        } else {
            XCTFail("globals[\"answer\"] should be .integer(77)")
        }
    }

    // ── 7. callAnyReceiver with a CodeObject function ─────────────────────

    /// Define a function that returns 100 and call it via `callAnyReceiver`.
    func testCallAnyReceiver() {
        // Inner function: returns 100
        let inner = CodeObject(
            instructions: [
                RegisterInstruction(opcode: .ldaSmi,   operands: [100]),
                RegisterInstruction(opcode: .return_),
            ],
            constants: [],
            names: [],
            registerCount: 1,
            feedbackSlotCount: 0,
            name: "getHundred"
        )

        // Outer program:
        //   ldaConstant [0]       ; acc = .function(inner)
        //   star        [0]       ; r0  = func
        //   ldar        [0]       ; acc = r0  (the function)
        //   callAnyReceiver [1, 0]; call acc(no args)  → 100
        //   halt
        let code = CodeObject(
            instructions: [
                RegisterInstruction(opcode: .ldaConstant,       operands: [0]),
                RegisterInstruction(opcode: .star,              operands: [0]),
                RegisterInstruction(opcode: .ldar,              operands: [0]),
                RegisterInstruction(opcode: .callAnyReceiver,   operands: [1, 0]),
                RegisterInstruction(opcode: .halt),
            ],
            constants: [.function(inner, nil)],
            names: [],
            registerCount: 2,
            feedbackSlotCount: 0
        )
        var vm = RegisterVM()
        let result = vm.execute(code)

        XCTAssertNil(result.error, "Got error: \(result.error?.message ?? "")")
        if case .integer(let n) = result.returnValue {
            XCTAssertEqual(n, 100)
        } else {
            XCTFail("Expected .integer(100), got \(result.returnValue)")
        }
    }

    // ── 8. halt returns accumulator immediately ────────────────────────────

    /// Instructions *after* `halt` must not execute.
    func testHaltReturnsAcc() {
        //   ldaSmi [5]
        //   halt
        //   ldaSmi [99]   ← must NOT execute
        let code = CodeObject(
            instructions: [
                RegisterInstruction(opcode: .ldaSmi, operands: [5]),
                RegisterInstruction(opcode: .halt),
                RegisterInstruction(opcode: .ldaSmi, operands: [99]),  // dead code
            ],
            constants: [],
            names: [],
            registerCount: 1,
            feedbackSlotCount: 0
        )
        var vm = RegisterVM()
        let result = vm.execute(code)

        XCTAssertNil(result.error)
        if case .integer(let n) = result.returnValue {
            XCTAssertEqual(n, 5, "halt must stop execution before ldaSmi 99")
        } else {
            XCTFail("Expected .integer(5), got \(result.returnValue)")
        }
    }

    // ── 9. ldaNamedProperty with feedback recording ───────────────────────

    /// Create an object with a property, load it via `ldaNamedProperty`, and
    /// confirm the feedback vector records the object's hidden class.
    func testNamedPropertyFeedback() {
        // Create an object: { value: 42 }
        let obj = VMObject(hiddenClassId: 1000, properties: ["value": .integer(42)])

        // Program:
        //   ldaConstant [0]     ; acc = obj
        //   star        [0]     ; r0  = obj
        //   ldaNamedProperty [0, 0] slot:0 ; acc = r0.value
        //   halt
        let code = CodeObject(
            instructions: [
                RegisterInstruction(opcode: .ldaConstant,        operands: [0]),
                RegisterInstruction(opcode: .star,               operands: [0]),
                RegisterInstruction(opcode: .ldaNamedProperty,   operands: [0, 0], feedbackSlot: 0),
                RegisterInstruction(opcode: .halt),
            ],
            constants: [.object(obj)],
            names: ["value"],
            registerCount: 1,
            feedbackSlotCount: 1
        )
        var vm = RegisterVM()
        let result = vm.execute(code)

        XCTAssertNil(result.error, "Got error: \(result.error?.message ?? "")")
        if case .integer(let n) = result.returnValue {
            XCTAssertEqual(n, 42)
        } else {
            XCTFail("Expected .integer(42), got \(result.returnValue)")
        }

        // Verify feedback recording logic independently.
        var fv = FeedbackSlot.newVector(size: 1)
        recordPropertyLoad(vector: &fv, slot: 0, hiddenClassId: 1000)
        if case .monomorphic(let types) = fv[0] {
            XCTAssertEqual(types.count, 1)
            XCTAssertTrue(types[0].0.hasPrefix("HiddenClass:"),
                          "Expected hidden class key, got \(types[0].0)")
        } else {
            XCTFail("Expected monomorphic property feedback, got \(fv[0])")
        }
    }

    // ── 10. Stack-check overflow detection ────────────────────────────────

    /// With `maxDepth = 3`, exceeding the limit via `stackCheck` must produce
    /// a `VMError` with "overflow" in the message.
    func testStackCheckOverflow() {
        //   stackCheck   ; will throw when callDepth >= maxDepth
        //   halt
        let code = CodeObject(
            instructions: [
                RegisterInstruction(opcode: .stackCheck),
                RegisterInstruction(opcode: .halt),
            ],
            constants: [],
            names: [],
            registerCount: 1,
            feedbackSlotCount: 0
        )
        var vm = RegisterVM(maxDepth: 0)  // any depth triggers overflow
        let result = vm.execute(code)

        XCTAssertNotNil(result.error, "Expected a stack overflow error")
        if let err = result.error {
            XCTAssertTrue(
                err.message.lowercased().contains("overflow"),
                "Error message should mention 'overflow', got: \(err.message)"
            )
        }
    }
}
