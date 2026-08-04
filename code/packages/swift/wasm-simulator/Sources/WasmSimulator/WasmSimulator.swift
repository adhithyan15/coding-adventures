public enum WasmSimulatorVersion {
  public static let version = "0.1.0"
}

public enum WasmOpcode {
  public static let end: UInt8 = 0x0B
  public static let localGet: UInt8 = 0x20
  public static let localSet: UInt8 = 0x21
  public static let i32Const: UInt8 = 0x41
  public static let i32Add: UInt8 = 0x6A
  public static let i32Sub: UInt8 = 0x6B
}

public enum WasmSimulatorError: Error, Equatable, CustomStringConvertible {
  case halted
  case localIndexOutOfRange(Int)
  case missingOperand(String)
  case stackUnderflow
  case truncatedInstruction(pc: Int, expectedBytes: Int, availableBytes: Int)
  case unknownOpcode(UInt8, pc: Int)

  public var description: String {
    switch self {
    case .halted:
      return "WASM simulator has halted"
    case .localIndexOutOfRange(let index):
      return "Local index \(index) is out of range"
    case .missingOperand(let mnemonic):
      return "WASM instruction \(mnemonic) requires an operand"
    case .stackUnderflow:
      return "Stack underflow"
    case .truncatedInstruction(let pc, let expectedBytes, let availableBytes):
      return
        "Truncated WASM instruction at PC=\(pc): expected \(expectedBytes) byte(s), found \(availableBytes)"
    case .unknownOpcode(let opcode, let pc):
      let rawHex = String(opcode, radix: 16, uppercase: true)
      let hex = rawHex.count == 1 ? "0\(rawHex)" : rawHex
      return "Unknown WASM opcode 0x\(hex) at PC=\(pc)"
    }
  }
}

public struct WasmInstruction: Equatable, Sendable {
  public let opcode: UInt8
  public let mnemonic: String
  public let operand: Int32?
  public let size: Int

  public init(opcode: UInt8, mnemonic: String, operand: Int32?, size: Int) {
    self.opcode = opcode
    self.mnemonic = mnemonic
    self.operand = operand
    self.size = size
  }
}

public struct WasmStepTrace: Equatable, Sendable {
  public let pc: Int
  public let instruction: WasmInstruction
  public let stackBefore: [Int32]
  public let stackAfter: [Int32]
  public let localsSnapshot: [Int32]
  public let description: String
  public let halted: Bool

  public init(
    pc: Int,
    instruction: WasmInstruction,
    stackBefore: [Int32],
    stackAfter: [Int32],
    localsSnapshot: [Int32],
    description: String,
    halted: Bool
  ) {
    self.pc = pc
    self.instruction = instruction
    self.stackBefore = stackBefore
    self.stackAfter = stackAfter
    self.localsSnapshot = localsSnapshot
    self.description = description
    self.halted = halted
  }
}

public struct WasmDecoder: Sendable {
  public init() {}

  public func decode(_ bytecode: [UInt8], at pc: Int) throws -> WasmInstruction {
    try requireBytes(bytecode, at: pc, count: 1)
    let opcode = bytecode[pc]

    switch opcode {
    case WasmOpcode.i32Const:
      try requireBytes(bytecode, at: pc, count: 5)
      let bits =
        UInt32(bytecode[pc + 1])
        | (UInt32(bytecode[pc + 2]) << 8)
        | (UInt32(bytecode[pc + 3]) << 16)
        | (UInt32(bytecode[pc + 4]) << 24)
      return WasmInstruction(
        opcode: opcode,
        mnemonic: "i32.const",
        operand: Int32(bitPattern: bits),
        size: 5
      )
    case WasmOpcode.i32Add:
      return WasmInstruction(opcode: opcode, mnemonic: "i32.add", operand: nil, size: 1)
    case WasmOpcode.i32Sub:
      return WasmInstruction(opcode: opcode, mnemonic: "i32.sub", operand: nil, size: 1)
    case WasmOpcode.localGet:
      try requireBytes(bytecode, at: pc, count: 2)
      return WasmInstruction(
        opcode: opcode,
        mnemonic: "local.get",
        operand: Int32(bytecode[pc + 1]),
        size: 2
      )
    case WasmOpcode.localSet:
      try requireBytes(bytecode, at: pc, count: 2)
      return WasmInstruction(
        opcode: opcode,
        mnemonic: "local.set",
        operand: Int32(bytecode[pc + 1]),
        size: 2
      )
    case WasmOpcode.end:
      return WasmInstruction(opcode: opcode, mnemonic: "end", operand: nil, size: 1)
    default:
      throw WasmSimulatorError.unknownOpcode(opcode, pc: pc)
    }
  }

  private func requireBytes(_ bytecode: [UInt8], at pc: Int, count: Int) throws {
    let available = pc >= 0 && pc < bytecode.count ? bytecode.count - pc : 0
    guard pc >= 0, available >= count else {
      throw WasmSimulatorError.truncatedInstruction(
        pc: pc,
        expectedBytes: count,
        availableBytes: available
      )
    }
  }
}

public struct WasmExecutor: Sendable {
  public init() {}

  public func execute(
    _ instruction: WasmInstruction,
    stack: inout [Int32],
    locals: inout [Int32],
    pc: Int
  ) throws -> WasmStepTrace {
    let stackBefore = stack
    let halted: Bool
    let description: String

    switch instruction.opcode {
    case WasmOpcode.i32Const:
      guard let value = instruction.operand else {
        throw WasmSimulatorError.missingOperand(instruction.mnemonic)
      }
      stack.append(value)
      description = "push \(value)"
      halted = false
    case WasmOpcode.i32Add:
      guard stack.count >= 2 else { throw WasmSimulatorError.stackUnderflow }
      let right = stack.removeLast()
      let left = stack.removeLast()
      let result = left &+ right
      stack.append(result)
      description = "pop \(right) and \(left), push \(result)"
      halted = false
    case WasmOpcode.i32Sub:
      guard stack.count >= 2 else { throw WasmSimulatorError.stackUnderflow }
      let right = stack.removeLast()
      let left = stack.removeLast()
      let result = left &- right
      stack.append(result)
      description = "pop \(right) and \(left), push \(result)"
      halted = false
    case WasmOpcode.localGet:
      let index = try checkedLocalIndex(instruction, locals: locals)
      stack.append(locals[index])
      description = "push local[\(index)]"
      halted = false
    case WasmOpcode.localSet:
      let index = try checkedLocalIndex(instruction, locals: locals)
      guard let value = stack.popLast() else { throw WasmSimulatorError.stackUnderflow }
      locals[index] = value
      description = "store \(value) in local[\(index)]"
      halted = false
    case WasmOpcode.end:
      description = "halt"
      halted = true
    default:
      throw WasmSimulatorError.unknownOpcode(instruction.opcode, pc: pc)
    }

    return WasmStepTrace(
      pc: pc,
      instruction: instruction,
      stackBefore: stackBefore,
      stackAfter: stack,
      localsSnapshot: locals,
      description: description,
      halted: halted
    )
  }

  private func checkedLocalIndex(_ instruction: WasmInstruction, locals: [Int32]) throws -> Int {
    guard let operand = instruction.operand else {
      throw WasmSimulatorError.missingOperand(instruction.mnemonic)
    }
    let index = Int(operand)
    guard locals.indices.contains(index) else {
      throw WasmSimulatorError.localIndexOutOfRange(index)
    }
    return index
  }
}

public final class WasmSimulator {
  private let decoder = WasmDecoder()
  private let executor = WasmExecutor()

  public private(set) var stack: [Int32] = []
  public private(set) var locals: [Int32]
  public private(set) var pc = 0
  public private(set) var cycle = 0
  public private(set) var halted = false
  public private(set) var bytecode: [UInt8] = []

  public init(localCount: Int = 16) {
    precondition(localCount >= 0, "localCount must be non-negative")
    locals = Array(repeating: 0, count: localCount)
  }

  public func load(_ program: [UInt8]) {
    bytecode = program
    pc = 0
    cycle = 0
    halted = false
    stack.removeAll(keepingCapacity: true)
    locals = Array(repeating: 0, count: locals.count)
  }

  @discardableResult
  public func step() throws -> WasmStepTrace {
    guard !halted else { throw WasmSimulatorError.halted }

    let instruction = try decoder.decode(bytecode, at: pc)
    let trace = try executor.execute(
      instruction,
      stack: &stack,
      locals: &locals,
      pc: pc
    )
    pc += instruction.size
    cycle += 1
    halted = trace.halted
    return trace
  }

  public func run(_ program: [UInt8], maxSteps: Int = 1_000) throws -> [WasmStepTrace] {
    precondition(maxSteps >= 0, "maxSteps must be non-negative")
    load(program)

    var traces: [WasmStepTrace] = []
    traces.reserveCapacity(min(maxSteps, program.count))
    while !halted, traces.count < maxSteps {
      traces.append(try step())
    }
    return traces
  }

  public func reset() {
    bytecode = []
    pc = 0
    cycle = 0
    halted = false
    stack.removeAll(keepingCapacity: true)
    locals = Array(repeating: 0, count: locals.count)
  }
}

public func encodeI32Const(_ value: Int32) -> [UInt8] {
  let bits = UInt32(bitPattern: value)
  return [
    WasmOpcode.i32Const,
    UInt8(truncatingIfNeeded: bits),
    UInt8(truncatingIfNeeded: bits >> 8),
    UInt8(truncatingIfNeeded: bits >> 16),
    UInt8(truncatingIfNeeded: bits >> 24),
  ]
}

public func encodeI32Add() -> [UInt8] {
  [WasmOpcode.i32Add]
}

public func encodeI32Sub() -> [UInt8] {
  [WasmOpcode.i32Sub]
}

public func encodeLocalGet(_ index: UInt8) -> [UInt8] {
  [WasmOpcode.localGet, index]
}

public func encodeLocalSet(_ index: UInt8) -> [UInt8] {
  [WasmOpcode.localSet, index]
}

public func encodeEnd() -> [UInt8] {
  [WasmOpcode.end]
}

public func assembleWasm(_ instructions: [[UInt8]]) -> [UInt8] {
  instructions.flatMap { $0 }
}
