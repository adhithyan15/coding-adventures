import XCTest
@testable import WasmValidator
@testable import WasmTypes

// ============================================================================
// WasmValidatorTests -- Tests for the WASM structural validator
// ============================================================================

final class WasmValidatorTests: XCTestCase {

    func testModuleLoads() {
        // WasmValidator is a namespace with the validate() function.
        // The stub test creates a WasmValidator struct -- we just need
        // the module to be importable.
        let _ = WasmValidator()
        XCTAssertTrue(true, "WasmValidator instantiated successfully")
    }

    func testValidEmptyModule() throws {
        let module = WasmModule()
        let validated = try validate(module)
        XCTAssertEqual(validated.funcTypes.count, 0)
    }

    func testValidSimpleModule() throws {
        let module = WasmModule()
        module.types = [FuncType(params: [.i32], results: [.i32])]
        module.functions = [0]
        module.code = [FunctionBody(locals: [], code: [0x20, 0x00, 0x0B])]
        module.exports = [Export(name: "f", kind: .function, index: 0)]

        let validated = try validate(module)
        XCTAssertEqual(validated.funcTypes.count, 1)
        XCTAssertEqual(validated.funcTypes[0].params, [.i32])
    }

    func testInvalidTypeIndex() {
        let module = WasmModule()
        module.types = [FuncType(params: [], results: [])]
        module.functions = [5]  // Invalid: only 1 type (index 0)
        module.code = [FunctionBody(locals: [], code: [0x0B])]

        XCTAssertThrowsError(try validate(module)) { error in
            if case ValidationError.invalidTypeIndex(5) = error {
                // Expected
            } else {
                XCTFail("Expected invalidTypeIndex(5), got \(error)")
            }
        }
    }

    func testMultipleMemories() {
        let module = WasmModule()
        module.memories = [
            MemoryType(limits: Limits(min: 1)),
            MemoryType(limits: Limits(min: 1)),
        ]

        XCTAssertThrowsError(try validate(module)) { error in
            if case ValidationError.multipleMemories = error {
                // Expected
            } else {
                XCTFail("Expected multipleMemories, got \(error)")
            }
        }
    }

    func testMultipleTables() {
        let module = WasmModule()
        module.tables = [
            TableType(elementType: 0x70, limits: Limits(min: 1)),
            TableType(elementType: 0x70, limits: Limits(min: 1)),
        ]

        XCTAssertThrowsError(try validate(module)) { error in
            if case ValidationError.multipleTables = error {
                // Expected
            } else {
                XCTFail("Expected multipleTables, got \(error)")
            }
        }
    }

    func testMemoryLimitExceeded() {
        let module = WasmModule()
        module.memories = [MemoryType(limits: Limits(min: 70000))]

        XCTAssertThrowsError(try validate(module)) { error in
            if case ValidationError.memoryLimitExceeded = error {
                // Expected
            } else {
                XCTFail("Expected memoryLimitExceeded, got \(error)")
            }
        }
    }

    func testMemoryLimitOrder() {
        let module = WasmModule()
        module.memories = [MemoryType(limits: Limits(min: 10, max: 5))]

        XCTAssertThrowsError(try validate(module)) { error in
            if case ValidationError.memoryLimitOrder = error {
                // Expected
            } else {
                XCTFail("Expected memoryLimitOrder, got \(error)")
            }
        }
    }

    func testDuplicateExportName() {
        let module = WasmModule()
        module.types = [FuncType(params: [], results: [])]
        module.functions = [0, 0]
        module.code = [
            FunctionBody(locals: [], code: [0x0B]),
            FunctionBody(locals: [], code: [0x0B]),
        ]
        module.exports = [
            Export(name: "f", kind: .function, index: 0),
            Export(name: "f", kind: .function, index: 1),
        ]

        XCTAssertThrowsError(try validate(module)) { error in
            if case ValidationError.duplicateExportName("f") = error {
                // Expected
            } else {
                XCTFail("Expected duplicateExportName, got \(error)")
            }
        }
    }

    func testExportIndexOutOfRange() {
        let module = WasmModule()
        module.types = [FuncType(params: [], results: [])]
        module.functions = [0]
        module.code = [FunctionBody(locals: [], code: [0x0B])]
        module.exports = [
            Export(name: "f", kind: .function, index: 5),  // Out of range
        ]

        XCTAssertThrowsError(try validate(module)) { error in
            if case ValidationError.exportIndexOutOfRange = error {
                // Expected
            } else {
                XCTFail("Expected exportIndexOutOfRange, got \(error)")
            }
        }
    }

    func testFunctionCodeMismatch() {
        let module = WasmModule()
        module.types = [FuncType(params: [], results: [])]
        module.functions = [0, 0]
        module.code = [FunctionBody(locals: [], code: [0x0B])]  // Only 1 body for 2 functions

        XCTAssertThrowsError(try validate(module)) { error in
            if case ValidationError.functionCodeMismatch = error {
                // Expected
            } else {
                XCTFail("Expected functionCodeMismatch, got \(error)")
            }
        }
    }

    func testStartFunctionValidation() throws {
        // Valid: start function with [] -> [] signature
        let module = WasmModule()
        module.types = [FuncType(params: [], results: [])]
        module.functions = [0]
        module.code = [FunctionBody(locals: [], code: [0x0B])]
        module.start = 0

        let validated = try validate(module)
        XCTAssertEqual(validated.module.start, 0)
    }

    func testStartFunctionBadType() {
        // Invalid: start function with [i32] -> [] signature
        let module = WasmModule()
        module.types = [FuncType(params: [.i32], results: [])]
        module.functions = [0]
        module.code = [FunctionBody(locals: [], code: [0x0B])]
        module.start = 0

        XCTAssertThrowsError(try validate(module)) { error in
            if case ValidationError.startFunctionBadType = error {
                // Expected
            } else {
                XCTFail("Expected startFunctionBadType, got \(error)")
            }
        }
    }

    func testTableLimitOrder() {
        let module = WasmModule()
        module.tables = [TableType(elementType: 0x70, limits: Limits(min: 10, max: 5))]

        XCTAssertThrowsError(try validate(module)) { error in
            if case ValidationError.tableLimitOrder = error {
                // Expected
            } else {
                XCTFail("Expected tableLimitOrder, got \(error)")
            }
        }
    }

    func testImportedFunctionTypeValidation() {
        let module = WasmModule()
        module.types = [FuncType(params: [], results: [])]
        module.imports = [
            Import(moduleName: "env", name: "f", kind: .function,
                   typeInfo: .function(typeIndex: 5))  // Invalid type index
        ]

        XCTAssertThrowsError(try validate(module)) { error in
            if case ValidationError.invalidTypeIndex(5) = error {
                // Expected
            } else {
                XCTFail("Expected invalidTypeIndex, got \(error)")
            }
        }
    }

    func testValidatedModuleContainsFuncTypes() throws {
        let module = WasmModule()
        module.types = [
            FuncType(params: [.i32], results: [.i32]),
            FuncType(params: [.i32, .i32], results: [.i32]),
        ]
        module.imports = [
            Import(moduleName: "env", name: "f", kind: .function,
                   typeInfo: .function(typeIndex: 0))
        ]
        module.functions = [1]
        module.code = [FunctionBody(locals: [], code: [0x0B])]

        let validated = try validate(module)
        // 1 imported + 1 module-defined
        XCTAssertEqual(validated.funcTypes.count, 2)
        XCTAssertEqual(validated.funcTypes[0].params, [.i32])
        XCTAssertEqual(validated.funcTypes[1].params, [.i32, .i32])
    }
}
