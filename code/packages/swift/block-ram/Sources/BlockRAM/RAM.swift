import Foundation

public enum ReadMode {
    case readFirst
    case writeFirst
    case noChange
}

public class SinglePortRAM {
    private let _depth: Int
    private let _width: Int
    private let _readMode: ReadMode
    private let _array: SRAMArray
    private var _prevClock: Bit = 0
    private var _lastRead: [Bit]

    public init(depth: Int, width: Int, readMode: ReadMode = .readFirst) throws {
        guard depth >= 1 else { throw BlockRAMError.invalidArgument("depth must be >= 1") }
        guard width >= 1 else { throw BlockRAMError.invalidArgument("width must be >= 1") }

        self._depth = depth
        self._width = width
        self._readMode = readMode
        self._array = try SRAMArray(rows: depth, cols: width)
        self._lastRead = Array(repeating: 0, count: width)
    }

    public func tick(clock: Bit, address: Int, dataIn: [Bit], writeEnable: Bit) throws -> [Bit] {
        try _validateAddress(address, name: "address")
        try _validateData(dataIn, name: "dataIn")

        let risingEdge = _prevClock == 0 && clock == 1
        _prevClock = clock

        if !risingEdge {
            return _lastRead
        }

        if writeEnable == 0 {
            _lastRead = try _array.read(row: address)
            return _lastRead
        }

        switch _readMode {
        case .readFirst:
            _lastRead = try _array.read(row: address)
            try _array.write(row: address, data: dataIn)
            return _lastRead
        case .writeFirst:
            try _array.write(row: address, data: dataIn)
            _lastRead = dataIn
            return _lastRead
        case .noChange:
            try _array.write(row: address, data: dataIn)
            return _lastRead
        }
    }

    public var depth: Int { return _depth }
    public var width: Int { return _width }

    public func dump() throws -> [[Bit]] {
        return try (0..<_depth).map { try _array.read(row: $0) }
    }

    private func _validateAddress(_ address: Int, name: String) throws {
        guard address >= 0 && address < _depth else {
            throw BlockRAMError.outOfRange("\(name) \(address) out of range [0, \(_depth - 1)]")
        }
    }

    private func _validateData(_ dataIn: [Bit], name: String) throws {
        guard dataIn.count == _width else {
            throw BlockRAMError.invalidArgument("\(name) length \(dataIn.count) does not match width \(_width)")
        }
    }
}

public class DualPortRAM {
    private let _depth: Int
    private let _width: Int
    private let _readModeA: ReadMode
    private let _readModeB: ReadMode
    private let _array: SRAMArray
    private var _prevClock: Bit = 0
    private var _lastReadA: [Bit]
    private var _lastReadB: [Bit]

    public init(depth: Int, width: Int, readModeA: ReadMode = .readFirst, readModeB: ReadMode = .readFirst) throws {
        guard depth >= 1 else { throw BlockRAMError.invalidArgument("depth must be >= 1") }
        guard width >= 1 else { throw BlockRAMError.invalidArgument("width must be >= 1") }

        self._depth = depth
        self._width = width
        self._readModeA = readModeA
        self._readModeB = readModeB
        self._array = try SRAMArray(rows: depth, cols: width)
        self._lastReadA = Array(repeating: 0, count: width)
        self._lastReadB = Array(repeating: 0, count: width)
    }

    public func tick(clock: Bit, addressA: Int, dataInA: [Bit], writeEnableA: Bit, addressB: Int, dataInB: [Bit], writeEnableB: Bit) throws -> ([Bit], [Bit]) {
        try _validateAddress(addressA, name: "addressA")
        try _validateAddress(addressB, name: "addressB")
        try _validateData(dataInA, name: "dataInA")
        try _validateData(dataInB, name: "dataInB")

        let risingEdge = _prevClock == 0 && clock == 1
        _prevClock = clock

        if !risingEdge {
            return (_lastReadA, _lastReadB)
        }

        if writeEnableA == 1 && writeEnableB == 1 && addressA == addressB {
            throw BlockRAMError.writeCollision(address: addressA)
        }

        let outA = try _processPort(address: addressA, dataIn: dataInA, writeEnable: writeEnableA, readMode: _readModeA, lastRead: _lastReadA)
        _lastReadA = outA

        let outB = try _processPort(address: addressB, dataIn: dataInB, writeEnable: writeEnableB, readMode: _readModeB, lastRead: _lastReadB)
        _lastReadB = outB

        return (outA, outB)
    }

    public var depth: Int { return _depth }
    public var width: Int { return _width }

    private func _processPort(address: Int, dataIn: [Bit], writeEnable: Bit, readMode: ReadMode, lastRead: [Bit]) throws -> [Bit] {
        if writeEnable == 0 {
            return try _array.read(row: address)
        }

        switch readMode {
        case .readFirst:
            let result = try _array.read(row: address)
            try _array.write(row: address, data: dataIn)
            return result
        case .writeFirst:
            try _array.write(row: address, data: dataIn)
            return dataIn
        case .noChange:
            try _array.write(row: address, data: dataIn)
            return lastRead
        }
    }

    private func _validateAddress(_ address: Int, name: String) throws {
        guard address >= 0 && address < _depth else {
            throw BlockRAMError.outOfRange("\(name) \(address) out of range [0, \(_depth - 1)]")
        }
    }

    private func _validateData(_ dataIn: [Bit], name: String) throws {
        guard dataIn.count == _width else {
            throw BlockRAMError.invalidArgument("\(name) length \(dataIn.count) does not match width \(_width)")
        }
    }
}
