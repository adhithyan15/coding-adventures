import Foundation
import LogicGates

public enum IOMode: String {
    case input = "input"
    case output = "output"
    case tristate = "tristate"
}

public class IOBlock {
    private let _name: String
    private var _mode: IOMode
    private var _padValue: Bit = 0
    private var _internalValue: Bit = 0

    public init(name: String, mode: IOMode = .input) throws {
        guard !name.isEmpty else {
            throw FPGAError.invalidArgument("name must be a non-empty string")
        }
        self._name = name
        self._mode = mode
    }

    public func configure(mode: IOMode) {
        self._mode = mode
    }

    public func drivePad(value: Bit) throws {
        guard value == 0 || value == 1 else {
            throw FPGAError.invalidArgument("value must be 0 or 1, got \(value)")
        }
        self._padValue = value
    }

    public func driveInternal(value: Bit) throws {
        guard value == 0 || value == 1 else {
            throw FPGAError.invalidArgument("value must be 0 or 1, got \(value)")
        }
        self._internalValue = value
    }

    public func readInternal() -> Bit {
        if _mode == .input {
            return _padValue
        }
        return _internalValue
    }

    public func readPad() throws -> Bit? {
        if _mode == .input {
            return _padValue
        }
        if _mode == .tristate {
            return try triState(data: _internalValue, enable: 0)
        }
        return try triState(data: _internalValue, enable: 1)
    }

    public var name: String { return _name }
    public var mode: IOMode { return _mode }
}
