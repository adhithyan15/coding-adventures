import Foundation
import LogicGates

public struct SliceConfig {
    public let lutA: [Bit]
    public let lutB: [Bit]
    public let ffAEnabled: Bool
    public let ffBEnabled: Bool
    public let carryEnabled: Bool

    public init(lutA: [Bit], lutB: [Bit], ffAEnabled: Bool, ffBEnabled: Bool, carryEnabled: Bool) {
        self.lutA = lutA
        self.lutB = lutB
        self.ffAEnabled = ffAEnabled
        self.ffBEnabled = ffBEnabled
        self.carryEnabled = carryEnabled
    }
}

public struct CLBConfig {
    public let slice0: SliceConfig
    public let slice1: SliceConfig

    public init(slice0: SliceConfig, slice1: SliceConfig) {
        self.slice0 = slice0
        self.slice1 = slice1
    }
}

public struct RouteConfig {
    public let source: String
    public let destination: String

    public init(source: String, destination: String) {
        self.source = source
        self.destination = destination
    }
}

public struct IOConfig {
    public let mode: String

    public init(mode: String) {
        self.mode = mode
    }
}

public class Bitstream {
    public let clbs: [String: CLBConfig]
    public let routing: [String: [RouteConfig]]
    public let io: [String: IOConfig]
    public let lutK: Int

    public init(clbs: [String: CLBConfig] = [:], routing: [String: [RouteConfig]] = [:], io: [String: IOConfig] = [:], lutK: Int = 4) {
        self.clbs = clbs
        self.routing = routing
        self.io = io
        self.lutK = lutK
    }

    public static func empty(lutK: Int = 4) -> Bitstream {
        return Bitstream(clbs: [:], routing: [:], io: [:], lutK: lutK)
    }
}
