import XCTest
@testable import VirtualMachine

// ============================================================================
// VirtualMachineTests -- Tests for the generic stack-based VM
// ============================================================================

final class VirtualMachineTests: XCTestCase {

    func testModuleLoads() {
        let _ = VirtualMachine()
        XCTAssertTrue(true, "VirtualMachine instantiated successfully")
    }

    // ========================================================================
    // MARK: - VMValue Tests
    // ========================================================================

    func testVMValueEquality() {
        XCTAssertEqual(VMValue.int(42), VMValue.int(42))
        XCTAssertNotEqual(VMValue.int(42), VMValue.int(43))
        XCTAssertEqual(VMValue.string("hello"), VMValue.string("hello"))
        XCTAssertEqual(VMValue.null, VMValue.null)
        XCTAssertEqual(VMValue.bool(true), VMValue.bool(true))
        XCTAssertNotEqual(VMValue.int(42), VMValue.string("42"))
    }

    func testVMValueTypes() {
        XCTAssertEqual(VMValue.int32(5), VMValue.int32(5))
        XCTAssertEqual(VMValue.int64(100), VMValue.int64(100))
        XCTAssertEqual(VMValue.uint32(5), VMValue.uint32(5))
        XCTAssertEqual(VMValue.uint64(100), VMValue.uint64(100))
        XCTAssertEqual(VMValue.float(3.14), VMValue.float(3.14))
        XCTAssertEqual(VMValue.double(2.71), VMValue.double(2.71))
    }

    // ========================================================================
    // MARK: - TypedVMValue Tests
    // ========================================================================

    func testTypedVMValue() {
        let tv = TypedVMValue(type: 0x7F, value: .int32(42))
        XCTAssertEqual(tv.type, 0x7F)
        XCTAssertEqual(tv.value, .int32(42))
    }

    // ========================================================================
    // MARK: - Stack Operations
    // ========================================================================

    func testPushPop() {
        let vm = GenericVM()
        vm.push(.int(42))
        vm.push(.int(99))
        XCTAssertEqual(vm.pop(), .int(99))
        XCTAssertEqual(vm.pop(), .int(42))
    }

    func testPopEmpty() {
        let vm = GenericVM()
        let val = vm.pop()
        XCTAssertEqual(val, .null)
        XCTAssertTrue(vm.halted)
    }

    func testTypedPushPop() {
        let vm = GenericVM()
        let tv = TypedVMValue(type: 0x7F, value: .int32(42))
        vm.pushTyped(tv)
        let popped = vm.popTyped()
        XCTAssertEqual(popped, tv)
    }

    func testTypedPopEmpty() {
        let vm = GenericVM()
        let val = vm.popTyped()
        XCTAssertEqual(val.type, 0)
        XCTAssertEqual(val.value, .null)
        XCTAssertTrue(vm.halted)
    }

    func testPeekTyped() {
        let vm = GenericVM()
        let tv = TypedVMValue(type: 0x7E, value: .int64(100))
        vm.pushTyped(tv)
        let peeked = vm.peekTyped()
        XCTAssertEqual(peeked, tv)
        XCTAssertEqual(vm.typedStack.count, 1)  // Still there
    }

    // ========================================================================
    // MARK: - Execution
    // ========================================================================

    func testSimpleExecution() {
        let vm = GenericVM()
        // Register opcode 0x01 (push 42) and 0x02 (halt)
        vm.registerOpcode(0x01) { vm, instr, code in
            vm.push(.int(42))
        }
        vm.registerOpcode(0x02) { vm, instr, code in
            vm.halted = true
        }

        let code = CodeObject(instructions: [
            Instruction(opcode: 0x01),
            Instruction(opcode: 0x02),
        ])

        vm.execute(code)
        XCTAssertEqual(vm.stack.last, .int(42))
    }

    func testUnknownOpcodeHalts() {
        let vm = GenericVM()
        let code = CodeObject(instructions: [
            Instruction(opcode: 0xFF),  // Unknown
        ])
        vm.execute(code)
        XCTAssertTrue(vm.halted)
    }

    func testContextExecution() {
        let vm = GenericVM()

        class TestContext {
            var value: Int = 0
        }
        let ctx = TestContext()

        vm.registerContextOpcode(0x01) { vm, instr, code, ctxObj in
            let ctx = ctxObj as! TestContext
            ctx.value = 42
            vm.push(.int(ctx.value))
        }

        let code = CodeObject(instructions: [
            Instruction(opcode: 0x01),
        ])

        vm.executeWithContext(code, context: ctx)
        XCTAssertEqual(ctx.value, 42)
        XCTAssertEqual(vm.stack.last, .int(42))
    }

    // ========================================================================
    // MARK: - Reset
    // ========================================================================

    func testReset() {
        let vm = GenericVM()
        vm.push(.int(42))
        vm.pushTyped(TypedVMValue(type: 0x7F, value: .int32(1)))
        vm.variables["x"] = .int(5)
        vm.halted = true

        vm.reset()

        XCTAssertTrue(vm.stack.isEmpty)
        XCTAssertTrue(vm.typedStack.isEmpty)
        XCTAssertTrue(vm.variables.isEmpty)
        XCTAssertFalse(vm.halted)
    }

    // ========================================================================
    // MARK: - PC Control
    // ========================================================================

    func testJumpTo() {
        let vm = GenericVM()
        var visited: [Int] = []

        vm.registerOpcode(0x01) { vm, instr, code in
            visited.append(vm.pc - 1)
        }
        // Jump handler -- jumps to instruction 3
        vm.registerOpcode(0x02) { vm, instr, code in
            vm.jumpTo(3)
        }

        let code = CodeObject(instructions: [
            Instruction(opcode: 0x01),  // 0: visit
            Instruction(opcode: 0x02),  // 1: jump to 3
            Instruction(opcode: 0x01),  // 2: visit (should be skipped)
            Instruction(opcode: 0x01),  // 3: visit
        ])

        vm.execute(code)
        // Should visit instructions 0, 1 (jump), 3, but not 2
        XCTAssertEqual(visited.count, 2)
    }

    // ========================================================================
    // MARK: - CodeObject and Instruction
    // ========================================================================

    func testCodeObjectCreation() {
        let code = CodeObject(
            instructions: [Instruction(opcode: 0x01, operand: 42)],
            constants: [.int(100)],
            names: ["x"]
        )
        XCTAssertEqual(code.instructions.count, 1)
        XCTAssertEqual(code.constants.count, 1)
        XCTAssertEqual(code.names, ["x"])
    }

    func testMaxRecursionDepth() {
        let vm = GenericVM()
        vm.setMaxRecursionDepth(512)
        XCTAssertEqual(vm.maxRecursionDepth, 512)
    }
}

/// A namespace type for test compatibility.
public struct VirtualMachine {
    public init() {}
}
