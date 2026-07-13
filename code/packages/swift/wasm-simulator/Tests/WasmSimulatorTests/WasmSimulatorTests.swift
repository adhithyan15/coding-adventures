import XCTest

@testable import WasmSimulator

final class WasmSimulatorTests: XCTestCase {
  func testVersionAndEncodingHelpers() {
    XCTAssertEqual(WasmSimulatorVersion.version, "0.1.0")
    XCTAssertEqual(encodeI32Add(), [0x6A])
    XCTAssertEqual(encodeLocalSet(2), [0x21, 0x02])
    XCTAssertEqual(encodeI32Const(-2), [0x41, 0xFE, 0xFF, 0xFF, 0xFF])
  }

  func testDecoderReadsSignedI32Const() throws {
    let instruction = try WasmDecoder().decode(encodeI32Const(-42), at: 0)

    XCTAssertEqual(instruction.mnemonic, "i32.const")
    XCTAssertEqual(instruction.operand, -42)
    XCTAssertEqual(instruction.size, 5)
  }

  func testSimulatorRunsSimpleAdditionProgram() throws {
    let simulator = WasmSimulator(localCount: 2)
    let program = assembleWasm([
      encodeI32Const(1),
      encodeI32Const(2),
      encodeI32Add(),
      encodeLocalSet(0),
      encodeEnd(),
    ])

    let traces = try simulator.run(program)

    XCTAssertEqual(traces.count, 5)
    XCTAssertEqual(traces[2].stackBefore, [1, 2])
    XCTAssertEqual(traces[2].stackAfter, [3])
    XCTAssertEqual(simulator.locals[0], 3)
    XCTAssertEqual(simulator.stack, [])
    XCTAssertEqual(simulator.pc, program.count)
    XCTAssertEqual(simulator.cycle, 5)
    XCTAssertTrue(simulator.halted)
  }

  func testLocalGetRestoresStoredValue() throws {
    let simulator = WasmSimulator(localCount: 2)
    let program = assembleWasm([
      encodeI32Const(42),
      encodeLocalSet(1),
      encodeLocalGet(1),
      encodeEnd(),
    ])

    let traces = try simulator.run(program)

    XCTAssertEqual(traces[2].stackAfter, [42])
    XCTAssertEqual(traces[2].localsSnapshot, [0, 42])
    XCTAssertEqual(simulator.stack, [42])
  }

  func testSubtractionUsesWrappingI32Arithmetic() throws {
    let simulator = WasmSimulator()
    let program = assembleWasm([
      encodeI32Const(.min),
      encodeI32Const(1),
      encodeI32Sub(),
      encodeEnd(),
    ])

    _ = try simulator.run(program)

    XCTAssertEqual(simulator.stack, [.max])
  }

  func testRunHonorsStepLimit() throws {
    let simulator = WasmSimulator()
    let program = assembleWasm([
      encodeI32Const(1),
      encodeI32Const(2),
      encodeI32Add(),
      encodeEnd(),
    ])

    let traces = try simulator.run(program, maxSteps: 2)

    XCTAssertEqual(traces.count, 2)
    XCTAssertEqual(simulator.stack, [1, 2])
    XCTAssertFalse(simulator.halted)
  }

  func testLoadAndResetClearState() throws {
    let simulator = WasmSimulator(localCount: 1)
    _ = try simulator.run(
      assembleWasm([
        encodeI32Const(7),
        encodeLocalSet(0),
        encodeEnd(),
      ]))

    simulator.load(assembleWasm([encodeI32Const(3), encodeEnd()]))
    XCTAssertEqual(simulator.locals, [0])
    XCTAssertEqual(simulator.stack, [])
    XCTAssertEqual(simulator.pc, 0)
    XCTAssertEqual(simulator.cycle, 0)
    XCTAssertFalse(simulator.halted)

    _ = try simulator.step()
    simulator.reset()
    XCTAssertEqual(simulator.bytecode, [])
    XCTAssertEqual(simulator.stack, [])
    XCTAssertEqual(simulator.pc, 0)
    XCTAssertEqual(simulator.cycle, 0)
  }

  func testDecoderRejectsUnknownAndTruncatedInstructions() {
    XCTAssertThrowsError(try WasmDecoder().decode([0xFF], at: 0)) { error in
      XCTAssertEqual(error as? WasmSimulatorError, .unknownOpcode(0xFF, pc: 0))
    }
    XCTAssertThrowsError(try WasmDecoder().decode([WasmOpcode.i32Const, 0x01], at: 0)) { error in
      XCTAssertEqual(
        error as? WasmSimulatorError,
        .truncatedInstruction(pc: 0, expectedBytes: 5, availableBytes: 2)
      )
    }
  }

  func testSimulatorReportsExecutionErrors() throws {
    let simulator = WasmSimulator(localCount: 1)

    XCTAssertThrowsError(try simulator.run(assembleWasm([encodeI32Add()]))) { error in
      XCTAssertEqual(error as? WasmSimulatorError, .stackUnderflow)
    }
    XCTAssertThrowsError(
      try simulator.run(assembleWasm([encodeLocalGet(1)]))
    ) { error in
      XCTAssertEqual(error as? WasmSimulatorError, .localIndexOutOfRange(1))
    }

    _ = try simulator.run(assembleWasm([encodeEnd()]))
    XCTAssertThrowsError(try simulator.step()) { error in
      XCTAssertEqual(error as? WasmSimulatorError, .halted)
    }
  }

  func testExecutorRejectsMissingOperands() {
    var stack: [Int32] = []
    var locals: [Int32] = [0]
    let malformed = WasmInstruction(
      opcode: WasmOpcode.i32Const,
      mnemonic: "i32.const",
      operand: nil,
      size: 5
    )

    XCTAssertThrowsError(
      try WasmExecutor().execute(malformed, stack: &stack, locals: &locals, pc: 0)
    ) { error in
      XCTAssertEqual(error as? WasmSimulatorError, .missingOperand("i32.const"))
    }
  }
}
