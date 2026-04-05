import XCTest
@testable import WasmOpcodes

final class WasmOpcodesTests: XCTestCase {

    func testGetOpcodeByByte() {
        let info = getOpcode(0x6A)
        XCTAssertNotNil(info)
        XCTAssertEqual(info!.name, "i32.add")
        XCTAssertEqual(info!.opcode, 0x6A)
        XCTAssertEqual(info!.category, "numeric_i32")
        XCTAssertEqual(info!.stackPop, 2)
        XCTAssertEqual(info!.stackPush, 1)
    }

    func testGetOpcodeByName() {
        let info = getOpcodeByName("i32.add")
        XCTAssertNotNil(info)
        XCTAssertEqual(info!.opcode, 0x6A)
    }

    func testUnknownOpcode() {
        XCTAssertNil(getOpcode(0xFF))
    }

    func testUnknownName() {
        XCTAssertNil(getOpcodeByName("i32.foo"))
    }

    func testControlFlowOpcodes() {
        XCTAssertEqual(getOpcode(0x00)?.name, "unreachable")
        XCTAssertEqual(getOpcode(0x01)?.name, "nop")
        XCTAssertEqual(getOpcode(0x02)?.name, "block")
        XCTAssertEqual(getOpcode(0x03)?.name, "loop")
        XCTAssertEqual(getOpcode(0x04)?.name, "if")
        XCTAssertEqual(getOpcode(0x0B)?.name, "end")
        XCTAssertEqual(getOpcode(0x10)?.name, "call")
    }

    func testI32ConstHasImmediate() {
        let info = getOpcode(0x41)
        XCTAssertNotNil(info)
        XCTAssertEqual(info!.immediates, ["i32"])
        XCTAssertEqual(info!.stackPop, 0)
        XCTAssertEqual(info!.stackPush, 1)
    }

    func testMemoryOpcodes() {
        let info = getOpcode(0x28)  // i32.load
        XCTAssertNotNil(info)
        XCTAssertEqual(info!.immediates, ["memarg"])
        XCTAssertEqual(info!.stackPop, 1)
        XCTAssertEqual(info!.stackPush, 1)
    }

    func testTotalOpcodeCount() {
        // WASM 1.0 has ~183 instructions
        XCTAssertGreaterThan(OPCODES.count, 170)
    }
}
