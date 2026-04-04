/// Non-deterministic Finite Automaton (NFA) with epsilon transitions.
///
/// === What is an NFA? ===
///
/// An NFA relaxes the deterministic constraint of a DFA in two ways:
///
/// 1. **Multiple transitions:** A single (state, input) pair can lead to
///    multiple target states. The machine explores all possibilities
///    simultaneously — like spawning parallel universes.
///
/// 2. **Epsilon transitions:** The machine can jump to another state
///    without consuming any input. These are "free" moves.
///
/// === The "parallel universes" model ===
///
/// Think of an NFA as a machine that clones itself at every non-deterministic
/// choice point. All clones run in parallel:
///
///     - A clone that reaches a dead end (no transition) simply vanishes.
///     - A clone that reaches an accepting state means the whole NFA accepts.
///     - If ALL clones die without reaching an accepting state, the NFA rejects.
///
/// The NFA accepts if there EXISTS at least one path through the machine
/// that ends in an accepting state.
///
/// === Why NFAs matter ===
///
/// NFAs are much easier to construct for certain problems. For example, "does
/// this string contain the substring 'abc'?" is trivial as an NFA (just guess
/// where 'abc' starts) but requires careful tracking as a DFA.
///
/// Every NFA can be converted to an equivalent DFA via subset construction.
/// This is how regex engines work: regex -> NFA (easy) -> DFA (mechanical) ->
/// efficient execution (O(1) per character).
///
/// === Formal definition ===
///
///     NFA = (Q, Sigma, delta, q0, F)
///
///     Q     = finite set of states
///     Sigma = finite alphabet (input symbols)
///     delta = transition function: Q x (Sigma | {epsilon}) -> P(Q)
///           maps (state, input_or_epsilon) to a SET of states
///     q0    = initial state
///     F     = accepting states

// MARK: - Epsilon Sentinel

/// Sentinel value for epsilon transitions (transitions that consume no input).
///
/// We use the empty string "" as the epsilon symbol. This works because
/// no real input alphabet should contain the empty string — input symbols
/// are always at least one character long.
///
/// Used as the event in transition keys to represent "free" moves that
/// don't consume any input character.
public let EPSILON = ""

// MARK: - NFA Errors

/// Errors that can occur during NFA construction or processing.
public enum NFAError: Error, Sendable, Equatable {
    /// The states set was empty.
    case emptyStates
    /// The alphabet contains the empty string (reserved for epsilon).
    case epsilonInAlphabet
    /// The initial state is not in the states set.
    case initialStateNotInStates(state: String)
    /// An accepting state is not in the states set.
    case acceptingStateNotInStates(state: String)
    /// A transition's source state is not in the states set.
    case transitionSourceNotInStates(source: String)
    /// A transition's event is not in the alphabet and is not epsilon.
    case transitionEventNotInAlphabet(event: String)
    /// A transition's target state is not in the states set.
    case transitionTargetNotInStates(target: String, source: String, event: String)
    /// An event was processed that is not in the alphabet.
    case eventNotInAlphabet(event: String)
}

// MARK: - NFA Transition Rule

/// A named NFA transition rule: in state `from`, on event `on`, go to states `to`.
///
/// Unlike a DFA transition which has exactly one target, an NFA transition
/// can have multiple targets (non-determinism). The `on` field can be
/// ``EPSILON`` for epsilon transitions.
public struct NFATransitionRule: Sendable {
    public let from: String
    public let on: String
    public let to: Set<String>

    public init(from: String, on: String, to: Set<String>) {
        self.from = from
        self.on = on
        self.to = to
    }
}

// MARK: - NFA

/// Non-deterministic Finite Automaton with epsilon transitions.
///
/// An NFA can be in multiple states simultaneously. Processing an input
/// event means: for each current state, find all transitions on that
/// event, take the union of target states, then compute the epsilon
/// closure of the result.
///
/// The NFA accepts an input sequence if, after processing all inputs,
/// ANY of the current states is an accepting state.
///
/// ### Usage
///
/// ```swift
/// // NFA that accepts strings containing "ab"
/// let nfa = try NFA(
///     states: ["q0", "q1", "q2"],
///     alphabet: ["a", "b"],
///     transitions: [
///         NFATransitionRule(from: "q0", on: "a", to: ["q0", "q1"]),
///         NFATransitionRule(from: "q0", on: "b", to: ["q0"]),
///         NFATransitionRule(from: "q1", on: "b", to: ["q2"]),
///         NFATransitionRule(from: "q2", on: "a", to: ["q2"]),
///         NFATransitionRule(from: "q2", on: "b", to: ["q2"]),
///     ],
///     initial: "q0",
///     accepting: ["q2"]
/// )
/// nfa.accepts(["a", "b"]) // true
/// nfa.accepts(["b", "a"]) // false
/// ```
public final class NFA: @unchecked Sendable {

    // MARK: - Internal State

    /// The finite set of states (Q).
    private let _states: Set<String>
    /// The finite set of input symbols (Sigma).
    private let _alphabet: Set<String>
    /// The transition function: transitionKey(state, event) -> Set of target states.
    /// Epsilon transitions use EPSILON ("") as the event.
    private let _transitions: [String: Set<String>]
    /// The initial state (q0).
    private let _initial: String
    /// The set of accepting/final states (F).
    private let _accepting: Set<String>

    /// The set of states the NFA is currently in (mutable).
    /// Starts as the epsilon closure of the initial state.
    private var _current: Set<String>

    // MARK: - Construction

    /// Create a new NFA.
    ///
    /// - Parameters:
    ///   - states: The finite set of states. Must be non-empty.
    ///   - alphabet: The finite set of input symbols. Must not contain
    ///     the empty string (reserved for epsilon).
    ///   - transitions: A list of ``NFATransitionRule`` values.
    ///   - initial: The starting state. Must be in `states`.
    ///   - accepting: The set of accepting/final states.
    /// - Throws: ``NFAError`` if any validation check fails.
    public init(
        states: Set<String>,
        alphabet: Set<String>,
        transitions: [NFATransitionRule],
        initial: String,
        accepting: Set<String>
    ) throws(NFAError) {
        guard !states.isEmpty else {
            throw .emptyStates
        }
        guard !alphabet.contains(EPSILON) else {
            throw .epsilonInAlphabet
        }
        guard states.contains(initial) else {
            throw .initialStateNotInStates(state: initial)
        }
        for s in accepting {
            guard states.contains(s) else {
                throw .acceptingStateNotInStates(state: s)
            }
        }

        // Build and validate transitions
        var transitionDict: [String: Set<String>] = [:]
        for rule in transitions {
            guard states.contains(rule.from) else {
                throw .transitionSourceNotInStates(source: rule.from)
            }
            guard rule.on == EPSILON || alphabet.contains(rule.on) else {
                throw .transitionEventNotInAlphabet(event: rule.on)
            }
            for t in rule.to {
                guard states.contains(t) else {
                    throw .transitionTargetNotInStates(
                        target: t, source: rule.from, event: rule.on
                    )
                }
            }
            let key = transitionKey(rule.from, rule.on)
            // Merge if multiple rules for the same (state, event) pair
            if let existing = transitionDict[key] {
                transitionDict[key] = existing.union(rule.to)
            } else {
                transitionDict[key] = rule.to
            }
        }

        self._states = states
        self._alphabet = alphabet
        self._transitions = transitionDict
        self._initial = initial
        self._accepting = accepting

        // The NFA starts in the epsilon closure of the initial state
        self._current = NFA.computeEpsilonClosure(
            [initial], transitions: transitionDict
        )
    }

    // MARK: - Properties

    /// The finite set of states.
    public var states: Set<String> { _states }

    /// The finite set of input symbols.
    public var alphabet: Set<String> { _alphabet }

    /// The initial state.
    public var initial: String { _initial }

    /// The set of accepting/final states.
    public var accepting: Set<String> { _accepting }

    /// The set of states the NFA is currently in.
    public var currentStates: Set<String> { _current }

    // MARK: - Epsilon Closure

    /// Compute the epsilon closure of a set of states.
    ///
    /// Starting from the given states, follow ALL epsilon transitions
    /// recursively. Return the full set of states reachable via zero or
    /// more epsilon transitions.
    ///
    /// This is the key operation that makes NFAs work: before and after
    /// processing each input, we expand to include all states reachable
    /// via "free" epsilon moves.
    ///
    /// The algorithm is a simple BFS over epsilon edges:
    ///
    ///     1. Start with the input set
    ///     2. For each state, find epsilon transitions
    ///     3. Add all targets to the set
    ///     4. Repeat until no new states are found
    ///
    /// - Parameter states: The starting set of states.
    /// - Returns: A Set of all states reachable via epsilon transitions
    ///   from any state in the input set.
    public func epsilonClosure(_ states: Set<String>) -> Set<String> {
        NFA.computeEpsilonClosure(states, transitions: _transitions)
    }

    /// Static helper for epsilon closure computation.
    ///
    /// This is a static method so it can be called from the initializer
    /// before `self` is fully initialized.
    private static func computeEpsilonClosure(
        _ states: Set<String>,
        transitions: [String: Set<String>]
    ) -> Set<String> {
        var closure = states
        var worklist = Array(states)

        while !worklist.isEmpty {
            let state = worklist.removeLast()
            // Find epsilon transitions from this state
            let key = transitionKey(state, EPSILON)
            if let targets = transitions[key] {
                for target in targets {
                    if !closure.contains(target) {
                        closure.insert(target)
                        worklist.append(target)
                    }
                }
            }
        }

        return closure
    }

    // MARK: - Processing

    /// Process one input event and return the new set of states.
    ///
    /// For each current state, find all transitions on this event.
    /// Take the union of all target states, then compute the epsilon
    /// closure of the result.
    ///
    /// - Parameter event: An input symbol from the alphabet.
    /// - Returns: The new set of current states after processing.
    /// - Throws: ``NFAError`` if the event is not in the alphabet.
    @discardableResult
    public func process(_ event: String) throws(NFAError) -> Set<String> {
        guard _alphabet.contains(event) else {
            throw .eventNotInAlphabet(event: event)
        }

        // Collect all target states from all current states
        var nextStates: Set<String> = []
        for state in _current {
            let key = transitionKey(state, event)
            if let targets = _transitions[key] {
                nextStates.formUnion(targets)
            }
        }

        // Expand via epsilon closure
        _current = epsilonClosure(nextStates)
        return _current
    }

    /// Process a sequence of inputs and return the trace.
    ///
    /// Each entry in the trace is: (states_before, event, states_after).
    ///
    /// - Parameter events: A list of input symbols.
    /// - Returns: A list of (before_states, event, after_states) tuples.
    /// - Throws: ``NFAError`` if any event is invalid.
    public func processSequence(
        _ events: [String]
    ) throws(NFAError) -> [(Set<String>, String, Set<String>)] {
        var traceEntries: [(Set<String>, String, Set<String>)] = []
        for event in events {
            let before = _current
            try process(event)
            traceEntries.append((before, event, _current))
        }
        return traceEntries
    }

    /// Check if the NFA accepts the input sequence.
    ///
    /// The NFA accepts if, after processing all inputs, ANY of the
    /// current states is an accepting state.
    ///
    /// Does NOT modify the NFA's current state — runs on a copy.
    ///
    /// - Parameter events: A list of input symbols.
    /// - Returns: `true` if the NFA accepts, `false` otherwise.
    public func accepts(_ events: [String]) -> Bool {
        // Simulate without modifying this NFA's state
        var current = NFA.computeEpsilonClosure(
            [_initial], transitions: _transitions
        )

        for event in events {
            guard _alphabet.contains(event) else {
                return false
            }
            var nextStates: Set<String> = []
            for state in current {
                let key = transitionKey(state, event)
                if let targets = _transitions[key] {
                    nextStates.formUnion(targets)
                }
            }
            current = NFA.computeEpsilonClosure(nextStates, transitions: _transitions)

            // If no states are active, the NFA is dead — reject early
            if current.isEmpty {
                return false
            }
        }

        return !current.isDisjoint(with: _accepting)
    }

    /// Reset to the initial state (with epsilon closure).
    public func reset() {
        _current = NFA.computeEpsilonClosure(
            [_initial], transitions: _transitions
        )
    }

    // MARK: - Conversion to DFA

    /// Convert this NFA to an equivalent DFA using subset construction.
    ///
    /// === The Subset Construction Algorithm ===
    ///
    /// The key insight: if an NFA can be in states {q0, q1, q3}
    /// simultaneously, we create a single DFA state representing that
    /// entire set. The DFA's states are sets of NFA states.
    ///
    /// Algorithm:
    ///     1. Start with d0 = epsilon-closure({q0})
    ///     2. For each DFA state D and each input symbol a:
    ///         - For each NFA state q in D, find delta(q, a)
    ///         - Take the union of all targets
    ///         - Compute epsilon-closure of the union
    ///         - That is the new DFA state D'
    ///     3. Repeat until no new DFA states are discovered
    ///     4. A DFA state is accepting if it contains ANY NFA accepting state
    ///
    /// DFA state names are generated from sorted NFA state names:
    ///     Set(["q0", "q1"]) -> "{q0,q1}"
    ///
    /// - Returns: A ``DFA`` that recognizes exactly the same language as this NFA.
    /// - Throws: ``DFAError`` if the resulting DFA fails validation (should not
    ///   happen for a correctly constructed NFA).
    public func toDfa() throws -> DFA {
        // Step 1: initial DFA state = epsilon-closure of NFA initial state
        let startClosure = NFA.computeEpsilonClosure(
            [_initial], transitions: _transitions
        )
        let dfaStart = stateSetName(startClosure)

        // Track DFA states and transitions as we discover them
        var dfaStates: Set<String> = [dfaStart]
        var dfaTransitions: [String: String] = [:]
        var dfaAccepting: Set<String> = []

        // Map from DFA state name -> Set of NFA states
        var stateMap: [String: Set<String>] = [dfaStart: startClosure]

        // Check if start state is accepting
        if !startClosure.isDisjoint(with: _accepting) {
            dfaAccepting.insert(dfaStart)
        }

        // Step 2-3: BFS over DFA states
        var worklist: [String] = [dfaStart]

        while !worklist.isEmpty {
            let currentName = worklist.removeLast()
            let currentNfaStates = stateMap[currentName]!

            for event in _alphabet.sorted() {
                // Collect all NFA states reachable via this event
                var nextNfa: Set<String> = []
                for nfaState in currentNfaStates {
                    let key = transitionKey(nfaState, event)
                    if let targets = _transitions[key] {
                        nextNfa.formUnion(targets)
                    }
                }

                // Epsilon closure of the result
                let nextClosure = NFA.computeEpsilonClosure(
                    nextNfa, transitions: _transitions
                )

                if nextClosure.isEmpty {
                    // Dead state — no transition (DFA will be incomplete)
                    continue
                }

                let nextName = stateSetName(nextClosure)

                // Record this DFA transition
                dfaTransitions[transitionKey(currentName, event)] = nextName

                // If this is a new DFA state, add it to the worklist
                if !dfaStates.contains(nextName) {
                    dfaStates.insert(nextName)
                    stateMap[nextName] = nextClosure
                    worklist.append(nextName)

                    // Check if accepting
                    if !nextClosure.isDisjoint(with: _accepting) {
                        dfaAccepting.insert(nextName)
                    }
                }
            }
        }

        return try DFA(
            states: dfaStates,
            alphabet: _alphabet,
            transitionDict: dfaTransitions,
            initial: dfaStart,
            accepting: dfaAccepting
        )
    }
}

// MARK: - State Set Naming

/// Convert a Set of state names to a DFA state name.
///
/// The name is deterministic: sorted state names joined with commas
/// and wrapped in braces. This ensures that the same set of NFA states
/// always produces the same DFA state name, regardless of iteration order.
///
/// - Parameter states: The set of NFA states.
/// - Returns: A canonical name like "{q0,q1,q2}".
public func stateSetName(_ states: Set<String>) -> String {
    "{\(states.sorted().joined(separator: ","))}"
}
