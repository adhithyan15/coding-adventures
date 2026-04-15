import Foundation

/// Core errors for Clock package
public enum ClockError: Error, Equatable {
    case invalidDivisor(Int)
    case invalidPhases(Int)
    case listenerNotFound
}

/// Record of a clock transition.
public struct ClockEdge: Equatable {
    public let cycle: Int
    public let value: Int
    public let isRising: Bool
    public let isFalling: Bool
}

/// A function that receives clock edges.
public typealias ClockListener = (ClockEdge) -> Void

/// Unique identifier for unregistering listeners.
public typealias ClockListenerToken = UUID

/// System clock generator.
public class Clock {
    public let frequencyHz: Int
    public private(set) var cycle: Int = 0
    public private(set) var value: Int = 0
    private var _tickCount: Int = 0
    private var _listeners: [ClockListenerToken: ClockListener] = [:]

    public init(frequencyHz: Int = 1_000_000) {
        self.frequencyHz = frequencyHz
    }

    @discardableResult
    public func tick() -> ClockEdge {
        let oldValue = self.value
        self.value = 1 - self.value
        self._tickCount += 1

        let isRising = oldValue == 0 && self.value == 1
        let isFalling = oldValue == 1 && self.value == 0

        if isRising {
            self.cycle += 1
        }

        let edge = ClockEdge(
            cycle: self.cycle,
            value: self.value,
            isRising: isRising,
            isFalling: isFalling
        )

        for listener in _listeners.values {
            listener(edge)
        }

        return edge
    }

    public func fullCycle() -> (ClockEdge, ClockEdge) {
        let rising = self.tick()
        let falling = self.tick()
        return (rising, falling)
    }

    public func run(cycles: Int) -> [ClockEdge] {
        var edges: [ClockEdge] = []
        edges.reserveCapacity(cycles * 2)
        for _ in 0..<cycles {
            let (r, f) = self.fullCycle()
            edges.append(r)
            edges.append(f)
        }
        return edges
    }

    @discardableResult
    public func registerListener(_ callback: @escaping ClockListener) -> ClockListenerToken {
        let token = UUID()
        _listeners[token] = callback
        return token
    }

    public func unregisterListener(_ token: ClockListenerToken) throws {
        guard _listeners.removeValue(forKey: token) != nil else {
            throw ClockError.listenerNotFound
        }
    }

    public func reset() {
        self.cycle = 0
        self.value = 0
        self._tickCount = 0
    }

    public var periodNs: Double {
        return 1e9 / Double(frequencyHz)
    }

    public var totalTicks: Int {
        return self._tickCount
    }
}

/// Divides a clock frequency by an integer factor.
public class ClockDivider {
    public let source: Clock
    public let divisor: Int
    public let output: Clock
    private var _counter: Int = 0
    private var listenerToken: ClockListenerToken?

    public init(source: Clock, divisor: Int) throws {
        guard divisor >= 2 else {
            throw ClockError.invalidDivisor(divisor)
        }
        self.source = source
        self.divisor = divisor
        self.output = Clock(frequencyHz: source.frequencyHz / divisor)
        
        self.listenerToken = self.source.registerListener { [weak self] edge in
            self?._onEdge(edge)
        }
    }

    private func _onEdge(_ edge: ClockEdge) {
        if edge.isRising {
            self._counter += 1
            if self._counter >= self.divisor {
                self._counter = 0
                self.output.tick() // rising
                self.output.tick() // falling
            }
        }
    }
}

/// Generates multiple clock phases from a single source.
public class MultiPhaseClock {
    public let source: Clock
    public let phases: Int
    public private(set) var activePhase: Int = 0
    private var _phaseValues: [Int]
    private var listenerToken: ClockListenerToken?

    public init(source: Clock, phases: Int = 4) throws {
        guard phases >= 2 else {
            throw ClockError.invalidPhases(phases)
        }
        self.source = source
        self.phases = phases
        self._phaseValues = Array(repeating: 0, count: phases)
        
        self.listenerToken = self.source.registerListener { [weak self] edge in
            self?._onEdge(edge)
        }
    }

    private func _onEdge(_ edge: ClockEdge) {
        if edge.isRising {
            self._phaseValues = Array(repeating: 0, count: phases)
            self._phaseValues[self.activePhase] = 1
            self.activePhase = (self.activePhase + 1) % self.phases
        }
    }

    public func getPhase(_ index: Int) -> Int {
        guard index >= 0 && index < phases else { return 0 }
        return self._phaseValues[index]
    }
}
