// ============================================================================
// ShrinkBudget.swift -- the hierarchical shrink budget (VER05e section 4.1)
// ============================================================================
//
// THE PROBLEM
// -----------
// Shrinking calls the user's property many times on candidates the engine
// builds. Without a ceiling a hostile (or merely unlucky) property can make one
// `check` cost gigabytes of copying or hours of evaluations (CWE-400). Worse,
// properties and shrinkers can *nest*: a property may itself call `Runner.check`,
// a custom shrinker may call `Gen.shrink`, a filter predicate may shrink. Each
// level must be bounded, AND the whole stack together must be bounded by the
// outermost caller.
//
// THE DESIGN: A TREE OF BUDGETS
// -----------------------------
// A *scope* is a node of a tree. Every public entry point that runs user code
// opens a CHILD of whatever scope is ambient on this thread, with a fresh local
// cap. A charge walks from the node up to the root and checks every node on the
// way; it is ALL OR NOTHING: if any node on the path lacks room nothing is
// charged. So isolation only ever means "a fresh *local* cap", never "unbounded":
// whatever the nesting, the outermost root's cap bounds the total.
//
//         root   (cap R)          take(cost): every node on the path must have
//          |                      room (cost <= cap - used); only then are ALL of
//        child  (cap C1)          them incremented. A local-only take (checking
//          |                      just the starting node) lets a nest of scopes
//        child  (cap C2)          each spend up to its own cap and multiplies
//                                 the work: the sabotage test pins this.
//
// TWO COUNTERS
// ------------
//   * attempts: one per property evaluation (cap 16 * maxSteps + 64);
//   * work:     elements copied, at least 1 per charge (cap 64M).
// The room test is written `cost > max - used`, which cannot wrap even for
// `Int.max` (used <= max always holds, so the subtraction never underflows).
//
// ONE LOCK
// --------
// One `NSLock`, created by the outermost root and shared by every node of its
// tree, guards every counter. User code never runs while it is held.
//
// THE AMBIENT SCOPE (THREADING STORY)
// -----------------------------------
// The "current" scope is stored per thread in `Thread.threadDictionary`. Code
// that runs on the same thread as the engine (properties, shrinkers, filter
// predicates) sees the right scope with no signature changes. A *new* thread
// does not inherit it (unlike Java's InheritableThreadLocal): work done there
// opens a fresh root. That is the documented "pool-thread escape"; keep user
// code on the calling thread. A node whose chain has all closed is skipped, so
// a thread that outlives its scope never charges a dead root.
// ============================================================================

import Foundation

final class ShrinkBudget: @unchecked Sendable {
    /// The most nested *owning* scopes; deeper scopes join, keeping the walk short.
    static let maxDepth = 64

    /// Test hook: the work cap given to a new root (restored by `Hooks.reset`).
    static var defaultMaxWork: Int {
        get { statLock.lock(); defer { statLock.unlock() }; return _defaultMaxWork }
        set { statLock.lock(); defer { statLock.unlock() }; _defaultMaxWork = newValue }
    }
    /// Work / attempts spent by the most recent owning scope that closed (tests).
    static var lastWork: Int {
        get { statLock.lock(); defer { statLock.unlock() }; return _lastWork }
        set { statLock.lock(); defer { statLock.unlock() }; _lastWork = newValue }
    }
    static var lastAttempts: Int {
        get { statLock.lock(); defer { statLock.unlock() }; return _lastAttempts }
        set { statLock.lock(); defer { statLock.unlock() }; _lastAttempts = newValue }
    }
    // The statics above are read and written from any thread; one lock guards them.
    private static let statLock = NSLock()
    private static var _defaultMaxWork = Limits.maxShrinkWork
    private static var _lastWork = 0
    private static var _lastAttempts = 0

    private static let ambientKey = "coding-adventures.test-bootstrap.shrink-budget"
    private static let nestingKey = "coding-adventures.test-bootstrap.nesting"

    /// The most nested public entries (checks and shrinks) one thread may have
    /// open at once; deeper entries refuse to run (F4: bounded recursion).
    static let maxNesting = 64

    /// The scope bound to the current structured-concurrency task, so a
    /// `Task {}`, `async let` or task group created inside a scope inherits it
    /// (F1b). The per-thread dictionary is consulted first.
    @TaskLocal static var taskAmbient: ShrinkBudget?

    /// Runs `body` with `b` bound as the task-local ambient scope.
    static func bind<R>(_ b: ShrinkBudget, _ body: () throws -> R) rethrows -> R {
        try $taskAmbient.withValue(b.ownerNode, operation: body)
    }

    /// Installs `node` as this thread's ambient scope; returns the previous raw value.
    static func install(_ node: ShrinkBudget?) -> ShrinkBudget? {
        let previous = ambient
        ambient = node
        return previous
    }

    /// Opens one public entry on this thread; returns the new depth.
    static func enterNesting() -> Int {
        let d = (Thread.current.threadDictionary[nestingKey] as? Int ?? 0) + 1
        Thread.current.threadDictionary[nestingKey] = d
        return d
    }

    static func leaveNesting() {
        let d = (Thread.current.threadDictionary[nestingKey] as? Int ?? 1) - 1
        if d <= 0 { Thread.current.threadDictionary.removeObject(forKey: nestingKey) } else { Thread.current.threadDictionary[nestingKey] = d }
    }

    /// The open scope visible to this code: the thread's, else the task's.
    private static var effectiveAmbient: ShrinkBudget? { usable(ambient) ?? usable(taskAmbient) }

    /// The per-thread ambient node (may be closed; see `usable`).
    private static var ambient: ShrinkBudget? {
        get { Thread.current.threadDictionary[ambientKey] as? ShrinkBudget }
        set {
            if let node = newValue {
                Thread.current.threadDictionary[ambientKey] = node
            } else {
                Thread.current.threadDictionary.removeObject(forKey: ambientKey)
            }
        }
    }

    private let lock: NSLock              // the outermost root's lock, shared by the tree
    private let rootRef: ShrinkBudget?    // outermost node (nil means self)
    private let parent: ShrinkBudget?     // nearest open ancestor at creation
    private let target: ShrinkBudget?     // for a joined handle: the owner it aliases
    private let previous: ShrinkBudget?   // ambient value to restore when an owner closes
    let depth: Int
    private let maxAttempts: Int
    private let maxWork: Int
    private var attempts = 0              // guarded by lock
    private var work = 0                  // guarded by lock
    private var closedFlag = false        // guarded by lock
    private var refusedFlag = false       // on a ROOT: any charge in the tree was refused

    private var root: ShrinkBudget { rootRef ?? self }

    /// An owning node.
    private init(parent: ShrinkBudget?, maxAttempts: Int, maxWork: Int, previous: ShrinkBudget?) {
        self.parent = parent
        self.rootRef = parent?.root
        self.lock = parent?.lock ?? NSLock()
        self.target = nil
        self.previous = previous
        self.depth = parent.map { $0.depth + 1 } ?? 0
        self.maxAttempts = maxAttempts
        self.maxWork = maxWork
    }

    /// A joined handle: charges and queries go to the owner it aliases.
    private init(joining owner: ShrinkBudget) {
        self.parent = nil
        self.rootRef = owner.root
        self.lock = owner.lock
        self.target = owner
        self.previous = nil
        self.depth = owner.depth
        self.maxAttempts = 0
        self.maxWork = 0
    }

    private var owner: ShrinkBudget { target ?? self }
    /// The node that owns the counters (a joined handle's owner, else self).
    var ownerNode: ShrinkBudget { owner }

    /// True when this scope has an owning ancestor (it is not the outermost root).
    var isNested: Bool { owner.parent != nil }
    /// True for a handle that joined an existing scope instead of owning one.
    var isJoined: Bool { target != nil }
    /// True once any charge anywhere in this scope's tree has been refused.
    var wasRefused: Bool {
        let r = root
        r.lock.lock(); defer { r.lock.unlock() }
        return r.refusedFlag
    }

    private var isClosed: Bool {
        lock.lock(); defer { lock.unlock() }
        return closedFlag
    }

    /// The nearest open node from `a` upward whose root is open, or nil.
    private static func usable(_ start: ShrinkBudget?) -> ShrinkBudget? {
        var a = start
        while let node = a, node.isClosed { a = node.parent }
        if let node = a, !node.root.isClosed { return node }
        return nil
    }

    /// The ambient open budget of this thread, or nil outside any shrink.
    static func current() -> ShrinkBudget? { effectiveAmbient }

    private static func newRoot(maxAttempts: Int, maxWork: Int) -> ShrinkBudget {
        let fresh = ShrinkBudget(parent: nil, maxAttempts: maxAttempts, maxWork: maxWork, previous: nil)
        ambient = fresh
        return fresh
    }

    /// Joins the ambient scope when there is one, else opens a fresh root.
    static func enter(maxSteps: Int) -> ShrinkBudget {
        if let open = effectiveAmbient { return ShrinkBudget(joining: open) }
        return newRoot(maxAttempts: Limits.attemptBudget(maxSteps), maxWork: defaultMaxWork)
    }

    /// The scope of a public entry point: a CHILD of the ambient scope with a
    /// fresh local cap, or a fresh root when none is open. Past `maxDepth` it joins.
    static func enterChild(maxSteps: Int, maxWork: Int = Limits.maxShrinkWork) -> ShrinkBudget {
        let amb = ambient
        guard let open = effectiveAmbient else {
            return newRoot(maxAttempts: Limits.attemptBudget(maxSteps),
                           maxWork: maxWork == Limits.maxShrinkWork ? defaultMaxWork : maxWork)
        }
        if open.depth + 1 >= maxDepth { return ShrinkBudget(joining: open) }
        let child = ShrinkBudget(parent: open, maxAttempts: Limits.attemptBudget(maxSteps), maxWork: maxWork, previous: amb)
        ambient = child
        return child
    }

    /// The scope of a whole `Runner.check` (case loop included). Its attempt cap
    /// is `numTests` plus the shrink attempt cap.
    static func enterCheck(numTests: Int, maxSteps: Int) -> ShrinkBudget {
        let attempts = numTests + Limits.attemptBudget(maxSteps)
        let amb = ambient
        guard let open = effectiveAmbient else {
            return newRoot(maxAttempts: attempts, maxWork: defaultMaxWork)
        }
        if open.depth + 1 >= maxDepth { return ShrinkBudget(joining: open) }
        let child = ShrinkBudget(parent: open, maxAttempts: attempts, maxWork: Limits.maxShrinkWork, previous: amb)
        ambient = child
        return child
    }

    // MARK: counters ----------------------------------------------------------

    var attemptsSpent: Int {
        let o = owner
        o.lock.lock(); defer { o.lock.unlock() }
        return o.attempts
    }

    var workSpent: Int {
        let o = owner
        o.lock.lock(); defer { o.lock.unlock() }
        return o.work
    }

    /// Charges one attempt over a candidate of `elements` elements (at least 1)
    /// to this node and every ancestor. Returns false, charging nothing, when any
    /// of them cannot afford it: callers must check this BEFORE building the
    /// candidate.
    func charge(_ elements: Int) -> Bool { take(elements, attemptCost: 1) }

    /// Charges a copy that is not itself a property evaluation: work only.
    func chargeCopy(_ elements: Int) -> Bool { take(elements, attemptCost: 0) }

    /// The all-or-nothing charge. Phase one only READS every node on the path;
    /// phase two increments them all. A refusal leaves every counter unchanged.
    private func take(_ elements: Int, attemptCost: Int) -> Bool {
        let start = owner
        let cost = max(1, elements)
        start.lock.lock(); defer { start.lock.unlock() }
        var n: ShrinkBudget? = start
        while let node = n {
            // Overflow-safe: work <= maxWork always, so the subtraction cannot wrap.
            if node.attempts >= node.maxAttempts || cost > node.maxWork - node.work {
                start.root.refusedFlag = true
                return false
            }
            n = node.parent
        }
        n = start
        while let node = n {
            node.attempts += attemptCost
            node.work += cost
            n = node.parent
        }
        return true
    }

    /// True while at least one more attempt of one element fits everywhere on the path.
    func hasRoom() -> Bool {
        let start = owner
        start.lock.lock(); defer { start.lock.unlock() }
        var n: ShrinkBudget? = start
        while let node = n {
            if node.attempts >= node.maxAttempts || node.work >= node.maxWork { return false }
            n = node.parent
        }
        return true
    }

    /// Wraps a property so each evaluation is charged one attempt first.
    func charged<V>(_ property: @escaping (V) -> Bool) -> (V) -> Bool {
        { v in self.charge(1) && property(v) }
    }

    /// Ends an owning scope. Restores the previous ambient value FIRST, so
    /// nothing below can leave a stale one. A joined handle does nothing.
    func close() {
        if target != nil { return }
        ShrinkBudget.ambient = previous
        lock.lock()
        closedFlag = true
        ShrinkBudget.lastWork = work
        ShrinkBudget.lastAttempts = attempts
        lock.unlock()
    }
}
