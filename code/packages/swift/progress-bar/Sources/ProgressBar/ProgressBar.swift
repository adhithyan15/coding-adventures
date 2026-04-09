import Foundation

public enum EventType: String, Sendable {
    case started
    case finished
    case skipped
}

public struct Event: Sendable {
    public let type: EventType
    public let name: String
    public let status: String
    
    public init(type: EventType, name: String, status: String = "") {
        self.type = type
        self.name = name
        self.status = status
    }
}

public protocol ProgressTracker: Sendable {
    func start()
    func send(_ event: Event)
    func child(total: Int, label: String) -> any ProgressTracker
    func finish()
    func stop()
    
    var completed: Int { get }
    var total: Int { get }
    var label: String { get }
}

public final class NullTracker: ProgressTracker {
    public init() {}
    public func start() {}
    public func send(_ event: Event) {}
    public func child(total: Int, label: String) -> any ProgressTracker { return NullTracker() }
    public func finish() {}
    public func stop() {}
    
    public var completed: Int { 0 }
    public var total: Int { 0 }
    public var label: String { "" }
}

fileprivate enum QueueItem {
    case event(Event)
    case sentinel
}

fileprivate final class EventQueue: @unchecked Sendable {
    private let condition = NSCondition()
    private var items = [QueueItem]()

    func put(_ item: QueueItem) {
        condition.lock()
        items.append(item)
        condition.signal()
        condition.unlock()
    }

    func get() -> QueueItem {
        condition.lock()
        while items.isEmpty {
            condition.wait()
        }
        let item = items.removeFirst()
        condition.unlock()
        return item
    }
}

public final class Tracker: ProgressTracker, @unchecked Sendable {
    private let _total: Int
    private var _completed: Int = 0
    private var _building: [String: Bool] = [:]
    
    private let _events = EventQueue()
    private let _writer: @Sendable (String) -> Void
    private var _startTime: TimeInterval = 0.0
    private let _label: String
    
    private var _thread: Thread?
    private weak var _parent: Tracker?
    
    private let _stateLock = NSLock()
    private let _completionCondition = NSCondition()
    private var _isFinished = false
    
    public init(total: Int, writer: @escaping @Sendable (String) -> Void = { 
        if let data = $0.data(using: .utf8) {
            FileHandle.standardError.write(data)
        }
    }, label: String = "") {
        self._total = total
        self._writer = writer
        self._label = label
    }
    
    public var total: Int { _total }
    public var label: String { _label }
    public var completed: Int {
        _stateLock.lock()
        defer { _stateLock.unlock() }
        return _completed
    }
    
    public func start() {
        _startTime = Date().timeIntervalSince1970
        _thread = Thread { [weak self] in self?._render() }
        _thread?.name = "progress-bar-renderer"
        _thread?.start()
    }
    
    public func send(_ event: Event) {
        _events.put(.event(event))
    }
    
    public func child(total: Int, label: String) -> any ProgressTracker {
        let childTracker = Tracker(total: total, writer: self._writer, label: label)
        childTracker._startTime = self._startTime
        childTracker._parent = self
        // Auto-start child
        childTracker._thread = Thread { [weak childTracker] in childTracker?._render() }
        childTracker._thread?.name = "progress-bar-child-renderer"
        childTracker._thread?.start()
        return childTracker
    }
    
    public func finish() {
        _events.put(.sentinel)
        _completionCondition.lock()
        while !_isFinished {
            _completionCondition.wait()
        }
        _completionCondition.unlock()
        _parent?.send(Event(type: .finished, name: self._label))
    }
    
    public func stop() {
        _events.put(.sentinel)
        _completionCondition.lock()
        while !_isFinished {
            _completionCondition.wait()
        }
        _completionCondition.unlock()
        _writer("\n")
    }
    
    private func _render() {
        while true {
            let item = _events.get()
            switch item {
            case .sentinel:
                _draw()
                _completionCondition.lock()
                _isFinished = true
                _completionCondition.broadcast()
                _completionCondition.unlock()
                return
            case .event(let event):
                _stateLock.lock()
                if event.type == .started {
                    _building[event.name] = true
                } else if event.type == .finished {
                    _building.removeValue(forKey: event.name)
                    _completed += 1
                } else if event.type == .skipped {
                    _completed += 1
                }
                _stateLock.unlock()
                _draw()
            }
        }
    }
    
    private func formatActivity(building: [String: Bool], completed: Int, total: Int) -> String {
        if building.isEmpty {
            if completed >= total {
                return "done"
            }
            return "waiting..."
        }
        let names = building.keys.sorted()
        let maxNames = 3
        if names.count <= maxNames {
            return "Building: " + names.joined(separator: ", ")
        }
        let shown = names.prefix(maxNames).joined(separator: ", ")
        return "Building: \(shown) +\(names.count - maxNames) more"
    }
    
    private func _draw() {
        let elapsed = Date().timeIntervalSince1970 - _startTime
        
        _stateLock.lock()
        let currentCompleted = _completed
        let currentBuilding = _building
        _stateLock.unlock()
        
        let barWidth = 20
        var filled = 0
        if _total > 0 {
            filled = (currentCompleted * barWidth) / _total
        }
        filled = min(filled, barWidth)
        
        let blockFull = "\u{2588}"
        let blockEmpty = "\u{2591}"
        let bar = String(repeating: blockFull, count: filled) + String(repeating: blockEmpty, count: barWidth - filled)
        
        let activity = formatActivity(building: currentBuilding, completed: currentCompleted, total: _total)
        
        var line: String
        if let parent = _parent {
            let parentCompleted = parent.completed + 1
            line = String(format: "\r%@ %d/%d  [%@]  %d/%d  %@  (%.1fs)",
                          parent.label, parentCompleted, parent.total, bar, currentCompleted, _total, activity, elapsed)
        } else if !_label.isEmpty {
            line = String(format: "\r%@ %d/%d  [%@]  %@  (%.1fs)",
                          _label, currentCompleted, _total, bar, activity, elapsed)
        } else {
            line = String(format: "\r[%@]  %d/%d  %@  (%.1fs)",
                          bar, currentCompleted, _total, activity, elapsed)
        }
        
        let padded = line.padding(toLength: max(80, line.count), withPad: " ", startingAt: 0)
        _writer(padded)
    }
}
