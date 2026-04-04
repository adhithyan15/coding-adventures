/// Deterministic Finite Automaton (DFA) — the workhorse of state machines.
///
/// === What is a DFA? ===
///
/// A DFA is the simplest kind of state machine. It has a fixed set of states,
/// reads input symbols one at a time, and follows exactly one transition for
/// each (state, input) pair. There is no ambiguity, no guessing, no backtracking.
///
/// Formally, a DFA is a 5-tuple (Q, Sigma, delta, q0, F):
///
///     Q     = a finite set of states
///     Sigma = a finite set of input symbols (the "alphabet")
///     delta = a transition function: Q x Sigma -> Q
///     q0    = the initial state (q0 in Q)
///     F     = a set of accepting/final states (F is a subset of Q)
///
/// === Why "deterministic"? ===
///
/// "Deterministic" means there is exactly ONE next state for every (state, input)
/// combination. Given the same starting state and the same input sequence, a DFA
/// always follows the same path and reaches the same final state. This makes DFAs
/// predictable, efficient, and easy to implement in hardware — which is why they
/// appear everywhere from CPU branch predictors to network protocol handlers.
///
/// === Example: a turnstile ===
///
/// A turnstile at a subway station has two states: locked and unlocked.
/// Insert a coin -> it unlocks. Push the arm -> it locks.
///
///     States:      {locked, unlocked}
///     Alphabet:    {coin, push}
///     Transitions: (locked, coin) -> unlocked
///                  (locked, push) -> locked
///                  (unlocked, coin) -> unlocked
///                  (unlocked, push) -> locked
///     Initial:     locked
///     Accepting:   {unlocked}
///
/// This DFA answers the question: "after this sequence of coin/push events,
/// is the turnstile unlocked?"
///
/// === Connection to existing code ===
///
/// The 2-bit branch predictor in the branch-predictor package (D02) is a DFA:
///
///     States:      {SNT, WNT, WT, ST}  (strongly/weakly not-taken/taken)
///     Alphabet:    {taken, not_taken}
///     Transitions: defined by the saturating counter logic
///     Initial:     WNT
///     Accepting:   {WT, ST}  (states that predict "taken")

// MARK: - Transition Record

/// One step in a state machine's execution trace.
///
/// Every time a machine processes an input and transitions from one state
/// to another, a ``TransitionRecord`` is created. This gives complete
/// visibility into the machine's execution history.
///
/// === Why trace everything? ===
///
/// In the coding-adventures philosophy, we want to be able to trace any
/// computation all the way down to the logic gates that implement it.
/// TransitionRecords are the state machine layer's contribution to that
/// trace: they record exactly what happened, when, and why.
///
/// You can replay an execution by walking through its list of
/// TransitionRecords. You can verify correctness by checking that the
/// source of each record matches the target of the previous one.
///
/// === Fields ===
///
/// - `source`: the state before the transition
/// - `event`: the input that triggered it
/// - `target`: the state after the transition
/// - `actionName`: the name of the action that fired, if any
public struct TransitionRecord: Sendable, Equatable {
    /// The state before the transition.
    public let source: String
    /// The input that triggered it.
    public let event: String
    /// The state after the transition.
    public let target: String
    /// The name of the action that fired, if any.
    public let actionName: String?
}

// MARK: - Transition Key

/// Encode a (state, event) pair as a single string key for Dictionary lookup.
///
/// Uses a null character ("\0") as separator to avoid ambiguity — no valid
/// state or event name should contain a null character.
///
/// This is the same technique used in the TypeScript implementation. In
/// languages with tuple-keyed dicts (like Python), you would use a tuple
/// directly. In Swift, while we could use a struct as a dictionary key,
/// using a string key is simpler and matches the cross-language convention.
///
/// - Parameters:
///   - state: The state part of the key
///   - event: The event part of the key
/// - Returns: A string key like "locked\0coin"
public func transitionKey(_ state: String, _ event: String) -> String {
    "\(state)\0\(event)"
}

// MARK: - Action Type

/// A callback executed when a transition fires.
///
/// The three arguments are: (source_state, event, target_state).
///
/// Actions are optional side effects — logging, incrementing counters,
/// emitting tokens, etc. The state machine itself does not depend on
/// action return values; actions are fire-and-forget.
public typealias Action = @Sendable (String, String, String) -> Void

// MARK: - DFA Errors

/// Errors that can occur during DFA construction or processing.
///
/// These are thrown eagerly — at construction time or at the moment of
/// an invalid operation — following the "fail fast" principle. The error
/// messages are designed to be human-readable and to help debug the
/// issue immediately.
public enum DFAError: Error, Sendable, Equatable {
    /// The states set was empty.
    case emptyStates
    /// The initial state is not in the states set.
    case initialStateNotInStates(state: String)
    /// An accepting state is not in the states set.
    case acceptingStateNotInStates(state: String)
    /// A transition's source state is not in the states set.
    case transitionSourceNotInStates(source: String)
    /// A transition's event is not in the alphabet.
    case transitionEventNotInAlphabet(event: String)
    /// A transition's target state is not in the states set.
    case transitionTargetNotInStates(target: String, source: String, event: String)
    /// An action was defined for a (state, event) pair that has no transition.
    case actionWithoutTransition(source: String, event: String)
    /// An event was processed that is not in the alphabet.
    case eventNotInAlphabet(event: String)
    /// No transition is defined for the current (state, event) pair.
    case noTransition(state: String, event: String)
}

// MARK: - DFA

/// Deterministic Finite Automaton.
///
/// A DFA is always in exactly one state. Each input causes exactly one
/// transition. If no transition is defined for the current (state, input)
/// pair, processing that input throws an error.
///
/// All transitions are traced via ``TransitionRecord`` objects, providing
/// complete execution history for debugging and visualization.
///
/// ### Usage
///
/// ```swift
/// let turnstile = try DFA(
///     states: ["locked", "unlocked"],
///     alphabet: ["coin", "push"],
///     transitions: [
///         TransitionRule(from: "locked", on: "coin", to: "unlocked"),
///         TransitionRule(from: "locked", on: "push", to: "locked"),
///         TransitionRule(from: "unlocked", on: "coin", to: "unlocked"),
///         TransitionRule(from: "unlocked", on: "push", to: "locked"),
///     ],
///     initial: "locked",
///     accepting: ["unlocked"]
/// )
/// try turnstile.process("coin") // "unlocked"
/// turnstile.accepts(["coin", "push", "coin"]) // true
/// ```
public final class DFA: @unchecked Sendable {

    // MARK: - Transition Rule (convenience for construction)

    /// A named transition rule: in state `from`, on event `on`, go to state `to`.
    ///
    /// This is a convenience type for constructing DFAs. Internally the DFA
    /// stores transitions in a Dictionary keyed by ``transitionKey(_:_:)``,
    /// but ``TransitionRule`` is more readable in user code.
    public struct TransitionRule: Sendable {
        public let from: String
        public let on: String
        public let to: String

        public init(from: String, on: String, to: String) {
            self.from = from
            self.on = on
            self.to = to
        }
    }

    // MARK: - Internal State

    // We store the 5-tuple as private let fields, plus mutable execution
    // state (_current and _trace). The let fields ensure the DFA's
    // definition cannot be mutated after construction.

    /// The finite set of states (Q).
    private let _states: Set<String>
    /// The finite set of input symbols (Sigma).
    private let _alphabet: Set<String>
    /// The transition function, encoded as a dictionary from
    /// transitionKey(state, event) -> target state.
    private let _transitions: [String: String]
    /// The initial state (q0).
    private let _initial: String
    /// The set of accepting/final states (F).
    private let _accepting: Set<String>
    /// Optional actions keyed by transitionKey(state, event).
    private let _actions: [String: Action]

    /// The state the machine is currently in (mutable).
    private var _current: String
    /// The execution trace (mutable).
    private var _trace: [TransitionRecord]

    // MARK: - Construction

    /// Create a new DFA.
    ///
    /// We validate all inputs eagerly in the initializer so that errors are
    /// caught at definition time, not at runtime when the machine processes
    /// its first input. This is the "fail fast" principle.
    ///
    /// - Parameters:
    ///   - states: The finite set of states. Must be non-empty.
    ///   - alphabet: The finite set of input symbols. Must be non-empty.
    ///   - transitions: A list of ``TransitionRule`` values. Every source
    ///     and target must be in `states`, every event must be in `alphabet`.
    ///   - initial: The starting state. Must be in `states`.
    ///   - accepting: The set of accepting/final states. Must be a subset of `states`.
    ///   - actions: Optional dictionary from transitionKey(state, event)
    ///     to a callback function that fires when that transition occurs.
    /// - Throws: ``DFAError`` if any validation check fails.
    public init(
        states: Set<String>,
        alphabet: Set<String>,
        transitions: [TransitionRule],
        initial: String,
        accepting: Set<String>,
        actions: [String: Action] = [:]
    ) throws(DFAError) {
        // --- Validate states ---
        guard !states.isEmpty else {
            throw .emptyStates
        }

        // --- Validate initial state ---
        guard states.contains(initial) else {
            throw .initialStateNotInStates(state: initial)
        }

        // --- Validate accepting states ---
        for s in accepting {
            guard states.contains(s) else {
                throw .acceptingStateNotInStates(state: s)
            }
        }

        // --- Build and validate transitions ---
        //
        // Every transition must go FROM a known state ON a known event
        // TO a known state. We check all three.
        var transitionDict: [String: String] = [:]
        for rule in transitions {
            guard states.contains(rule.from) else {
                throw .transitionSourceNotInStates(source: rule.from)
            }
            guard alphabet.contains(rule.on) else {
                throw .transitionEventNotInAlphabet(event: rule.on)
            }
            guard states.contains(rule.to) else {
                throw .transitionTargetNotInStates(
                    target: rule.to, source: rule.from, event: rule.on
                )
            }
            transitionDict[transitionKey(rule.from, rule.on)] = rule.to
        }

        // --- Validate actions ---
        for key in actions.keys {
            guard transitionDict[key] != nil else {
                let sep = key.firstIndex(of: "\0")!
                let source = String(key[key.startIndex..<sep])
                let event = String(key[key.index(after: sep)...])
                throw .actionWithoutTransition(source: source, event: event)
            }
        }

        // --- Store the 5-tuple + extras ---
        self._states = states
        self._alphabet = alphabet
        self._transitions = transitionDict
        self._initial = initial
        self._accepting = accepting
        self._actions = actions

        // --- Mutable execution state ---
        self._current = initial
        self._trace = []
    }

    // MARK: - Alternative initializer (raw dictionary)

    /// Create a new DFA from a raw transition dictionary.
    ///
    /// This initializer accepts transitions as a dictionary keyed by
    /// ``transitionKey(_:_:)`` strings. It is useful when constructing
    /// DFAs programmatically (e.g., from NFA subset construction).
    ///
    /// - Parameters:
    ///   - states: The finite set of states.
    ///   - alphabet: The finite set of input symbols.
    ///   - transitionDict: Dictionary from transitionKey(state, event) -> target.
    ///   - initial: The starting state.
    ///   - accepting: The set of accepting/final states.
    /// - Throws: ``DFAError`` if any validation check fails.
    public init(
        states: Set<String>,
        alphabet: Set<String>,
        transitionDict: [String: String],
        initial: String,
        accepting: Set<String>
    ) throws(DFAError) {
        guard !states.isEmpty else {
            throw .emptyStates
        }
        guard states.contains(initial) else {
            throw .initialStateNotInStates(state: initial)
        }
        for s in accepting {
            guard states.contains(s) else {
                throw .acceptingStateNotInStates(state: s)
            }
        }
        for (key, target) in transitionDict {
            let sep = key.firstIndex(of: "\0")!
            let source = String(key[key.startIndex..<sep])
            let event = String(key[key.index(after: sep)...])
            guard states.contains(source) else {
                throw .transitionSourceNotInStates(source: source)
            }
            guard alphabet.contains(event) else {
                throw .transitionEventNotInAlphabet(event: event)
            }
            guard states.contains(target) else {
                throw .transitionTargetNotInStates(
                    target: target, source: source, event: event
                )
            }
        }

        self._states = states
        self._alphabet = alphabet
        self._transitions = transitionDict
        self._initial = initial
        self._accepting = accepting
        self._actions = [:]
        self._current = initial
        self._trace = []
    }

    // MARK: - Properties

    /// The finite set of states.
    public var states: Set<String> { _states }

    /// The finite set of input symbols.
    public var alphabet: Set<String> { _alphabet }

    /// The transition function as a dictionary (copy).
    public var transitions: [String: String] { _transitions }

    /// The initial state.
    public var initial: String { _initial }

    /// The set of accepting/final states.
    public var accepting: Set<String> { _accepting }

    /// The state the machine is currently in.
    public var currentState: String { _current }

    /// The execution trace — a list of all transitions taken so far (copy).
    public var trace: [TransitionRecord] { _trace }

    // MARK: - Processing

    /// Process a single input event and return the new state.
    ///
    /// Looks up the transition for (currentState, event), moves to the
    /// target state, executes the action (if defined), logs a
    /// ``TransitionRecord``, and returns the new current state.
    ///
    /// - Parameter event: An input symbol from the alphabet.
    /// - Returns: The new current state after the transition.
    /// - Throws: ``DFAError`` if the event is not in the alphabet, or if no
    ///   transition is defined for (currentState, event).
    @discardableResult
    public func process(_ event: String) throws(DFAError) -> String {
        // Validate the event
        guard _alphabet.contains(event) else {
            throw .eventNotInAlphabet(event: event)
        }

        // Look up the transition
        let key = transitionKey(_current, event)
        guard let target = _transitions[key] else {
            throw .noTransition(state: _current, event: event)
        }

        // Execute the action if one exists
        var actionName: String? = nil
        if let action = _actions[key] {
            action(_current, event, target)
            actionName = String(describing: action)
        }

        // Log the transition
        let record = TransitionRecord(
            source: _current,
            event: event,
            target: target,
            actionName: actionName
        )
        _trace.append(record)

        // Move to the new state
        _current = target
        return target
    }

    /// Process a sequence of inputs and return the trace.
    ///
    /// Each input is processed in order. The trace of transitions for
    /// just this sequence is returned. The machine's state is updated
    /// after each input.
    ///
    /// - Parameter events: A list of input symbols.
    /// - Returns: A list of ``TransitionRecord`` objects, one per input.
    /// - Throws: ``DFAError`` if any event is invalid or has no transition.
    @discardableResult
    public func processSequence(_ events: [String]) throws(DFAError) -> [TransitionRecord] {
        let traceStart = _trace.count
        for event in events {
            try process(event)
        }
        return Array(_trace[traceStart...])
    }

    /// Check if the machine accepts the input sequence.
    ///
    /// Processes the entire sequence and returns true if the machine
    /// ends in an accepting state.
    ///
    /// **Important:** This method does NOT modify the machine's current state
    /// or trace. It runs on a fresh simulation starting from the initial state.
    ///
    /// - Parameter events: A list of input symbols.
    /// - Returns: `true` if the machine ends in an accepting state after
    ///   processing all inputs, `false` otherwise.
    public func accepts(_ events: [String]) -> Bool {
        // Run on a copy so we don't modify this machine's state
        var state = _initial
        for event in events {
            guard _alphabet.contains(event) else {
                return false
            }
            let key = transitionKey(state, event)
            guard let target = _transitions[key] else {
                return false
            }
            state = target
        }
        return _accepting.contains(state)
    }

    /// Reset the machine to its initial state and clear the trace.
    ///
    /// After reset, the machine is in the same state as when it was
    /// first constructed — as if no inputs had ever been processed.
    public func reset() {
        _current = _initial
        _trace = []
    }

    // MARK: - Introspection

    /// Return the set of states reachable from the initial state.
    ///
    /// Performs a BFS over the transition graph. A state is reachable
    /// if there exists any sequence of inputs that leads from the initial
    /// state to that state.
    ///
    /// States that are defined but not reachable are "dead weight" —
    /// they can never be entered and can be safely removed during
    /// minimization.
    ///
    /// - Returns: A Set of reachable state names.
    public func reachableStates() -> Set<String> {
        // BFS from the initial state
        var visited: Set<String> = [_initial]
        var queue: [String] = [_initial]

        while !queue.isEmpty {
            let state = queue.removeFirst()
            for event in _alphabet {
                let key = transitionKey(state, event)
                if let target = _transitions[key], !visited.contains(target) {
                    visited.insert(target)
                    queue.append(target)
                }
            }
        }

        return visited
    }

    /// Check if a transition is defined for every (state, input) pair.
    ///
    /// A complete DFA never gets "stuck" — every state handles every
    /// input. Textbook DFAs are usually complete (missing transitions
    /// go to an explicit "dead" or "trap" state). Practical DFAs often
    /// omit transitions to save space, treating missing transitions as
    /// errors.
    ///
    /// - Returns: `true` if every (state, event) pair has a defined transition.
    public func isComplete() -> Bool {
        for state in _states {
            for event in _alphabet {
                if _transitions[transitionKey(state, event)] == nil {
                    return false
                }
            }
        }
        return true
    }

    /// Check for common issues and return a list of warnings.
    ///
    /// Checks performed:
    /// - Unreachable states (defined but never entered)
    /// - Missing transitions (incomplete DFA)
    /// - Accepting states that are unreachable
    ///
    /// - Returns: A list of warning messages. Empty if no issues found.
    public func validate() -> [String] {
        var warnings: [String] = []

        // Check for unreachable states
        let reachable = reachableStates()
        let unreachable = _states.subtracting(reachable).sorted()
        if !unreachable.isEmpty {
            warnings.append("Unreachable states: [\(unreachable.joined(separator: ", "))]")
        }

        // Check for unreachable accepting states
        let unreachableAccepting = _accepting.subtracting(reachable).sorted()
        if !unreachableAccepting.isEmpty {
            warnings.append(
                "Unreachable accepting states: [\(unreachableAccepting.joined(separator: ", "))]"
            )
        }

        // Check for missing transitions
        var missing: [String] = []
        for state in _states.sorted() {
            for event in _alphabet.sorted() {
                if _transitions[transitionKey(state, event)] == nil {
                    missing.append("(\(state), \(event))")
                }
            }
        }
        if !missing.isEmpty {
            warnings.append("Missing transitions: \(missing.joined(separator: ", "))")
        }

        return warnings
    }
}
