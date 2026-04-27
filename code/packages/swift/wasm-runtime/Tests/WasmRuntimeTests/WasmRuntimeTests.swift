import XCTest
@testable import WasmRuntime
@testable import WasmExecution
@testable import WasmTypes

// ============================================================================
// WasmRuntimeTests -- End-to-end tests for the complete WASM pipeline
// ============================================================================

final class WasmRuntimeTests: XCTestCase {

    // ========================================================================
    // MARK: - Test Binaries
    // ========================================================================

    // The "square" function: takes an i32, returns i32 = x * x.
    //
    // WAT source:
    //   (module
    //     (func $square (export "square") (param i32) (result i32)
    //       local.get 0
    //       local.get 0
    //       i32.mul))
    //
    // Binary layout:
    //   00 61 73 6D    magic (\0asm)
    //   01 00 00 00    version 1
    //   01 06          type section (6 bytes)
    //     01           1 type
    //     60 01 7F 01 7F   func type: (i32) -> (i32)
    //   03 02          function section (2 bytes)
    //     01 00        1 function, type index 0
    //   07 0A          export section (10 bytes)
    //     01           1 export
    //     06 73 71 75 61 72 65  "square"
    //     00 00        function, index 0
    //   0A 09          code section (9 bytes)
    //     01           1 body
    //     07           body size 7
    //     00           0 local declarations
    //     20 00        local.get 0
    //     20 00        local.get 0
    //     6C           i32.mul
    //     0B           end
    static let squareWasm: [UInt8] = [
        0x00, 0x61, 0x73, 0x6D,  // magic
        0x01, 0x00, 0x00, 0x00,  // version
        // Type section
        0x01, 0x06,
        0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F,
        // Function section
        0x03, 0x02, 0x01, 0x00,
        // Export section
        0x07, 0x0A,
        0x01, 0x06,
        0x73, 0x71, 0x75, 0x61, 0x72, 0x65,  // "square"
        0x00, 0x00,
        // Code section
        0x0A, 0x09,
        0x01, 0x07, 0x00,
        0x20, 0x00,  // local.get 0
        0x20, 0x00,  // local.get 0
        0x6C,        // i32.mul
        0x0B,        // end
    ]

    // The "add" function: takes two i32, returns i32 = a + b.
    //
    // WAT:
    //   (module
    //     (func $add (export "add") (param i32 i32) (result i32)
    //       local.get 0
    //       local.get 1
    //       i32.add))
    static let addWasm: [UInt8] = [
        0x00, 0x61, 0x73, 0x6D,
        0x01, 0x00, 0x00, 0x00,
        // Type section: (i32, i32) -> (i32)
        0x01, 0x07,
        0x01, 0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7F,
        // Function section
        0x03, 0x02, 0x01, 0x00,
        // Export section
        0x07, 0x07,
        0x01, 0x03,
        0x61, 0x64, 0x64,  // "add"
        0x00, 0x00,
        // Code section
        0x0A, 0x09,
        0x01, 0x07, 0x00,
        0x20, 0x00,  // local.get 0
        0x20, 0x01,  // local.get 1
        0x6A,        // i32.add
        0x0B,        // end
    ]

    // The "factorial" function with a local variable and loop:
    //
    // WAT:
    //   (module
    //     (func $factorial (export "factorial") (param i32) (result i32)
    //       (local i32)
    //       i32.const 1
    //       local.set 1
    //       block
    //         loop
    //           local.get 0
    //           i32.eqz
    //           br_if 1
    //           local.get 1
    //           local.get 0
    //           i32.mul
    //           local.set 1
    //           local.get 0
    //           i32.const 1
    //           i32.sub
    //           local.set 0
    //           br 0
    //         end
    //       end))
    static let factorialWasm: [UInt8] = [
        0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
        // Type section: (i32) -> (i32)
        0x01, 0x06, 0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F,
        // Function section
        0x03, 0x02, 0x01, 0x00,
        // Export section: "factorial" -> func 0
        0x07, 0x0D, 0x01, 0x09,
        0x66, 0x61, 0x63, 0x74, 0x6F, 0x72, 0x69, 0x61, 0x6C,
        0x00, 0x00,
        // Code section
        0x0A, 0x27,        // section id=10, size=39
        0x01,              // 1 body
        0x25,              // body size=37
        0x01, 0x01, 0x7F,  // 1 local of type i32
        0x41, 0x01,        // i32.const 1
        0x21, 0x01,        // local.set 1
        0x02, 0x40,        // block (empty)
        0x03, 0x40,        // loop (empty)
        0x20, 0x00,        // local.get 0
        0x45,              // i32.eqz
        0x0D, 0x01,        // br_if 1
        0x20, 0x01,        // local.get 1
        0x20, 0x00,        // local.get 0
        0x6C,              // i32.mul
        0x21, 0x01,        // local.set 1
        0x20, 0x00,        // local.get 0
        0x41, 0x01,        // i32.const 1
        0x6B,              // i32.sub
        0x21, 0x00,        // local.set 0
        0x0C, 0x00,        // br 0
        0x0B,              // end (loop)
        0x0B,              // end (block)
        0x20, 0x01,        // local.get 1
        0x0B,              // end (function)
    ]

    // ========================================================================
    // MARK: - End-to-End Tests
    // ========================================================================

    /// The flagship test: square(5) = 25.
    func testSquare5Equals25() throws {
        let runtime = WasmRuntime()
        let result = try runtime.loadAndRun(
            WasmRuntimeTests.squareWasm,
            entry: "square",
            args: [5]
        )
        XCTAssertEqual(result, [25])
    }

    /// Test square with other values.
    func testSquareVarious() throws {
        let runtime = WasmRuntime()
        XCTAssertEqual(try runtime.loadAndRun(WasmRuntimeTests.squareWasm, entry: "square", args: [0]), [0])
        XCTAssertEqual(try runtime.loadAndRun(WasmRuntimeTests.squareWasm, entry: "square", args: [1]), [1])
        XCTAssertEqual(try runtime.loadAndRun(WasmRuntimeTests.squareWasm, entry: "square", args: [7]), [49])
        XCTAssertEqual(try runtime.loadAndRun(WasmRuntimeTests.squareWasm, entry: "square", args: [10]), [100])
    }

    /// Test add function.
    func testAdd() throws {
        let runtime = WasmRuntime()
        let result = try runtime.loadAndRun(
            WasmRuntimeTests.addWasm,
            entry: "add",
            args: [3, 4]
        )
        XCTAssertEqual(result, [7])
    }

    /// Test factorial function (uses block/loop/br_if).
    func testFactorial() throws {
        let runtime = WasmRuntime()
        XCTAssertEqual(try runtime.loadAndRun(WasmRuntimeTests.factorialWasm, entry: "factorial", args: [5]), [120])
        XCTAssertEqual(try runtime.loadAndRun(WasmRuntimeTests.factorialWasm, entry: "factorial", args: [1]), [1])
        XCTAssertEqual(try runtime.loadAndRun(WasmRuntimeTests.factorialWasm, entry: "factorial", args: [0]), [1])
        XCTAssertEqual(try runtime.loadAndRun(WasmRuntimeTests.factorialWasm, entry: "factorial", args: [10]), [3628800])
    }

    // ========================================================================
    // MARK: - Step-by-Step Pipeline Tests
    // ========================================================================

    func testParseValidateInstantiate() throws {
        let runtime = WasmRuntime()
        let module = try runtime.load(WasmRuntimeTests.squareWasm)

        // Parse check.
        XCTAssertEqual(module.types.count, 1)
        XCTAssertEqual(module.functions.count, 1)
        XCTAssertEqual(module.exports.count, 1)
        XCTAssertEqual(module.code.count, 1)

        // Validate.
        let validated = try runtime.validateModule(module)
        XCTAssertEqual(validated.funcTypes.count, 1)

        // Instantiate.
        let instance = try runtime.instantiate(module)
        XCTAssertNotNil(instance.exports["square"])
        XCTAssertEqual(instance.funcTypes.count, 1)
    }

    func testExportNotFound() throws {
        let runtime = WasmRuntime()
        XCTAssertThrowsError(try runtime.loadAndRun(
            WasmRuntimeTests.squareWasm,
            entry: "nonexistent",
            args: []
        ))
    }

    // ========================================================================
    // MARK: - WasiStub Tests
    // ========================================================================

    func testWasiStubCreation() {
        let wasi = WasiStub()
        XCTAssertEqual(wasi.stdoutOutput, "")
        XCTAssertNil(wasi.exitCode)
    }

    func testWasiResolvesKnownFunctions() {
        let wasi = WasiStub()
        let fdWrite = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "fd_write")
        XCTAssertNotNil(fdWrite)

        let fdRead = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "fd_read")
        XCTAssertNotNil(fdRead)

        let procExit = wasi.resolveFunction(moduleName: "wasi_snapshot_preview1", name: "proc_exit")
        XCTAssertNotNil(procExit)
    }

    func testWasiHostAliasExists() {
        let host = WasiHost()
        XCTAssertTrue(type(of: host) == WasiStub.self)
    }

    func testWasiReturnsNilForNonWasi() {
        let wasi = WasiStub()
        let result = wasi.resolveFunction(moduleName: "env", name: "something")
        XCTAssertNil(result)
    }

    // ========================================================================
    // MARK: - WasmInstance Tests
    // ========================================================================

    func testInstanceExportLookup() throws {
        let runtime = WasmRuntime()
        let module = try runtime.load(WasmRuntimeTests.addWasm)
        let instance = try runtime.instantiate(module)
        XCTAssertNotNil(instance.exports["add"])
        XCTAssertNil(instance.exports["nonexistent"])
    }
}
