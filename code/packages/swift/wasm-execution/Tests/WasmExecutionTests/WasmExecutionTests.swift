import XCTest
@testable import WasmExecution
@testable import WasmTypes
@testable import VirtualMachine

// ============================================================================
// WasmExecutionTests -- Tests for the WASM execution engine
// ============================================================================

final class WasmExecutionTests: XCTestCase {

    // ========================================================================
    // MARK: - WasmValue Tests
    // ========================================================================

    func testModuleLoads() {
        let _ = WasmExecution()
        XCTAssertTrue(true, "WasmExecution instantiated successfully")
    }

    func testWasmValueTypes() {
        let i32val = WasmValue.i32(42)
        XCTAssertEqual(i32val.type, .i32)

        let i64val = WasmValue.i64(100)
        XCTAssertEqual(i64val.type, .i64)

        let f32val = WasmValue.f32(3.14)
        XCTAssertEqual(f32val.type, .f32)

        let f64val = WasmValue.f64(2.71828)
        XCTAssertEqual(f64val.type, .f64)
    }

    func testWasmValueDefaultValue() {
        XCTAssertEqual(WasmValue.defaultValue(for: .i32), .i32(0))
        XCTAssertEqual(WasmValue.defaultValue(for: .i64), .i64(0))
        XCTAssertEqual(WasmValue.defaultValue(for: .f32), .f32(0))
        XCTAssertEqual(WasmValue.defaultValue(for: .f64), .f64(0))
    }

    func testWasmValueAsI32() throws {
        let val = WasmValue.i32(42)
        XCTAssertEqual(try val.asI32(), 42)

        let wrong = WasmValue.i64(42)
        XCTAssertThrowsError(try wrong.asI32())
    }

    func testWasmValueNumeric() {
        XCTAssertEqual(WasmValue.i32(5).numericValue, 5.0)
        XCTAssertEqual(WasmValue.i64(10).numericValue, 10.0)
        XCTAssertEqual(WasmValue.f32(3.5).numericValue, Double(Float(3.5)))
        XCTAssertEqual(WasmValue.f64(2.5).numericValue, 2.5)
    }

    func testWasmValueTypedRoundTrip() {
        let original = WasmValue.i32(42)
        let typed = original.typed
        let recovered = WasmValue.fromTyped(typed)
        XCTAssertEqual(original, recovered)
    }

    // ========================================================================
    // MARK: - LinearMemory Tests
    // ========================================================================

    func testLinearMemoryBasics() throws {
        let mem = LinearMemory(initialPages: 1)
        XCTAssertEqual(mem.size(), 1)
        XCTAssertEqual(mem.byteLength(), 65536)

        try mem.storeI32(0, 42)
        XCTAssertEqual(try mem.loadI32(0), 42)
    }

    func testLinearMemoryGrow() {
        let mem = LinearMemory(initialPages: 1, maxPages: 3)
        XCTAssertEqual(mem.grow(1), 1)
        XCTAssertEqual(mem.size(), 2)
        XCTAssertEqual(mem.grow(1), 2)
        XCTAssertEqual(mem.size(), 3)
        XCTAssertEqual(mem.grow(1), -1)  // Exceeds max
    }

    func testLinearMemoryBoundsCheck() {
        let mem = LinearMemory(initialPages: 1)
        XCTAssertThrowsError(try mem.loadI32(65536))
        XCTAssertThrowsError(try mem.loadI32(-1))
    }

    func testLinearMemoryNarrowLoads() throws {
        let mem = LinearMemory(initialPages: 1)
        try mem.storeI32(0, 0x01020304)
        XCTAssertEqual(try mem.loadI32_8u(0), 0x04)
        XCTAssertEqual(try mem.loadI32_16u(0), 0x0304)
    }

    func testLinearMemoryWriteBytes() throws {
        let mem = LinearMemory(initialPages: 1)
        try mem.writeBytes(0, [0x48, 0x65, 0x6C, 0x6C, 0x6F])
        XCTAssertEqual(try mem.loadI32_8u(0), 0x48)  // 'H'
        XCTAssertEqual(try mem.loadI32_8u(4), 0x6F)  // 'o'
    }

    // ========================================================================
    // MARK: - Table Tests
    // ========================================================================

    func testTableBasics() throws {
        let table = Table(initialSize: 10)
        XCTAssertEqual(table.tableSize(), 10)
        XCTAssertNil(try table.get(0))

        try table.set(0, 42)
        XCTAssertEqual(try table.get(0), 42)
    }

    func testTableBoundsCheck() {
        let table = Table(initialSize: 5)
        XCTAssertThrowsError(try table.get(5))
        XCTAssertThrowsError(try table.set(5, 0))
    }

    // ========================================================================
    // MARK: - Constant Expression Tests
    // ========================================================================

    func testConstExprI32() throws {
        let expr: [UInt8] = [0x41, 0x2A, 0x0B]  // i32.const 42, end
        let result = try evaluateConstExpr(expr, globals: [])
        XCTAssertEqual(result, .i32(42))
    }

    func testConstExprI64() throws {
        let expr: [UInt8] = [0x42, 0xC8, 0x01, 0x0B]  // i64.const 200, end
        let result = try evaluateConstExpr(expr, globals: [])
        XCTAssertEqual(result, .i64(200))
    }

    func testConstExprGlobalGet() throws {
        let expr: [UInt8] = [0x23, 0x00, 0x0B]  // global.get 0, end
        let result = try evaluateConstExpr(expr, globals: [.i32(99)])
        XCTAssertEqual(result, .i32(99))
    }

    // ========================================================================
    // MARK: - Bytecode Decoder Tests
    // ========================================================================

    func testDecodeFunctionBody() throws {
        // local.get 0, local.get 0, i32.mul, end
        let body = FunctionBody(locals: [], code: [0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B])
        let decoded = try decodeFunctionBody(body)
        XCTAssertEqual(decoded.count, 4)
        XCTAssertEqual(decoded[0].opcode, 0x20)  // local.get
        XCTAssertEqual(decoded[1].opcode, 0x20)  // local.get
        XCTAssertEqual(decoded[2].opcode, 0x6C)  // i32.mul
        XCTAssertEqual(decoded[3].opcode, 0x0B)  // end
    }

    // ========================================================================
    // MARK: - Control Flow Map Tests
    // ========================================================================

    func testControlFlowMap() throws {
        // block, nop, end
        let body = FunctionBody(locals: [], code: [0x02, 0x40, 0x01, 0x0B, 0x0B])
        let decoded = try decodeFunctionBody(body)
        let cfMap = buildControlFlowMap(decoded)
        XCTAssertNotNil(cfMap[0])
        XCTAssertEqual(cfMap[0]?.endPc, 3)
    }

    // ========================================================================
    // MARK: - Engine Tests
    // ========================================================================

    func testEngineDirectExecution() throws {
        // Simple function: i32.const 5, i32.const 5, i32.mul, end
        let body = FunctionBody(locals: [], code: [
            0x41, 0x05,  // i32.const 5
            0x41, 0x05,  // i32.const 5
            0x6C,        // i32.mul
            0x0B,        // end
        ])
        let funcType = FuncType(params: [], results: [.i32])
        let engine = WasmExecutionEngine(
            memory: nil, tables: [], globals: [],
            globalTypes: [], funcTypes: [funcType],
            funcBodies: [body], hostFunctions: [nil]
        )
        let result = try engine.callFunction(0, [])
        XCTAssertEqual(result, [.i32(25)])
    }

    func testEngineWithLocals() throws {
        // Function: param i32 -> result i32, returns param * param
        let body = FunctionBody(locals: [], code: [
            0x20, 0x00,  // local.get 0
            0x20, 0x00,  // local.get 0
            0x6C,        // i32.mul
            0x0B,        // end
        ])
        let funcType = FuncType(params: [.i32], results: [.i32])
        let engine = WasmExecutionEngine(
            memory: nil, tables: [], globals: [],
            globalTypes: [], funcTypes: [funcType],
            funcBodies: [body], hostFunctions: [nil]
        )
        let result = try engine.callFunction(0, [.i32(7)])
        XCTAssertEqual(result, [.i32(49)])
    }

    func testHostFunction() throws {
        let hostFunc = HostFunction(
            type: FuncType(params: [.i32], results: [.i32]),
            call: { args in
                if case .i32(let v) = args[0] { return [.i32(v &* 2)] }
                return [.i32(0)]
            }
        )
        let engine = WasmExecutionEngine(
            memory: nil, tables: [], globals: [],
            globalTypes: [], funcTypes: [hostFunc.type],
            funcBodies: [nil], hostFunctions: [hostFunc]
        )
        let result = try engine.callFunction(0, [.i32(21)])
        XCTAssertEqual(result, [.i32(42)])
    }

    func testI32Arithmetic() throws {
        // i32.const 10, i32.const 3, i32.sub => 7
        let body = FunctionBody(locals: [], code: [
            0x41, 0x0A,  // i32.const 10
            0x41, 0x03,  // i32.const 3
            0x6B,        // i32.sub
            0x0B,        // end
        ])
        let engine = WasmExecutionEngine(
            memory: nil, tables: [], globals: [],
            globalTypes: [], funcTypes: [FuncType(params: [], results: [.i32])],
            funcBodies: [body], hostFunctions: [nil]
        )
        let result = try engine.callFunction(0, [])
        XCTAssertEqual(result, [.i32(7)])
    }

    func testI32Comparison() throws {
        // i32.const 5, i32.const 3, i32.gt_s => 1
        let body = FunctionBody(locals: [], code: [
            0x41, 0x05,  // i32.const 5
            0x41, 0x03,  // i32.const 3
            0x4A,        // i32.gt_s
            0x0B,        // end
        ])
        let engine = WasmExecutionEngine(
            memory: nil, tables: [], globals: [],
            globalTypes: [], funcTypes: [FuncType(params: [], results: [.i32])],
            funcBodies: [body], hostFunctions: [nil]
        )
        let result = try engine.callFunction(0, [])
        XCTAssertEqual(result, [.i32(1)])
    }

    func testBlockAndBranchIf() throws {
        // if (param == 0) return 42 else return param
        // block, local.get 0, i32.eqz, br_if 0, local.get 0, return, end, i32.const 42, end
        let body = FunctionBody(locals: [], code: [
            0x02, 0x40,        // block (empty)
            0x20, 0x00,        // local.get 0
            0x45,              // i32.eqz
            0x0D, 0x00,        // br_if 0
            0x20, 0x00,        // local.get 0
            0x0F,              // return
            0x0B,              // end block
            0x41, 0x2A,        // i32.const 42
            0x0B,              // end function
        ])
        let funcType = FuncType(params: [.i32], results: [.i32])
        let engine = WasmExecutionEngine(
            memory: nil, tables: [], globals: [],
            globalTypes: [], funcTypes: [funcType],
            funcBodies: [body], hostFunctions: [nil]
        )
        // When param is 0, should return 42
        let result0 = try engine.callFunction(0, [.i32(0)])
        XCTAssertEqual(result0, [.i32(42)])

        // When param is non-zero, should return param
        let result5 = try engine.callFunction(0, [.i32(5)])
        XCTAssertEqual(result5, [.i32(5)])
    }

    func testSelect() throws {
        // select: i32.const 10, i32.const 20, i32.const 1, select => 10
        let body = FunctionBody(locals: [], code: [
            0x41, 0x0A,  // i32.const 10
            0x41, 0x14,  // i32.const 20
            0x41, 0x01,  // i32.const 1
            0x1B,        // select
            0x0B,        // end
        ])
        let engine = WasmExecutionEngine(
            memory: nil, tables: [], globals: [],
            globalTypes: [], funcTypes: [FuncType(params: [], results: [.i32])],
            funcBodies: [body], hostFunctions: [nil]
        )
        let result = try engine.callFunction(0, [])
        XCTAssertEqual(result, [.i32(10)])
    }

    func testDrop() throws {
        // i32.const 10, i32.const 20, drop => 10
        let body = FunctionBody(locals: [], code: [
            0x41, 0x0A,  // i32.const 10
            0x41, 0x14,  // i32.const 20
            0x1A,        // drop
            0x0B,        // end
        ])
        let engine = WasmExecutionEngine(
            memory: nil, tables: [], globals: [],
            globalTypes: [], funcTypes: [FuncType(params: [], results: [.i32])],
            funcBodies: [body], hostFunctions: [nil]
        )
        let result = try engine.callFunction(0, [])
        XCTAssertEqual(result, [.i32(10)])
    }

    func testGlobalGetSet() throws {
        // global.get 0, i32.const 1, i32.add, global.set 0, global.get 0, end
        let body = FunctionBody(locals: [], code: [
            0x23, 0x00,  // global.get 0
            0x41, 0x01,  // i32.const 1
            0x6A,        // i32.add
            0x24, 0x00,  // global.set 0
            0x23, 0x00,  // global.get 0
            0x0B,        // end
        ])
        let engine = WasmExecutionEngine(
            memory: nil, tables: [], globals: [.i32(10)],
            globalTypes: [GlobalType(valueType: .i32, mutable: true)],
            funcTypes: [FuncType(params: [], results: [.i32])],
            funcBodies: [body], hostFunctions: [nil]
        )
        let result = try engine.callFunction(0, [])
        XCTAssertEqual(result, [.i32(11)])
    }

    func testMemoryLoadStore() throws {
        // i32.const 0, i32.const 42, i32.store, i32.const 0, i32.load
        let body = FunctionBody(locals: [], code: [
            0x41, 0x00,          // i32.const 0 (address)
            0x41, 0x2A,          // i32.const 42 (value)
            0x36, 0x02, 0x00,    // i32.store align=2 offset=0
            0x41, 0x00,          // i32.const 0 (address)
            0x28, 0x02, 0x00,    // i32.load align=2 offset=0
            0x0B,                // end
        ])
        let mem = LinearMemory(initialPages: 1)
        let engine = WasmExecutionEngine(
            memory: mem, tables: [], globals: [],
            globalTypes: [], funcTypes: [FuncType(params: [], results: [.i32])],
            funcBodies: [body], hostFunctions: [nil]
        )
        let result = try engine.callFunction(0, [])
        XCTAssertEqual(result, [.i32(42)])
    }
}
