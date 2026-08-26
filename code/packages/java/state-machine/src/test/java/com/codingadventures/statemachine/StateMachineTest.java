package com.codingadventures.statemachine;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Set;
import org.junit.jupiter.api.Test;

final class StateMachineTest {
    @Test
    void dfaProcessesTracesActsAndPreservesStateDuringAcceptanceQueries() {
        List<String> effects = new ArrayList<>();
        Dfa machine = new Dfa(
                Set.of("locked", "unlocked"),
                Set.of("coin", "push"),
                Map.of(
                        new TransitionKey("locked", "coin"), "unlocked",
                        new TransitionKey("locked", "push"), "locked",
                        new TransitionKey("unlocked", "coin"), "unlocked",
                        new TransitionKey("unlocked", "push"), "locked"),
                "locked",
                Set.of("unlocked"),
                Map.of(
                        new TransitionKey("locked", "coin"),
                        new TransitionAction("unlock", (source, event, target) -> effects.add(source + event + target))));

        assertEquals("unlocked", machine.process("coin"));
        assertEquals(List.of("lockedcoinunlocked"), effects);
        assertEquals(
                List.of(new TransitionRecord("locked", "coin", "unlocked", "unlock")),
                machine.trace());
        assertFalse(machine.accepts(List.of("push")));
        assertEquals("unlocked", machine.currentState(), "accepts runs on an isolated copy");
        assertEquals(2, machine.processSequence(List.of("push", "coin")).size());
        assertTrue(machine.isAccepting());
        machine.reset();
        assertEquals("locked", machine.currentState());
        assertTrue(machine.trace().isEmpty());
        assertThrows(IllegalArgumentException.class, () -> machine.process("bogus"));
    }

    @Test
    void dfaValidatesCompletenessReachabilityAndVisualizations() {
        Dfa incomplete = new Dfa(
                Set.of("start", "accept", "orphan"),
                Set.of("a", "b"),
                Map.of(new TransitionKey("start", "a"), "accept"),
                "start",
                Set.of("accept"));

        assertEquals(Set.of("start", "accept"), incomplete.reachableStates());
        assertFalse(incomplete.isComplete());
        assertTrue(incomplete.validate().stream().anyMatch(message -> message.contains("orphan")));
        assertTrue(incomplete.validate().stream().anyMatch(message -> message.contains("missing")));
        assertEquals(List.of("State", "a", "b"), incomplete.toTable().get(0));
        assertTrue(incomplete.toAscii().contains("start"));
        assertTrue(incomplete.toDot().contains("doublecircle"));

        assertThrows(
                IllegalArgumentException.class,
                () -> new Dfa(Set.of(), Set.of("a"), Map.of(), "x", Set.of()));
        assertThrows(
                IllegalArgumentException.class,
                () -> new Dfa(Set.of("x"), Set.of("a"), Map.of(), "y", Set.of()));
        assertThrows(
                IllegalArgumentException.class,
                () -> new Dfa(Set.of("x"), Set.of("a"), Map.of(), "x", Set.of("y")));
        assertThrows(
                IllegalArgumentException.class,
                () -> new Dfa(
                        Set.of("x"),
                        Set.of("a"),
                        Map.of(new TransitionKey("x", "b"), "x"),
                        "x",
                        Set.of()));
    }

    @Test
    void nfaClosesEpsilonEdgesAndConvertsToAnEquivalentDfa() {
        Nfa nfa = containsAbNfa();
        assertEquals(Set.of("q0", "q1"), nfa.currentStates());
        assertEquals(Set.of("q0", "q1"), nfa.epsilonClosure(Set.of("q0")));
        assertEquals(Set.of("q1", "q2"), nfa.process("a"));
        assertEquals(1, nfa.processSequence(List.of("b")).size());
        assertTrue(nfa.isAccepting());
        assertTrue(nfa.accepts(List.of("a", "b")));
        assertFalse(nfa.accepts(List.of("b", "b")));
        assertTrue(nfa.toDot().contains("ε"));

        Dfa dfa = nfa.toDfa();
        for (List<String> sample : List.of(
                List.<String>of(),
                List.of("a"),
                List.of("a", "a"),
                List.of("a", "b"),
                List.of("b", "a", "b"))) {
            assertEquals(nfa.accepts(sample), dfa.accepts(sample));
        }
        assertThrows(IllegalArgumentException.class, () -> nfa.process("z"));
    }

    @Test
    void minimizationRemovesUnreachableAndEquivalentStates() {
        Dfa dfa = new Dfa(
                Set.of("a", "b", "c", "dead"),
                Set.of("0", "1"),
                Map.of(
                        new TransitionKey("a", "0"), "b",
                        new TransitionKey("a", "1"), "c",
                        new TransitionKey("b", "0"), "b",
                        new TransitionKey("b", "1"), "b",
                        new TransitionKey("c", "0"), "b",
                        new TransitionKey("c", "1"), "b",
                        new TransitionKey("dead", "0"), "dead",
                        new TransitionKey("dead", "1"), "dead"),
                "a",
                Set.of("b", "c"));

        Dfa minimized = DfaMinimizer.minimize(dfa);
        assertEquals(2, minimized.states().size());
        for (List<String> sample : List.of(List.<String>of(), List.of("0"), List.of("1"), List.of("1", "1"))) {
            assertEquals(dfa.accepts(sample), minimized.accepts(sample));
        }
    }

    @Test
    void pdaRecognizesBalancedParenthesesAndRecordsItsStack() {
        PushdownAutomaton pda = balancedParenthesesPda();
        assertTrue(pda.accepts(List.of("(", ")")));
        assertTrue(pda.accepts(List.of("(", "(", ")", ")")));
        assertFalse(pda.accepts(List.of("(", "(", ")")));
        assertFalse(pda.accepts(List.of(")")));

        pda.reset();
        assertEquals("scan", pda.process("("));
        assertEquals(List.of("$", "("), pda.stack());
        List<PdaTraceEntry> completed = pda.processSequence(List.of(")"));
        assertEquals(2, completed.size());
        assertEquals(List.of(), completed.get(0).stackPush());
        assertEquals(List.of(), completed.get(1).stackAfter());
        assertEquals("accept", pda.currentState());
        assertEquals(List.of(), pda.stack());
        assertFalse(pda.trace().isEmpty());
        assertThrows(IllegalStateException.class, () -> pda.process("("));
    }

    @Test
    void modalMachineSwitchesResetsAndRejectsUnknownTriggers() {
        Dfa data = loopingDfa("data", "char");
        Dfa tag = loopingDfa("tag", "name");
        ModalStateMachine modal = new ModalStateMachine(
                Map.of("DATA", data, "TAG", tag),
                Map.of(
                        new ModeTransitionKey("DATA", "open"), "TAG",
                        new ModeTransitionKey("TAG", "close"), "DATA"),
                "DATA");

        assertEquals("data", modal.process("char"));
        assertEquals("TAG", modal.switchMode("open"));
        assertEquals("tag", modal.process("name"));
        assertEquals(tag, modal.activeMachine());
        assertEquals(1, modal.modeTrace().size());
        modal.reset();
        assertEquals("DATA", modal.currentMode());
        assertTrue(modal.modeTrace().isEmpty());
        assertThrows(IllegalArgumentException.class, () -> modal.switchMode("missing"));
        assertThrows(
                IllegalArgumentException.class,
                () -> new ModalStateMachine(Map.of("DATA", data), Map.of(), "MISSING"));
    }

    @Test
    void rejectsInvalidDefinitionsAndExercisesFailureBoundaries() {
        assertThrows(IllegalArgumentException.class, () -> new TransitionKey(null, "x"));
        assertThrows(IllegalArgumentException.class, () -> new ModeTransitionKey("m", null));
        assertThrows(IllegalArgumentException.class, () -> new TransitionAction("", (a, b, c) -> {}));
        assertThrows(
                IllegalArgumentException.class,
                () -> new PdaTransition("s", null, "$", "s", null));

        Dfa incomplete = new Dfa(
                Set.of("s"), Set.of("a", "b"), Map.of(new TransitionKey("s", "a"), "s"), "s", Set.of());
        assertTrue(loopingDfa("s", "a").isComplete());
        assertFalse(incomplete.accepts(List.of("b")));
        assertFalse(incomplete.accepts(List.of("z")));
        assertThrows(IllegalStateException.class, () -> incomplete.process("b"));
        assertThrows(
                IllegalArgumentException.class,
                () -> new Dfa(
                        Set.of("s"),
                        Set.of("a"),
                        Map.of(new TransitionKey("s", "a"), "s"),
                        "s",
                        Set.of(),
                        Map.of(new TransitionKey("s", "b"), new TransitionAction("bad", (a, b, c) -> {}))));
        assertFalse(DfaMinimizer.minimize(incomplete).isComplete());
        Dfa traceBounded = new Dfa(
                Set.of("s"),
                Set.of("a"),
                Map.of(new TransitionKey("s", "a"), "s"),
                "s",
                Set.of(),
                Map.of(),
                1);
        traceBounded.process("a");
        assertThrows(IllegalStateException.class, () -> traceBounded.process("a"));

        assertThrows(
                IllegalArgumentException.class,
                () -> new Nfa(Set.of(), Set.of("a"), Map.of(), "s", Set.of()));
        assertThrows(
                IllegalArgumentException.class,
                () -> new Nfa(Set.of("s"), Set.of(Nfa.EPSILON), Map.of(), "s", Set.of()));
        assertThrows(
                IllegalArgumentException.class,
                () -> new Nfa(Set.of("s"), Set.of("a"), Map.of(), "x", Set.of()));
        assertThrows(
                IllegalArgumentException.class,
                () -> new Nfa(
                        Set.of("s"),
                        Set.of("a"),
                        Map.of(new TransitionKey("s", "b"), Set.of("s")),
                        "s",
                        Set.of()));
        Nfa nfa = containsAbNfa();
        assertThrows(IllegalArgumentException.class, () -> nfa.epsilonClosure(Set.of("missing")));
        assertFalse(nfa.accepts(List.of("z")));
        nfa.process("a");
        nfa.reset();
        assertTrue(nfa.trace().isEmpty());
        Nfa collisionNfa = subsetCollisionNfa(16, 16);
        assertEquals(4, collisionNfa.toDfa().states().size());
        assertThrows(IllegalStateException.class, () -> subsetCollisionNfa(2, 16).toDfa());
        Nfa boundedTraceNfa = subsetCollisionNfa(16, 1);
        boundedTraceNfa.process("x");
        assertThrows(IllegalStateException.class, () -> boundedTraceNfa.process("x"));
        Nfa boundedTraceCells = new Nfa(
                Set.of("a", "b"),
                Set.of("x"),
                Map.of(
                        new TransitionKey("a", "x"), Set.of("a", "b"),
                        new TransitionKey("b", "x"), Set.of("a", "b")),
                "a",
                Set.of(),
                16,
                16,
                5);
        boundedTraceCells.process("x");
        assertThrows(IllegalStateException.class, () -> boundedTraceCells.process("x"));

        assertThrows(
                IllegalArgumentException.class,
                () -> new PushdownAutomaton(Set.of(), Set.of(), Set.of("$"), List.of(), "s", "$", Set.of()));
        assertThrows(
                IllegalArgumentException.class,
                () -> new PushdownAutomaton(Set.of("s"), Set.of(), Set.of("$"), List.of(), "x", "$", Set.of()));
        PdaTransition duplicate = new PdaTransition("s", "x", "$", "s", List.of("$"));
        assertThrows(
                IllegalArgumentException.class,
                () -> new PushdownAutomaton(
                        Set.of("s"), Set.of("x"), Set.of("$"), List.of(duplicate, duplicate), "s", "$", Set.of()));
        assertThrows(
                IllegalArgumentException.class,
                () -> new PushdownAutomaton(
                        Set.of("s"),
                        Set.of("x"),
                        Set.of("$"),
                        List.of(new PdaTransition("s", "bad", "$", "s", List.of("$"))),
                        "s",
                        "$",
                        Set.of()));
        PushdownAutomaton pda = balancedParenthesesPda();
        assertThrows(IllegalArgumentException.class, () -> pda.process("bad"));
        assertThrows(IllegalStateException.class, () -> pda.process(")"));

        PdaTransition consume = new PdaTransition("s", "x", "$", "s", List.of("$"));
        PdaTransition epsilonCycle = new PdaTransition("s", null, "$", "s", List.of("$"));
        PushdownAutomaton cyclic = new PushdownAutomaton(
                Set.of("s"), Set.of("x"), Set.of("$"), List.of(consume, epsilonCycle), "s", "$", Set.of());
        assertFalse(cyclic.accepts(List.of()));
        assertThrows(IllegalStateException.class, () -> cyclic.processSequence(List.of("x")));

        PdaTransition grow = new PdaTransition("s", null, "$", "s", List.of("$", "$"));
        PushdownAutomaton growing = new PushdownAutomaton(
                Set.of("s"),
                Set.of("x"),
                Set.of("$"),
                List.of(consume, grow),
                "s",
                "$",
                Set.of(),
                4,
                16);
        assertThrows(IllegalStateException.class, () -> growing.processSequence(List.of("x")));

        assertThrows(IllegalArgumentException.class, () -> new ModalStateMachine(Map.of(), Map.of(), "x"));
        assertThrows(
                IllegalArgumentException.class,
                () -> new ModalStateMachine(
                        Map.of("m", loopingDfa("s", "x")),
                        Map.of(new ModeTransitionKey("m", "go"), "missing"),
                        "m"));
        Dfa first = loopingDfa("first", "stay");
        Dfa second = new Dfa(
                Set.of("fresh", "used"),
                Set.of("advance"),
                Map.of(
                        new TransitionKey("fresh", "advance"), "used",
                        new TransitionKey("used", "advance"), "used"),
                "fresh",
                Set.of());
        ModalStateMachine resetting = new ModalStateMachine(
                Map.of("ONE", first, "TWO", second),
                Map.of(
                        new ModeTransitionKey("ONE", "next"), "TWO",
                        new ModeTransitionKey("TWO", "back"), "ONE"),
                "ONE");
        resetting.switchMode("next");
        resetting.process("advance");
        resetting.switchMode("back");
        resetting.switchMode("next");
        assertEquals("fresh", resetting.activeMachine().currentState());
    }

    private static Nfa subsetCollisionNfa(int maxDfaStates, int maxTraceEntries) {
        return new Nfa(
                Set.of("s", "a,b", "a", "b"),
                Set.of("x", "y"),
                Map.of(
                        new TransitionKey("s", "x"), Set.of("a,b"),
                        new TransitionKey("s", "y"), Set.of("a", "b")),
                "s",
                Set.of("a,b"),
                maxDfaStates,
                maxTraceEntries);
    }

    private static Nfa containsAbNfa() {
        return new Nfa(
                Set.of("q0", "q1", "q2"),
                Set.of("a", "b"),
                Map.of(
                        new TransitionKey("q0", Nfa.EPSILON), Set.of("q1"),
                        new TransitionKey("q1", "a"), Set.of("q1", "q2"),
                        new TransitionKey("q1", "b"), Set.of("q1"),
                        new TransitionKey("q2", "b"), Set.of("q2")),
                "q0",
                Set.of("q2"));
    }

    private static PushdownAutomaton balancedParenthesesPda() {
        return new PushdownAutomaton(
                Set.of("scan", "accept"),
                Set.of("(", ")"),
                Set.of("$", "("),
                List.of(
                        new PdaTransition("scan", "(", "$", "scan", List.of("$", "(")),
                        new PdaTransition("scan", "(", "(", "scan", List.of("(", "(")),
                        new PdaTransition("scan", ")", "(", "scan", List.of()),
                        new PdaTransition("scan", null, "$", "accept", List.of())),
                "scan",
                "$",
                Set.of("accept"));
    }

    private static Dfa loopingDfa(String state, String event) {
        return new Dfa(
                Set.of(state),
                Set.of(event),
                Map.of(new TransitionKey(state, event), state),
                state,
                Set.of(state));
    }
}
