import Foundation
import LogicGates

public class SwitchMatrix {
    private let _ports: Set<String>
    private var _connections: [String: String] = [:]

    public init(ports: Set<String>) throws {
        guard !ports.isEmpty else {
            throw FPGAError.invalidArgument("ports must be non-empty")
        }
        for p in ports {
            guard !p.isEmpty else {
                throw FPGAError.invalidArgument("port names must be non-empty strings")
            }
        }
        self._ports = ports
    }

    public func connect(source: String, destination: String) throws {
        guard _ports.contains(source) else {
            throw FPGAError.invalidArgument("unknown source port: \(source)")
        }
        guard _ports.contains(destination) else {
            throw FPGAError.invalidArgument("unknown destination port: \(destination)")
        }
        guard source != destination else {
            throw FPGAError.invalidArgument("cannot connect port \(source) to itself")
        }
        if let existing = _connections[destination] {
            throw FPGAError.invalidArgument("destination \(destination) already connected to \(existing)")
        }
        _connections[destination] = source
    }

    public func disconnect(destination: String) throws {
        guard _ports.contains(destination) else {
            throw FPGAError.invalidArgument("unknown port: \(destination)")
        }
        guard _connections[destination] != nil else {
            throw FPGAError.invalidArgument("port \(destination) is not connected")
        }
        _connections.removeValue(forKey: destination)
    }

    public func clear() {
        _connections.removeAll()
    }

    public func route(inputs: [String: Bit]) -> [String: Bit] {
        var outputs: [String: Bit] = [:]
        for (dest, src) in _connections {
            if let val = inputs[src] {
                outputs[dest] = val
            }
        }
        return outputs
    }

    public var ports: Set<String> { return _ports }
    public var connections: [String: String] { return _connections }
    public var connectionCount: Int { return _connections.count }
}
