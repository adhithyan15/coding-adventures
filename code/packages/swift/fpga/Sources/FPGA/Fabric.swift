import Foundation
import LogicGates

public class FPGA {
    private let _bitstream: Bitstream
    private var _clbs: [String: CLB] = [:]
    private var _switches: [String: SwitchMatrix] = [:]
    private var _ios: [String: IOBlock] = [:]

    public init(bitstream: Bitstream) throws {
        self._bitstream = bitstream
        try _configure(bs: bitstream)
    }

    private func _configure(bs: Bitstream) throws {
        for (name, clbCfg) in bs.clbs {
            let clb = try CLB(lutInputs: bs.lutK)

            try clb.slice0.configure(
                lutATable: clbCfg.slice0.lutA,
                lutBTable: clbCfg.slice0.lutB,
                ffAEnabled: clbCfg.slice0.ffAEnabled,
                ffBEnabled: clbCfg.slice0.ffBEnabled,
                carryEnabled: clbCfg.slice0.carryEnabled
            )
            
            try clb.slice1.configure(
                lutATable: clbCfg.slice1.lutA,
                lutBTable: clbCfg.slice1.lutB,
                ffAEnabled: clbCfg.slice1.ffAEnabled,
                ffBEnabled: clbCfg.slice1.ffBEnabled,
                carryEnabled: clbCfg.slice1.carryEnabled
            )

            _clbs[name] = clb
        }

        for (swName, routes) in bs.routing {
            var ports = Set<String>()
            for route in routes {
                ports.insert(route.source)
                ports.insert(route.destination)
            }

            if !ports.isEmpty {
                let sm = try SwitchMatrix(ports: ports)
                for route in routes {
                    try sm.connect(source: route.source, destination: route.destination)
                }
                _switches[swName] = sm
            }
        }

        for (pinName, ioCfg) in bs.io {
            let mode = IOMode(rawValue: ioCfg.mode) ?? .input
            _ios[pinName] = try IOBlock(name: pinName, mode: mode)
        }
    }

    public func evaluateCLB(
        clbName: String,
        slice0InputsA: [Bit],
        slice0InputsB: [Bit],
        slice1InputsA: [Bit],
        slice1InputsB: [Bit],
        clock: Bit,
        carryIn: Bit = 0
    ) throws -> CLBOutput {
        guard let clb = _clbs[clbName] else {
            throw FPGAError.notFound("CLB \(clbName) not found")
        }

        return try clb.evaluate(
            slice0InputsA: slice0InputsA,
            slice0InputsB: slice0InputsB,
            slice1InputsA: slice1InputsA,
            slice1InputsB: slice1InputsB,
            clock: clock,
            carryIn: carryIn
        )
    }

    public func route(switchName: String, signals: [String: Bit]) throws -> [String: Bit] {
        guard let sm = _switches[switchName] else {
            throw FPGAError.notFound("Switch matrix \(switchName) not found")
        }
        return sm.route(inputs: signals)
    }

    public func setInput(pinName: String, value: Bit) throws {
        guard let io = _ios[pinName] else {
            throw FPGAError.notFound("I/O pin \(pinName) not found")
        }
        try io.drivePad(value: value)
    }

    public func readOutput(pinName: String) throws -> Bit? {
        guard let io = _ios[pinName] else {
            throw FPGAError.notFound("I/O pin \(pinName) not found")
        }
        return try io.readPad()
    }

    public func driveOutput(pinName: String, value: Bit) throws {
        guard let io = _ios[pinName] else {
            throw FPGAError.notFound("I/O pin \(pinName) not found")
        }
        try io.driveInternal(value: value)
    }

    public var clbs: [String: CLB] { return _clbs }
    public var switches: [String: SwitchMatrix] { return _switches }
    public var ios: [String: IOBlock] { return _ios }
    public var bitstream: Bitstream { return _bitstream }
}
