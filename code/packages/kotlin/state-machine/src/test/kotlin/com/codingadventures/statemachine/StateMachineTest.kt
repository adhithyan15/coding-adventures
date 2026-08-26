package com.codingadventures.statemachine

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class StateMachineTest {
    @Test
    fun `dfa processes traces acts and preserves state during acceptance queries`() {
        val effects = mutableListOf<String>()
        val machine = Dfa(
            states = setOf("locked", "unlocked"),
            alphabet = setOf("coin", "push"),
            transitions = mapOf(
                TransitionKey("locked", "coin") to "unlocked",
                TransitionKey("locked", "push") to "locked",
                TransitionKey("unlocked", "coin") to "unlocked",
                TransitionKey("unlocked", "push") to "locked",
            ),
            initial = "locked",
            accepting = setOf("unlocked"),
            actions = mapOf(
                TransitionKey("locked", "coin") to TransitionAction("unlock") { source, event, target ->
                    effects += source + event + target
                },
            ),
        )

        assertEquals("unlocked", machine.process("coin"))
        assertEquals(listOf("lockedcoinunlocked"), effects)
        assertEquals(listOf(TransitionRecord("locked", "coin", "unlocked", "unlock")), machine.trace)
        assertFalse(machine.accepts(listOf("push")))
        assertEquals("unlocked", machine.currentState)
        assertEquals(2, machine.processSequence(listOf("push", "coin")).size)
        assertTrue(machine.isAccepting)
        machine.reset()
        assertEquals("locked", machine.currentState)
        assertTrue(machine.trace.isEmpty())
        assertFailsWith<IllegalArgumentException> { machine.process("bogus") }
    }

    @Test
    fun `dfa validates completeness reachability and visualizations`() {
        val incomplete = Dfa(
            states = setOf("start", "accept", "orphan"),
            alphabet = setOf("a", "b"),
            transitions = mapOf(TransitionKey("start", "a") to "accept"),
            initial = "start",
            accepting = setOf("accept"),
        )
        assertEquals(setOf("start", "accept"), incomplete.reachableStates())
        assertFalse(incomplete.isComplete)
        assertTrue(incomplete.validate().any { "orphan" in it })
        assertTrue(incomplete.validate().any { "missing" in it })
        assertEquals(listOf("State", "a", "b"), incomplete.toTable().first())
        assertTrue("start" in incomplete.toAscii())
        assertTrue("doublecircle" in incomplete.toDot())

        assertFailsWith<IllegalArgumentException> { Dfa(emptySet(), setOf("a"), emptyMap(), "x", emptySet()) }
        assertFailsWith<IllegalArgumentException> { Dfa(setOf("x"), setOf("a"), emptyMap(), "y", emptySet()) }
        assertFailsWith<IllegalArgumentException> { Dfa(setOf("x"), setOf("a"), emptyMap(), "x", setOf("y")) }
        assertFailsWith<IllegalArgumentException> {
            Dfa(setOf("x"), setOf("a"), mapOf(TransitionKey("x", "b") to "x"), "x", emptySet())
        }
    }

    @Test
    fun `nfa closes epsilon edges and converts to an equivalent dfa`() {
        val nfa = containsAbNfa()
        assertEquals(setOf("q0", "q1"), nfa.currentStates)
        assertEquals(setOf("q0", "q1"), nfa.epsilonClosure(setOf("q0")))
        assertEquals(setOf("q1", "q2"), nfa.process("a"))
        val expectedTrace = NfaTraceEntry(setOf("q0", "q1"), "a", setOf("q1", "q2"))
        assertEquals(expectedTrace, nfa.trace.first())
        assertEquals(expectedTrace.hashCode(), nfa.trace.first().hashCode())
        assertTrue("NfaTraceEntry" in expectedTrace.toString())
        assertEquals(1, nfa.processSequence(listOf("b")).size)
        assertTrue(nfa.isAccepting)
        assertTrue(nfa.accepts(listOf("a", "b")))
        assertFalse(nfa.accepts(listOf("b", "b")))
        assertTrue("ε" in nfa.toDot())
        val dfa = nfa.toDfa()
        val samples = listOf(emptyList(), listOf("a"), listOf("a", "a"), listOf("a", "b"), listOf("b", "a", "b"))
        for (sample in samples) assertEquals(nfa.accepts(sample), dfa.accepts(sample))
        assertFailsWith<IllegalArgumentException> { nfa.process("z") }
    }

    @Test
    fun `minimization removes unreachable and equivalent states`() {
        val dfa = Dfa(
            states = setOf("a", "b", "c", "dead"),
            alphabet = setOf("0", "1"),
            transitions = mapOf(
                TransitionKey("a", "0") to "b",
                TransitionKey("a", "1") to "c",
                TransitionKey("b", "0") to "b",
                TransitionKey("b", "1") to "b",
                TransitionKey("c", "0") to "b",
                TransitionKey("c", "1") to "b",
                TransitionKey("dead", "0") to "dead",
                TransitionKey("dead", "1") to "dead",
            ),
            initial = "a",
            accepting = setOf("b", "c"),
        )
        val minimized = DfaMinimizer.minimize(dfa)
        assertEquals(2, minimized.states.size)
        val samples = listOf(emptyList(), listOf("0"), listOf("1"), listOf("1", "1"))
        for (sample in samples) assertEquals(dfa.accepts(sample), minimized.accepts(sample))
    }

    @Test
    fun `pda recognizes balanced parentheses and records its stack`() {
        val pda = balancedParenthesesPda()
        assertTrue(pda.accepts(listOf("(", ")")))
        assertTrue(pda.accepts(listOf("(", "(", ")", ")")))
        assertFalse(pda.accepts(listOf("(", "(", ")")))
        assertFalse(pda.accepts(listOf(")")))
        pda.reset()
        assertEquals("scan", pda.process("("))
        assertEquals(listOf("$", "("), pda.stack)
        val completed = pda.processSequence(listOf(")"))
        assertEquals(2, completed.size)
        assertTrue(completed.first().stackPush.isEmpty())
        assertTrue(completed.last().stackAfter.isEmpty())
        val expectedTrace = PdaTraceEntry("scan", null, "$", "accept", emptyList(), emptyList())
        assertEquals(expectedTrace, completed.last())
        assertEquals(expectedTrace.hashCode(), completed.last().hashCode())
        assertTrue("PdaTraceEntry" in expectedTrace.toString())
        assertEquals("accept", pda.currentState)
        assertTrue(pda.stack.isEmpty())
        assertTrue(pda.trace.isNotEmpty())
        assertFailsWith<IllegalStateException> { pda.process("(") }
    }

    @Test
    fun `modal machine switches resets and rejects unknown triggers`() {
        val data = loopingDfa("data", "char")
        val tag = loopingDfa("tag", "name")
        val modal = ModalStateMachine(
            modes = mapOf("DATA" to data, "TAG" to tag),
            modeTransitions = mapOf(
                ModeTransitionKey("DATA", "open") to "TAG",
                ModeTransitionKey("TAG", "close") to "DATA",
            ),
            initialMode = "DATA",
        )
        assertEquals("data", modal.process("char"))
        assertEquals("TAG", modal.switchMode("open"))
        assertEquals("tag", modal.process("name"))
        assertEquals(tag, modal.activeMachine)
        assertEquals(1, modal.modeTrace.size)
        modal.reset()
        assertEquals("DATA", modal.currentMode)
        assertTrue(modal.modeTrace.isEmpty())
        assertFailsWith<IllegalArgumentException> { modal.switchMode("missing") }
        assertFailsWith<IllegalArgumentException> { ModalStateMachine(mapOf("DATA" to data), emptyMap(), "MISSING") }
    }

    @Test
    fun `bounds expansion history and definitions`() {
        val bounded = Dfa(
            states = setOf("s"),
            alphabet = setOf("a"),
            transitions = mapOf(TransitionKey("s", "a") to "s"),
            initial = "s",
            accepting = emptySet(),
            maxTraceEntries = 1,
        )
        bounded.process("a")
        assertFailsWith<IllegalStateException> { bounded.process("a") }
        assertFailsWith<UnsupportedOperationException> { (bounded.states as MutableSet).add("mutated") }
        assertFailsWith<UnsupportedOperationException> {
            (bounded.transitions as MutableMap)[TransitionKey("s", "b")] = "s"
        }

        val collisionNfa = subsetCollisionNfa(16, 16)
        assertEquals(4, collisionNfa.toDfa().states.size)
        assertFailsWith<IllegalStateException> { subsetCollisionNfa(2, 16).toDfa() }
        val traceNfa = subsetCollisionNfa(16, 1)
        traceNfa.process("x")
        assertFailsWith<IllegalStateException> { traceNfa.process("x") }
        assertFailsWith<UnsupportedOperationException> {
            (containsAbNfa().currentStates as MutableSet).add("mutated")
        }
        val traceCellNfa = Nfa(
            states = setOf("a", "b"),
            alphabet = setOf("x"),
            transitions = mapOf(
                TransitionKey("a", "x") to setOf("a", "b"),
                TransitionKey("b", "x") to setOf("a", "b"),
            ),
            initial = "a",
            accepting = emptySet(),
            maxDfaStates = 16,
            maxTraceEntries = 16,
            maxTraceStateCells = 5,
        )
        traceCellNfa.process("x")
        assertFailsWith<IllegalStateException> { traceCellNfa.process("x") }

        val consume = PdaTransition("s", "x", "$", "s", listOf("$"))
        val sameConsume = PdaTransition("s", "x", "$", "s", listOf("$"))
        assertEquals(consume, sameConsume)
        assertEquals(consume.hashCode(), sameConsume.hashCode())
        assertTrue("PdaTransition" in consume.toString())
        val grow = PdaTransition("s", null, "$", "s", listOf("$", "$"))
        val growing = PushdownAutomaton(
            states = setOf("s"),
            inputAlphabet = setOf("x"),
            stackAlphabet = setOf("$"),
            transitions = listOf(consume, grow),
            initial = "s",
            initialStackSymbol = "$",
            accepting = emptySet(),
            maxStackDepth = 4,
            maxTraceEntries = 16,
        )
        assertFailsWith<IllegalStateException> { growing.processSequence(listOf("x")) }

        val first = loopingDfa("first", "stay")
        val second = Dfa(
            states = setOf("fresh", "used"),
            alphabet = setOf("advance"),
            transitions = mapOf(
                TransitionKey("fresh", "advance") to "used",
                TransitionKey("used", "advance") to "used",
            ),
            initial = "fresh",
            accepting = emptySet(),
        )
        val modal = ModalStateMachine(
            modes = mapOf("ONE" to first, "TWO" to second),
            modeTransitions = mapOf(
                ModeTransitionKey("ONE", "next") to "TWO",
                ModeTransitionKey("TWO", "back") to "ONE",
            ),
            initialMode = "ONE",
        )
        modal.switchMode("next")
        modal.process("advance")
        modal.switchMode("back")
        modal.switchMode("next")
        assertEquals("fresh", modal.activeMachine.currentState)
    }

    private fun containsAbNfa() = Nfa(
        states = setOf("q0", "q1", "q2"),
        alphabet = setOf("a", "b"),
        transitions = mapOf(
            TransitionKey("q0", Nfa.EPSILON) to setOf("q1"),
            TransitionKey("q1", "a") to setOf("q1", "q2"),
            TransitionKey("q1", "b") to setOf("q1"),
            TransitionKey("q2", "b") to setOf("q2"),
        ),
        initial = "q0",
        accepting = setOf("q2"),
    )

    private fun subsetCollisionNfa(maxDfaStates: Int, maxTraceEntries: Int) = Nfa(
        states = setOf("s", "a,b", "a", "b"),
        alphabet = setOf("x", "y"),
        transitions = mapOf(
            TransitionKey("s", "x") to setOf("a,b"),
            TransitionKey("s", "y") to setOf("a", "b"),
        ),
        initial = "s",
        accepting = setOf("a,b"),
        maxDfaStates = maxDfaStates,
        maxTraceEntries = maxTraceEntries,
    )

    private fun balancedParenthesesPda() = PushdownAutomaton(
        states = setOf("scan", "accept"),
        inputAlphabet = setOf("(", ")"),
        stackAlphabet = setOf("$", "("),
        transitions = listOf(
            PdaTransition("scan", "(", "$", "scan", listOf("$", "(")),
            PdaTransition("scan", "(", "(", "scan", listOf("(", "(")),
            PdaTransition("scan", ")", "(", "scan", emptyList()),
            PdaTransition("scan", null, "$", "accept", emptyList()),
        ),
        initial = "scan",
        initialStackSymbol = "$",
        accepting = setOf("accept"),
    )

    private fun loopingDfa(state: String, event: String) = Dfa(
        states = setOf(state),
        alphabet = setOf(event),
        transitions = mapOf(TransitionKey(state, event) to state),
        initial = state,
        accepting = setOf(state),
    )
}
