import XCTest
@testable import WasmTypes

final class WasmTypesTests: XCTestCase {

    func testValueTypeRawValues() {
        XCTAssertEqual(ValueType.i32.rawValue, 0x7F)
        XCTAssertEqual(ValueType.i64.rawValue, 0x7E)
        XCTAssertEqual(ValueType.f32.rawValue, 0x7D)
        XCTAssertEqual(ValueType.f64.rawValue, 0x7C)
    }

    func testExternalKindRawValues() {
        XCTAssertEqual(ExternalKind.function.rawValue, 0x00)
        XCTAssertEqual(ExternalKind.table.rawValue, 0x01)
        XCTAssertEqual(ExternalKind.memory.rawValue, 0x02)
        XCTAssertEqual(ExternalKind.global.rawValue, 0x03)
    }

    func testFuncTypeEquality() {
        let t1 = FuncType(params: [.i32, .i32], results: [.i32])
        let t2 = FuncType(params: [.i32, .i32], results: [.i32])
        let t3 = FuncType(params: [.i32], results: [.i32])
        XCTAssertEqual(t1, t2)
        XCTAssertNotEqual(t1, t3)
    }

    func testLimits() {
        let l1 = Limits(min: 1, max: nil)
        let l2 = Limits(min: 1, max: 10)
        XCTAssertEqual(l1.min, 1)
        XCTAssertNil(l1.max)
        XCTAssertEqual(l2.max, 10)
    }

    func testWasmModuleInit() {
        let mod = WasmModule()
        XCTAssertTrue(mod.types.isEmpty)
        XCTAssertTrue(mod.imports.isEmpty)
        XCTAssertTrue(mod.functions.isEmpty)
        XCTAssertNil(mod.start)
    }

    func testGlobalType() {
        let gt = GlobalType(valueType: .i32, mutable: true)
        XCTAssertEqual(gt.valueType, .i32)
        XCTAssertTrue(gt.mutable)
    }

    func testExport() {
        let exp = Export(name: "main", kind: .function, index: 0)
        XCTAssertEqual(exp.name, "main")
        XCTAssertEqual(exp.kind, .function)
        XCTAssertEqual(exp.index, 0)
    }

    func testFunctionBody() {
        let body = FunctionBody(locals: [.i32, .f64], code: [0x20, 0x00, 0x0B])
        XCTAssertEqual(body.locals.count, 2)
        XCTAssertEqual(body.code.count, 3)
    }

    func testConstants() {
        XCTAssertEqual(FUNCREF, 0x70)
        XCTAssertEqual(BLOCK_TYPE_EMPTY, 0x40)
    }
}
