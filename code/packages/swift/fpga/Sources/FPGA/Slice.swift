import Foundation
import LogicGates
import BlockRAM

public struct SliceOutput: Equatable {
    public let outputA: Bit
    public let outputB: Bit
    public let carryOut: Bit
}

public class Slice {
    private let _lutA: LUT
    private let _lutB: LUT
    private let _k: Int

    private var _ffAState: FlipFlopState
    private var _ffBState: FlipFlopState

    private var _ffAEnabled: Bool = false
    private var _ffBEnabled: Bool = false
    private var _carryEnabled: Bool = false

    public init(lutInputs: Int = 4) throws {
        self._lutA = try LUT(k: lutInputs)
        self._lutB = try LUT(k: lutInputs)
        self._k = lutInputs
        self._ffAState = FlipFlopState(q: 0, qBar: 1, masterQ: 0)
        self._ffBState = FlipFlopState(q: 0, qBar: 1, masterQ: 0)
    }

    public func configure(
        lutATable: [Bit],
        lutBTable: [Bit],
        ffAEnabled: Bool = false,
        ffBEnabled: Bool = false,
        carryEnabled: Bool = false
    ) throws {
        try _lutA.configure(truthTable: lutATable)
        try _lutB.configure(truthTable: lutBTable)
        self._ffAEnabled = ffAEnabled
        self._ffBEnabled = ffBEnabled
        self._carryEnabled = carryEnabled

        // Reset flip-flop state on reconfiguration
        self._ffAState = FlipFlopState(q: 0, qBar: 1, masterQ: 0)
        self._ffBState = FlipFlopState(q: 0, qBar: 1, masterQ: 0)
    }

    public func evaluate(
        inputsA: [Bit],
        inputsB: [Bit],
        clock: Bit,
        carryIn: Bit = 0
    ) throws -> SliceOutput {
        // Evaluate LUTs
        let lutAOut = try _lutA.evaluate(inputs: inputsA)
        let lutBOut = try _lutB.evaluate(inputs: inputsB)

        // Flip-flop A
        let outputA: Bit
        if _ffAEnabled {
            let newState = try dFlipFlop(
                data: lutAOut, clock: clock,
                q: _ffAState.q, qBar: _ffAState.qBar,
                masterQ: _ffAState.masterQ, masterQBar: 1 - _ffAState.masterQ
            )
            self._ffAState = newState
            outputA = try mux2(d0: lutAOut, d1: newState.q, sel: 1)
        } else {
            outputA = lutAOut
        }

        // Flip-flop B
        let outputB: Bit
        if _ffBEnabled {
            let newState = try dFlipFlop(
                data: lutBOut, clock: clock,
                q: _ffBState.q, qBar: _ffBState.qBar,
                masterQ: _ffBState.masterQ, masterQBar: 1 - _ffBState.masterQ
            )
            self._ffBState = newState
            outputB = try mux2(d0: lutBOut, d1: newState.q, sel: 1)
        } else {
            outputB = lutBOut
        }

        // Carry chain
        let carryOut: Bit
        if _carryEnabled {
            let term1 = try andGate(lutAOut, lutBOut)
            let term2 = try andGate(carryIn, try xorGate(lutAOut, lutBOut))
            carryOut = try orGate(term1, term2)
        } else {
            carryOut = 0
        }

        return SliceOutput(outputA: outputA, outputB: outputB, carryOut: carryOut)
    }

    public var lutA: LUT { return _lutA }
    public var lutB: LUT { return _lutB }
    public var k: Int { return _k }
}
