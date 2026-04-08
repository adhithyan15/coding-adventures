import Foundation
import LogicGates
import BlockRAM

/// K-input Look-Up Table -- the atom of programmable logic.
public class LUT {
    private let _k: Int
    private let _size: Int
    private let _sram: [SRAMCell]

    public init(k: Int = 4, truthTable: [Bit]? = nil) throws {
        guard k >= 2 && k <= 6 else { throw FPGAError.invalidArgument("k must be between 2 and 6, got \(k)") }
        self._k = k
        self._size = 1 << k
        self._sram = (0..<_size).map { _ in SRAMCell() }
        
        if let tt = truthTable {
            try configure(truthTable: tt)
        }
    }

    public func configure(truthTable: [Bit]) throws {
        guard truthTable.count == _size else {
            throw FPGAError.invalidArgument("truthTable length \(truthTable.count) does not match 2^k = \(_size)")
        }
        for i in 0..<truthTable.count {
            _sram[i].write(wordLine: 1, bitLine: truthTable[i])
        }
    }

    public func evaluate(inputs: [Bit]) throws -> Bit {
        guard inputs.count == _k else {
            throw FPGAError.invalidArgument("inputs length \(inputs.count) does not match k = \(_k)")
        }
        
        var data: [Bit] = []
        data.reserveCapacity(_size)
        for cell in _sram {
            data.append(cell.read(wordLine: 1) ?? 0)
        }
        
        return try muxN(inputs: data, sel: inputs)
    }

    public var k: Int { return _k }
    public var truthTable: [Bit] { return _sram.map { $0.read(wordLine: 1) ?? 0 } }
}
