import Foundation
import LogicGates

public struct CLBOutput: Equatable {
    public let slice0: SliceOutput
    public let slice1: SliceOutput
}

public class CLB {
    private let _slice0: Slice
    private let _slice1: Slice
    private let _k: Int

    public init(lutInputs: Int = 4) throws {
        self._slice0 = try Slice(lutInputs: lutInputs)
        self._slice1 = try Slice(lutInputs: lutInputs)
        self._k = lutInputs
    }

    public var slice0: Slice { return _slice0 }
    public var slice1: Slice { return _slice1 }
    public var k: Int { return _k }

    public func evaluate(
        slice0InputsA: [Bit],
        slice0InputsB: [Bit],
        slice1InputsA: [Bit],
        slice1InputsB: [Bit],
        clock: Bit,
        carryIn: Bit = 0
    ) throws -> CLBOutput {
        let out0 = try _slice0.evaluate(
            inputsA: slice0InputsA,
            inputsB: slice0InputsB,
            clock: clock,
            carryIn: carryIn
        )

        let out1 = try _slice1.evaluate(
            inputsA: slice1InputsA,
            inputsB: slice1InputsB,
            clock: clock,
            carryIn: out0.carryOut
        )

        return CLBOutput(slice0: out0, slice1: out1)
    }
}
