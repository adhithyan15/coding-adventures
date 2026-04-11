import Foundation

public class ConfigurableBRAM {
    private let _totalBits: Int
    private var _width: Int
    private var _depth: Int
    private var _ram: DualPortRAM

    public init(totalBits: Int = 18432, width: Int = 8) throws {
        guard totalBits >= 1 else { throw BlockRAMError.invalidArgument("totalBits must be >= 1") }
        guard width >= 1 else { throw BlockRAMError.invalidArgument("width must be >= 1") }
        guard totalBits % width == 0 else { throw BlockRAMError.invalidArgument("width \(width) does not evenly divide totalBits \(totalBits)") }

        self._totalBits = totalBits
        self._width = width
        self._depth = totalBits / width
        self._ram = try DualPortRAM(depth: self._depth, width: self._width)
    }

    public func reconfigure(width: Int) throws {
        guard width >= 1 else { throw BlockRAMError.invalidArgument("width must be >= 1") }
        guard _totalBits % width == 0 else { throw BlockRAMError.invalidArgument("width \(width) does not evenly divide totalBits \(_totalBits)") }

        self._width = width
        self._depth = self._totalBits / width
        self._ram = try DualPortRAM(depth: self._depth, width: self._width)
    }

    public func tickA(clock: Bit, address: Int, dataIn: [Bit], writeEnable: Bit) throws -> [Bit] {
        let zeros = Array(repeating: 0, count: _width)
        let (outA, _) = try _ram.tick(
            clock: clock,
            addressA: address, dataInA: dataIn, writeEnableA: writeEnable,
            addressB: 0, dataInB: zeros, writeEnableB: 0
        )
        return outA
    }

    public func tickB(clock: Bit, address: Int, dataIn: [Bit], writeEnable: Bit) throws -> [Bit] {
        let zeros = Array(repeating: 0, count: _width)
        let (_, outB) = try _ram.tick(
            clock: clock,
            addressA: 0, dataInA: zeros, writeEnableA: 0,
            addressB: address, dataInB: dataIn, writeEnableB: writeEnable
        )
        return outB
    }

    public var depth: Int { return _depth }
    public var width: Int { return _width }
    public var totalBits: Int { return _totalBits }
}
