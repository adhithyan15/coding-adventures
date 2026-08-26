package com.codingadventures.statemachine

import java.util.ArrayDeque
import java.util.ArrayList
import java.util.Collections
import java.util.LinkedHashMap
import java.util.LinkedHashSet

private fun <T> immutableList(values: Collection<T>): List<T> =
    Collections.unmodifiableList(ArrayList(values))

private fun <T> immutableSet(values: Collection<T>): Set<T> =
    Collections.unmodifiableSet(LinkedHashSet(values))

private fun <K, V> immutableMap(values: Map<K, V>): Map<K, V> =
    Collections.unmodifiableMap(LinkedHashMap(values))

data class TransitionKey(val state: String, val event: String)
data class TransitionRecord(val source: String, val event: String, val target: String, val actionName: String?)
data class TransitionAction(
    val name: String,
    val effect: (source: String, event: String, target: String) -> Unit,
) {
    init {
        require(name.isNotBlank()) { "action name must not be blank" }
    }
}

class NfaTraceEntry(source: Set<String>, val event: String, target: Set<String>) {
    val source: Set<String> = immutableSet(source)
    val target: Set<String> = immutableSet(target)

    override fun equals(other: Any?): Boolean =
        other is NfaTraceEntry && source == other.source && event == other.event && target == other.target

    override fun hashCode(): Int = listOf(source, event, target).hashCode()

    override fun toString(): String = "NfaTraceEntry(source=$source, event=$event, target=$target)"
}

data class ModeTransitionKey(val mode: String, val trigger: String)
data class ModeTransitionRecord(val fromMode: String, val trigger: String, val toMode: String)
class PdaTransition(
    val source: String,
    val event: String?,
    val stackRead: String,
    val target: String,
    stackPush: List<String>,
) {
    val stackPush: List<String> = immutableList(stackPush)

    override fun equals(other: Any?): Boolean = other is PdaTransition &&
        source == other.source && event == other.event && stackRead == other.stackRead &&
        target == other.target && stackPush == other.stackPush

    override fun hashCode(): Int = listOf(source, event, stackRead, target, stackPush).hashCode()

    override fun toString(): String =
        "PdaTransition(source=$source, event=$event, stackRead=$stackRead, target=$target, stackPush=$stackPush)"
}

class PdaTraceEntry(
    val source: String,
    val event: String?,
    val stackRead: String,
    val target: String,
    stackPush: List<String>,
    stackAfter: List<String>,
) {
    val stackPush: List<String> = immutableList(stackPush)
    val stackAfter: List<String> = immutableList(stackAfter)

    override fun equals(other: Any?): Boolean = other is PdaTraceEntry &&
        source == other.source && event == other.event && stackRead == other.stackRead &&
        target == other.target && stackPush == other.stackPush && stackAfter == other.stackAfter

    override fun hashCode(): Int = listOf(source, event, stackRead, target, stackPush, stackAfter).hashCode()

    override fun toString(): String =
        "PdaTraceEntry(source=$source, event=$event, stackRead=$stackRead, target=$target, " +
            "stackPush=$stackPush, stackAfter=$stackAfter)"
}

class Dfa(
    states: Set<String>,
    alphabet: Set<String>,
    transitions: Map<TransitionKey, String>,
    val initial: String,
    accepting: Set<String>,
    actions: Map<TransitionKey, TransitionAction> = emptyMap(),
    private val maxTraceEntries: Int = DEFAULT_MAX_TRACE_ENTRIES,
) {
    val states: Set<String> = immutableSet(states)
    val alphabet: Set<String> = immutableSet(alphabet)
    val transitions: Map<TransitionKey, String> = immutableMap(transitions)
    val accepting: Set<String> = immutableSet(accepting)
    private val actions: Map<TransitionKey, TransitionAction> = immutableMap(actions)
    private val mutableTrace = mutableListOf<TransitionRecord>()
    val trace: List<TransitionRecord> get() = immutableList(mutableTrace)
    var currentState: String = initial
        private set

    init {
        require(this.states.isNotEmpty()) { "DFA requires at least one state" }
        require(maxTraceEntries >= 0) { "maximum trace entries must be non-negative" }
        require(initial in this.states) { "initial state is not declared" }
        require(this.accepting.all { it in this.states }) { "accepting states must be declared" }
        for ((key, target) in this.transitions) {
            require(key.state in this.states && key.event in this.alphabet && target in this.states) {
                "transition references an undeclared state or event"
            }
        }
        require(this.transitions.keys.containsAll(this.actions.keys)) { "actions require matching transitions" }
    }

    fun process(event: String): String {
        require(event in alphabet) { "event is not in the DFA alphabet" }
        val source = currentState
        val key = TransitionKey(source, event)
        val target = transitions[key] ?: throw IllegalStateException("no transition for current state and event")
        check(mutableTrace.size < maxTraceEntries) { "DFA trace limit exceeded" }
        val action = actions[key]
        currentState = target
        action?.effect?.invoke(source, event, target)
        mutableTrace += TransitionRecord(source, event, target, action?.name)
        return target
    }

    fun processSequence(events: List<String>): List<TransitionRecord> {
        val start = mutableTrace.size
        events.forEach(::process)
        return mutableTrace.drop(start)
    }

    fun accepts(events: List<String>): Boolean {
        var state = initial
        for (event in events) {
            if (event !in alphabet) return false
            state = transitions[TransitionKey(state, event)] ?: return false
        }
        return state in accepting
    }

    fun reset() {
        currentState = initial
        mutableTrace.clear()
    }

    val isAccepting: Boolean get() = currentState in accepting
    val isComplete: Boolean get() = states.all { state -> alphabet.all { TransitionKey(state, it) in transitions } }

    fun reachableStates(): Set<String> {
        val reached = linkedSetOf(initial)
        val queue = ArrayDeque<String>()
        queue += initial
        while (queue.isNotEmpty()) {
            val state = queue.removeFirst()
            for (event in alphabet) {
                val target = transitions[TransitionKey(state, event)]
                if (target != null && reached.add(target)) queue += target
            }
        }
        return reached.toSet()
    }

    fun validate(): List<String> {
        val reached = reachableStates()
        return buildList {
            for (state in states.sorted()) {
                if (state !in reached) add("unreachable state: $state")
                for (event in alphabet.sorted()) {
                    if (TransitionKey(state, event) !in transitions) add("missing transition: $state / $event")
                }
            }
        }
    }

    fun toTable(): List<List<String>> = buildList {
        val events = alphabet.sorted()
        add(listOf("State") + events)
        for (state in states.sorted()) {
            add(listOf(state) + events.map { transitions[TransitionKey(state, it)].orEmpty() })
        }
    }

    fun toAscii(): String = toTable().joinToString(separator = "\n", postfix = "\n") { it.joinToString(" | ") }

    fun toDot(): String = buildString {
        appendLine("digraph DFA {")
        appendLine("  rankdir=LR;")
        append("  node [shape=doublecircle];")
        accepting.sorted().forEach { append(" \"${escape(it)}\"") }
        appendLine(";")
        appendLine("  node [shape=circle];")
        appendLine("  __start [shape=point];")
        appendLine("  __start -> \"${escape(initial)}\";")
        transitions.toSortedMap(compareBy<TransitionKey> { it.state }.thenBy { it.event }).forEach { (key, target) ->
            appendLine("  \"${escape(key.state)}\" -> \"${escape(target)}\" [label=\"${escape(key.event)}\"];")
        }
        appendLine("}")
    }

    private fun escape(value: String): String = value.replace("\\", "\\\\").replace("\"", "\\\"")

    companion object {
        const val DEFAULT_MAX_TRACE_ENTRIES = 100_000
    }
}

class Nfa(
    states: Set<String>,
    alphabet: Set<String>,
    transitions: Map<TransitionKey, Set<String>>,
    val initial: String,
    accepting: Set<String>,
    private val maxDfaStates: Int = DEFAULT_MAX_DFA_STATES,
    private val maxTraceEntries: Int = DEFAULT_MAX_TRACE_ENTRIES,
    private val maxTraceStateCells: Long = DEFAULT_MAX_TRACE_STATE_CELLS,
) {
    val states: Set<String> = immutableSet(states)
    val alphabet: Set<String> = immutableSet(alphabet)
    val transitions: Map<TransitionKey, Set<String>> = immutableMap(
        transitions.mapValues { immutableSet(it.value) },
    )
    val accepting: Set<String> = immutableSet(accepting)
    private val mutableTrace = mutableListOf<NfaTraceEntry>()
    private var traceStateCells = 0L
    val trace: List<NfaTraceEntry> get() = immutableList(mutableTrace)
    var currentStates: Set<String>
        private set

    init {
        require(this.states.isNotEmpty()) { "NFA requires at least one state" }
        require(maxDfaStates >= 1 && maxTraceEntries >= 0 && maxTraceStateCells >= 0) {
            "NFA state and trace limits are invalid"
        }
        require(EPSILON !in this.alphabet) { "epsilon must not be part of the input alphabet" }
        require(initial in this.states && this.accepting.all { it in this.states }) {
            "initial and accepting states must be declared"
        }
        for ((key, targets) in this.transitions) {
            require(
                key.state in this.states &&
                    (key.event == EPSILON || key.event in this.alphabet) &&
                    targets.all { it in this.states },
            ) { "transition references an undeclared state or event" }
        }
        currentStates = epsilonClosure(setOf(initial))
    }

    fun epsilonClosure(seeds: Set<String>): Set<String> {
        require(seeds.all { it in states }) { "epsilon-closure seed is not declared" }
        val closure = seeds.toMutableSet()
        val queue = ArrayDeque(seeds)
        while (queue.isNotEmpty()) {
            val state = queue.removeFirst()
            for (target in transitions[TransitionKey(state, EPSILON)].orEmpty()) {
                if (closure.add(target)) queue += target
            }
        }
        return immutableSet(closure)
    }

    fun process(event: String): Set<String> {
        require(event in alphabet) { "event is not in the NFA alphabet" }
        check(mutableTrace.size < maxTraceEntries) { "NFA trace limit exceeded" }
        val source = currentStates
        val moved = source.flatMapTo(linkedSetOf()) { transitions[TransitionKey(it, event)].orEmpty() }
        val target = epsilonClosure(moved)
        val stateCells = source.size.toLong() + target.size
        check(stateCells <= maxTraceStateCells - traceStateCells) { "NFA trace state-cell limit exceeded" }
        currentStates = target
        mutableTrace += NfaTraceEntry(source, event, target)
        traceStateCells += stateCells
        return currentStates
    }

    fun processSequence(events: List<String>): List<NfaTraceEntry> {
        val start = mutableTrace.size
        events.forEach(::process)
        return mutableTrace.drop(start)
    }

    fun accepts(events: List<String>): Boolean {
        var active = epsilonClosure(setOf(initial))
        for (event in events) {
            if (event !in alphabet) return false
            active = epsilonClosure(active.flatMapTo(linkedSetOf()) { transitions[TransitionKey(it, event)].orEmpty() })
        }
        return active.any { it in accepting }
    }

    fun reset() {
        currentStates = epsilonClosure(setOf(initial))
        mutableTrace.clear()
        traceStateCells = 0
    }

    val isAccepting: Boolean get() = currentStates.any { it in accepting }

    fun toDfa(): Dfa {
        val startSet = epsilonClosure(setOf(initial))
        val names = mutableMapOf(startSet to "S0")
        val queue = ArrayDeque<Set<String>>()
        queue += startSet
        val dfaStates = linkedSetOf<String>()
        val dfaAccepting = linkedSetOf<String>()
        val dfaTransitions = mutableMapOf<TransitionKey, String>()
        while (queue.isNotEmpty()) {
            val active = queue.removeFirst()
            val sourceName = names.getValue(active)
            dfaStates += sourceName
            if (active.any { it in accepting }) dfaAccepting += sourceName
            for (event in alphabet) {
                val moved = active.flatMapTo(linkedSetOf()) { transitions[TransitionKey(it, event)].orEmpty() }
                val target = epsilonClosure(moved)
                val targetName = names[target] ?: run {
                    check(names.size < maxDfaStates) { "NFA subset construction exceeds configured limit" }
                    "S${names.size}".also {
                        names[target] = it
                        queue += target
                    }
                }
                dfaTransitions[TransitionKey(sourceName, event)] = targetName
            }
        }
        return Dfa(dfaStates, alphabet, dfaTransitions, names.getValue(startSet), dfaAccepting)
    }

    fun toDot(): String = buildString {
        appendLine("digraph NFA {")
        appendLine("  rankdir=LR;")
        append("  node [shape=doublecircle];")
        accepting.sorted().forEach { append(" \"${escape(it)}\"") }
        appendLine(";")
        appendLine("  node [shape=circle];")
        appendLine("  __start [shape=point];")
        appendLine("  __start -> \"${escape(initial)}\";")
        transitions.toSortedMap(compareBy<TransitionKey> { it.state }.thenBy { it.event }).forEach { (key, targets) ->
            val label = if (key.event == EPSILON) "ε" else key.event
            targets.sorted().forEach { target ->
                appendLine("  \"${escape(key.state)}\" -> \"${escape(target)}\" [label=\"${escape(label)}\"];")
            }
        }
        appendLine("}")
    }

    private fun escape(value: String): String = value.replace("\\", "\\\\").replace("\"", "\\\"")

    companion object {
        const val EPSILON = ""
        const val DEFAULT_MAX_DFA_STATES = 4_096
        const val DEFAULT_MAX_TRACE_ENTRIES = 100_000
        const val DEFAULT_MAX_TRACE_STATE_CELLS = 1_000_000L
    }
}

object DfaMinimizer {
    fun minimize(machine: Dfa): Dfa {
        val reachable = machine.reachableStates()
        val accepting = machine.accepting intersect reachable
        val rejecting = reachable - accepting
        var partitions = listOf(accepting, rejecting).filter { it.isNotEmpty() }
        var previousSize: Int
        do {
            previousSize = partitions.size
            val previous = partitions
            val partitionByState = buildMap {
                previous.forEachIndexed { index, partition -> partition.forEach { put(it, index) } }
            }
            partitions = previous.flatMap { partition ->
                partition.groupBy { state ->
                    machine.alphabet.sorted().map { event ->
                        machine.transitions[TransitionKey(state, event)]?.let(partitionByState::get) ?: -1
                    }
                }.values.map { it.toSet() }
            }
        } while (partitions.size != previousSize)

        val nameByState = mutableMapOf<String, String>()
        partitions.forEachIndexed { index, partition ->
            val name = "M$index"
            partition.forEach { nameByState[it] = name }
        }
        val minimizedTransitions = buildMap {
            for (partition in partitions) {
                val representative = partition.first()
                val source = nameByState.getValue(representative)
                for (event in machine.alphabet) {
                    machine.transitions[TransitionKey(representative, event)]?.let { target ->
                        put(TransitionKey(source, event), nameByState.getValue(target))
                    }
                }
            }
        }
        return Dfa(
            states = nameByState.values.toSet(),
            alphabet = machine.alphabet,
            transitions = minimizedTransitions,
            initial = nameByState.getValue(machine.initial),
            accepting = partitions.filter { partition -> partition.any { it in machine.accepting } }
                .mapTo(mutableSetOf()) { nameByState.getValue(it.first()) },
        )
    }
}

class PushdownAutomaton(
    states: Set<String>,
    inputAlphabet: Set<String>,
    stackAlphabet: Set<String>,
    transitions: List<PdaTransition>,
    val initial: String,
    val initialStackSymbol: String,
    accepting: Set<String>,
    private val maxStackDepth: Int = DEFAULT_MAX_STACK_DEPTH,
    private val maxTraceEntries: Int = DEFAULT_MAX_TRACE_ENTRIES,
) {
    private data class PdaKey(val state: String, val input: String?, val stackTop: String)
    private data class Configuration(val state: String, val stack: List<String>)

    val states = immutableSet(states)
    val inputAlphabet = immutableSet(inputAlphabet)
    val stackAlphabet = immutableSet(stackAlphabet)
    val transitions = immutableList(transitions)
    val accepting = immutableSet(accepting)
    private val transitionIndex: Map<PdaKey, PdaTransition>
    private val mutableTrace = mutableListOf<PdaTraceEntry>()
    val trace: List<PdaTraceEntry> get() = immutableList(mutableTrace)
    private val mutableStack = mutableListOf<String>()
    val stack: List<String> get() = immutableList(mutableStack)
    var currentState = initial
        private set

    init {
        require(this.states.isNotEmpty()) { "PDA requires at least one state" }
        require(maxStackDepth >= 1 && maxTraceEntries >= 0) { "PDA stack and trace limits are invalid" }
        require(initial in this.states && this.accepting.all { it in this.states }) {
            "initial and accepting states must be declared"
        }
        require(initialStackSymbol in this.stackAlphabet) { "initial stack symbol must be declared" }
        val index = mutableMapOf<PdaKey, PdaTransition>()
        for (transition in this.transitions) {
            require(
                transition.source in this.states && transition.target in this.states &&
                    (transition.event == null || transition.event in this.inputAlphabet) &&
                    transition.stackRead in this.stackAlphabet &&
                    transition.stackPush.all { it in this.stackAlphabet },
            ) { "PDA transition references an undeclared symbol or state" }
            val key = PdaKey(transition.source, transition.event, transition.stackRead)
            require(index.put(key, transition) == null) { "PDA transitions must be deterministic" }
        }
        transitionIndex = index.toMap()
        reset()
    }

    fun process(input: String): String {
        require(input in inputAlphabet) { "input is not in the PDA alphabet" }
        check(mutableStack.isNotEmpty()) { "PDA stack is empty" }
        val transition = transitionIndex[PdaKey(currentState, input, mutableStack.last())]
            ?: throw IllegalStateException("no PDA transition for current configuration")
        apply(transition)
        return currentState
    }

    fun processSequence(inputs: List<String>): List<PdaTraceEntry> = buildList {
        val start = mutableTrace.size
        for (input in inputs) {
            process(input)
        }
        closeEpsilon()
        addAll(mutableTrace.drop(start))
    }

    fun accepts(inputs: List<String>): Boolean {
        var state = initial
        var localStack = mutableListOf(initialStackSymbol)
        return try {
            for (input in inputs) {
                if (input !in inputAlphabet || localStack.isEmpty()) return false
                val transition = transitionIndex[PdaKey(state, input, localStack.last())] ?: return false
                state = applyLocal(transition, localStack)
            }
            state = closeEpsilon(state, localStack).state
            state in accepting
        } catch (_: IllegalStateException) {
            false
        }
    }

    fun reset() {
        currentState = initial
        mutableStack.clear()
        mutableStack += initialStackSymbol
        mutableTrace.clear()
    }

    private fun closeEpsilon() {
        var steps = 0
        while (mutableStack.isNotEmpty()) {
            check(steps++ < EPSILON_STEP_LIMIT) { "PDA epsilon cycle detected" }
            val transition = transitionIndex[PdaKey(currentState, null, mutableStack.last())] ?: return
            apply(transition)
        }
    }

    private fun closeEpsilon(startState: String, startStack: List<String>): Configuration {
        var state = startState
        val localStack = startStack.toMutableList()
        var steps = 0
        while (localStack.isNotEmpty()) {
            check(steps++ < EPSILON_STEP_LIMIT) { "PDA epsilon cycle detected" }
            val transition = transitionIndex[PdaKey(state, null, localStack.last())]
                ?: return Configuration(state, immutableList(localStack))
            state = applyLocal(transition, localStack)
        }
        return Configuration(state, localStack.toList())
    }

    private fun apply(transition: PdaTransition) {
        check(mutableTrace.size < maxTraceEntries) { "PDA trace limit exceeded" }
        val source = currentState
        val popped = mutableStack.last()
        currentState = applyLocal(transition, mutableStack)
        mutableTrace += PdaTraceEntry(
            source,
            transition.event,
            popped,
            currentState,
            transition.stackPush,
            mutableStack,
        )
    }

    private fun applyLocal(transition: PdaTransition, targetStack: MutableList<String>): String {
        check(targetStack.size - 1 + transition.stackPush.size <= maxStackDepth) {
            "PDA stack limit exceeded"
        }
        targetStack.removeAt(targetStack.lastIndex)
        targetStack += transition.stackPush
        return transition.target
    }

    companion object {
        const val EPSILON_STEP_LIMIT = 10_000
        const val DEFAULT_MAX_STACK_DEPTH = 4_096
        const val DEFAULT_MAX_TRACE_ENTRIES = 2_048
    }
}

class ModalStateMachine(
    modes: Map<String, Dfa>,
    modeTransitions: Map<ModeTransitionKey, String>,
    val initialMode: String,
    private val maxTraceEntries: Int = DEFAULT_MAX_TRACE_ENTRIES,
) {
    val modes = immutableMap(modes)
    val modeTransitions = immutableMap(modeTransitions)
    private val mutableModeTrace = mutableListOf<ModeTransitionRecord>()
    val modeTrace: List<ModeTransitionRecord> get() = immutableList(mutableModeTrace)
    var currentMode = initialMode
        private set

    init {
        require(this.modes.isNotEmpty()) { "modal machine requires at least one mode" }
        require(maxTraceEntries >= 0) { "maximum trace entries must be non-negative" }
        require(initialMode in this.modes) { "initial mode is not declared" }
        require(this.modeTransitions.all { (key, target) -> key.mode in this.modes && target in this.modes }) {
            "mode transition references an undeclared mode"
        }
        reset()
    }

    fun process(event: String): String = modes.getValue(currentMode).process(event)

    fun switchMode(trigger: String): String {
        val source = currentMode
        val target = modeTransitions[ModeTransitionKey(source, trigger)]
            ?: throw IllegalArgumentException("unknown mode trigger for current mode")
        check(mutableModeTrace.size < maxTraceEntries) { "modal trace limit exceeded" }
        modes.getValue(target).reset()
        currentMode = target
        mutableModeTrace += ModeTransitionRecord(source, trigger, target)
        return target
    }

    fun reset() {
        modes.values.forEach(Dfa::reset)
        currentMode = initialMode
        mutableModeTrace.clear()
    }

    val activeMachine: Dfa get() = modes.getValue(currentMode)

    companion object {
        const val DEFAULT_MAX_TRACE_ENTRIES = 100_000
    }
}
