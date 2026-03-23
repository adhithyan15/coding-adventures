-- Tests for state-machine — comprehensive busted tests covering DFA, NFA, PDA,
-- ModalStateMachine, and Minimize.
--
-- These tests port the Go test suite and add additional coverage for
-- Lua-specific behavior. Target: 95%+ coverage.

-- Add src/ and directed_graph to the module search path.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" ..
    "../../directed_graph/src/?.lua;" .. "../../directed_graph/src/?/init.lua;" ..
    package.path

local sm = require("coding_adventures.state_machine")
local DFA = sm.DFA
local NFA = sm.NFA
local PDA = sm.PDA
local ModalStateMachine = sm.ModalStateMachine
local Minimize = sm.Minimize
local EPSILON = sm.EPSILON

-- =========================================================================
-- Helper: build a turnstile DFA (the canonical example)
-- =========================================================================
--
-- A turnstile has two states: locked and unlocked.
-- Insert a coin -> it unlocks. Push the arm -> it locks.
--
--   (locked) --coin--> (unlocked) --push--> (locked)
--   (locked) --push--> (locked)
--   (unlocked) --coin--> (unlocked)

local function make_turnstile(actions)
    return DFA.new(
        {"locked", "unlocked"},
        {"coin", "push"},
        {
            {"locked", "coin"}, "unlocked",
            {"locked", "push"}, "locked",
            {"unlocked", "coin"}, "unlocked",
            {"unlocked", "push"}, "locked",
        },
        "locked",
        {"unlocked"},
        actions
    )
end

-- Helper to convert the array-based transition format to a proper table.
-- The format used above is: key1, val1, key2, val2, ...
-- This helper converts it to a Lua table {[key1]=val1, [key2]=val2, ...}.
local function make_transitions(...)
    local args = {...}
    local t = {}
    for i = 1, #args, 2 do
        t[args[i]] = args[i + 1]
    end
    return t
end

-- Rebuild the turnstile helper with proper table format
local function build_turnstile(actions)
    return DFA.new(
        {"locked", "unlocked"},
        {"coin", "push"},
        make_transitions(
            {"locked", "coin"}, "unlocked",
            {"locked", "push"}, "locked",
            {"unlocked", "coin"}, "unlocked",
            {"unlocked", "push"}, "locked"
        ),
        "locked",
        {"unlocked"},
        actions
    )
end


-- =========================================================================
-- DFA Tests
-- =========================================================================

describe("DFA", function()

    describe("construction", function()

        it("creates a valid DFA with all parameters", function()
            local dfa = build_turnstile()
            assert.are.same({"locked", "unlocked"}, dfa:states())
            assert.are.same({"coin", "push"}, dfa:alphabet())
            assert.are.equal("locked", dfa:initial())
            assert.are.same({"unlocked"}, dfa:accepting())
            assert.are.equal("locked", dfa:current_state())
        end)

        it("errors on empty states", function()
            assert.has_error(function()
                DFA.new({}, {"a"}, {}, "q0", {}, nil)
            end, "statemachine: states set must be non-empty")
        end)

        it("errors on invalid initial state", function()
            assert.has_error(function()
                DFA.new({"q0"}, {"a"}, {}, "q1", {}, nil)
            end)
        end)

        it("errors on invalid accepting state", function()
            assert.has_error(function()
                DFA.new({"q0"}, {"a"}, {}, "q0", {"q1"}, nil)
            end)
        end)

        it("errors on transition with unknown source state", function()
            assert.has_error(function()
                DFA.new(
                    {"q0"}, {"a"},
                    make_transitions({"unknown", "a"}, "q0"),
                    "q0", {}, nil
                )
            end)
        end)

        it("errors on transition with unknown event", function()
            assert.has_error(function()
                DFA.new(
                    {"q0"}, {"a"},
                    make_transitions({"q0", "b"}, "q0"),
                    "q0", {}, nil
                )
            end)
        end)

        it("errors on transition with unknown target state", function()
            assert.has_error(function()
                DFA.new(
                    {"q0"}, {"a"},
                    make_transitions({"q0", "a"}, "q1"),
                    "q0", {}, nil
                )
            end)
        end)

        it("errors on action without matching transition", function()
            assert.has_error(function()
                DFA.new(
                    {"q0"}, {"a"},
                    make_transitions({"q0", "a"}, "q0"),
                    "q0", {},
                    make_transitions({"q0", "b"}, function() end)
                )
            end)
        end)

    end)

    describe("process", function()

        it("transitions correctly on valid events", function()
            local dfa = build_turnstile()
            assert.are.equal("unlocked", dfa:process("coin"))
            assert.are.equal("unlocked", dfa:current_state())
            assert.are.equal("locked", dfa:process("push"))
            assert.are.equal("locked", dfa:current_state())
        end)

        it("errors on unknown event", function()
            local dfa = build_turnstile()
            assert.has_error(function()
                dfa:process("kick")
            end)
        end)

        it("errors on missing transition", function()
            -- DFA with a missing transition
            local dfa = DFA.new(
                {"q0", "q1"}, {"a", "b"},
                make_transitions({"q0", "a"}, "q1"),
                "q0", {"q1"}, nil
            )
            assert.has_error(function()
                dfa:process("b")
            end)
        end)

        it("records trace correctly", function()
            local dfa = build_turnstile()
            dfa:process("coin")
            dfa:process("push")
            local trace = dfa:trace()
            assert.are.equal(2, #trace)
            assert.are.equal("locked", trace[1].source)
            assert.are.equal("coin", trace[1].event)
            assert.are.equal("unlocked", trace[1].target)
            assert.are.equal("unlocked", trace[2].source)
            assert.are.equal("push", trace[2].event)
            assert.are.equal("locked", trace[2].target)
        end)

        it("executes actions on transitions", function()
            local log = {}
            local actions = make_transitions(
                {"locked", "coin"}, function(s, e, t)
                    log[#log + 1] = s .. " -> " .. t
                end
            )
            local dfa = build_turnstile(actions)
            dfa:process("coin")
            assert.are.equal(1, #log)
            assert.are.equal("locked -> unlocked", log[1])

            -- Verify action_name is recorded
            local trace = dfa:trace()
            assert.are.equal("action", trace[1].action_name)
        end)

    end)

    describe("process_sequence", function()

        it("processes multiple events in order", function()
            local dfa = build_turnstile()
            local new_trace = dfa:process_sequence({"coin", "push", "coin"})
            assert.are.equal(3, #new_trace)
            assert.are.equal("unlocked", dfa:current_state())
        end)

        it("returns only new trace entries", function()
            local dfa = build_turnstile()
            dfa:process("coin")  -- 1 existing trace entry
            local new_trace = dfa:process_sequence({"push", "coin"})
            assert.are.equal(2, #new_trace)
            assert.are.equal(3, #dfa:trace())
        end)

    end)

    describe("accepts", function()

        it("accepts sequences ending in accepting state", function()
            local dfa = build_turnstile()
            assert.is_true(dfa:accepts({"coin"}))
            assert.is_true(dfa:accepts({"coin", "push", "coin"}))
        end)

        it("rejects sequences ending in non-accepting state", function()
            local dfa = build_turnstile()
            assert.is_false(dfa:accepts({}))  -- starts locked
            assert.is_false(dfa:accepts({"push"}))
            assert.is_false(dfa:accepts({"coin", "push"}))
        end)

        it("returns false on missing transition (does not error)", function()
            local dfa = DFA.new(
                {"q0", "q1"}, {"a", "b"},
                make_transitions({"q0", "a"}, "q1"),
                "q0", {"q1"}, nil
            )
            -- "b" from q0 has no transition — should return false, not error
            assert.is_false(dfa:accepts({"b"}))
        end)

        it("does not modify current state or trace", function()
            local dfa = build_turnstile()
            dfa:accepts({"coin", "push", "coin"})
            assert.are.equal("locked", dfa:current_state())
            assert.are.equal(0, #dfa:trace())
        end)

        it("errors on unknown event in accepts", function()
            local dfa = build_turnstile()
            assert.has_error(function()
                dfa:accepts({"kick"})
            end)
        end)

    end)

    describe("reset", function()

        it("returns to initial state and clears trace", function()
            local dfa = build_turnstile()
            dfa:process("coin")
            dfa:process("push")
            assert.are.equal(2, #dfa:trace())

            dfa:reset()
            assert.are.equal("locked", dfa:current_state())
            assert.are.equal(0, #dfa:trace())
        end)

    end)

    describe("transitions getter", function()

        it("returns all transitions as sorted arrays", function()
            local dfa = build_turnstile()
            local trans = dfa:transitions()
            assert.are.equal(4, #trans)
            -- First should be locked/coin -> unlocked
            assert.are.equal("locked", trans[1][1])
            assert.are.equal("coin", trans[1][2])
            assert.are.equal("unlocked", trans[1][3])
        end)

    end)

    describe("reachable_states", function()

        it("finds all reachable states from initial", function()
            local dfa = build_turnstile()
            local reachable = dfa:reachable_states()
            assert.is_true(reachable["locked"])
            assert.is_true(reachable["unlocked"])
        end)

        it("detects unreachable states", function()
            local dfa = DFA.new(
                {"q0", "q1", "orphan"}, {"a"},
                make_transitions(
                    {"q0", "a"}, "q1",
                    {"q1", "a"}, "q0",
                    {"orphan", "a"}, "orphan"
                ),
                "q0", {}, nil
            )
            local reachable = dfa:reachable_states()
            assert.is_true(reachable["q0"])
            assert.is_true(reachable["q1"])
            assert.is_nil(reachable["orphan"])
        end)

    end)

    describe("is_complete", function()

        it("returns true for complete DFA", function()
            local dfa = build_turnstile()
            assert.is_true(dfa:is_complete())
        end)

        it("returns false for incomplete DFA", function()
            local dfa = DFA.new(
                {"q0", "q1"}, {"a", "b"},
                make_transitions({"q0", "a"}, "q1"),
                "q0", {}, nil
            )
            assert.is_false(dfa:is_complete())
        end)

    end)

    describe("validate", function()

        it("returns empty for a valid complete DFA", function()
            local dfa = build_turnstile()
            local warnings = dfa:validate()
            assert.are.equal(0, #warnings)
        end)

        it("warns about unreachable states", function()
            local dfa = DFA.new(
                {"q0", "q1", "orphan"}, {"a"},
                make_transitions(
                    {"q0", "a"}, "q1",
                    {"q1", "a"}, "q0",
                    {"orphan", "a"}, "orphan"
                ),
                "q0", {}, nil
            )
            local warnings = dfa:validate()
            assert.is_true(#warnings > 0)
            -- Should mention "orphan"
            local found = false
            for _, w in ipairs(warnings) do
                if w:find("orphan") then found = true end
            end
            assert.is_true(found)
        end)

        it("warns about missing transitions", function()
            local dfa = DFA.new(
                {"q0", "q1"}, {"a", "b"},
                make_transitions({"q0", "a"}, "q1"),
                "q0", {}, nil
            )
            local warnings = dfa:validate()
            local found_missing = false
            for _, w in ipairs(warnings) do
                if w:find("Missing transitions") then found_missing = true end
            end
            assert.is_true(found_missing)
        end)

        it("warns about unreachable accepting states", function()
            local dfa = DFA.new(
                {"q0", "orphan"}, {"a"},
                make_transitions(
                    {"q0", "a"}, "q0",
                    {"orphan", "a"}, "orphan"
                ),
                "q0", {"orphan"}, nil
            )
            local warnings = dfa:validate()
            local found = false
            for _, w in ipairs(warnings) do
                if w:find("Unreachable accepting") then found = true end
            end
            assert.is_true(found)
        end)

    end)

    describe("to_dot", function()

        it("produces valid DOT output", function()
            local dfa = build_turnstile()
            local dot = dfa:to_dot()
            assert.is_truthy(dot:find("digraph DFA"))
            assert.is_truthy(dot:find("__start"))
            assert.is_truthy(dot:find("doublecircle"))
            assert.is_truthy(dot:find("rankdir=LR"))
        end)

    end)

    describe("to_ascii", function()

        it("produces formatted ASCII table", function()
            local dfa = build_turnstile()
            local ascii = dfa:to_ascii()
            assert.is_truthy(ascii:find("coin"))
            assert.is_truthy(ascii:find("push"))
            assert.is_truthy(ascii:find("locked"))
            assert.is_truthy(ascii:find("unlocked"))
            -- Check markers
            assert.is_truthy(ascii:find(">"))
            assert.is_truthy(ascii:find("*"))
        end)

    end)

    describe("to_table", function()

        it("returns header and data rows", function()
            local dfa = build_turnstile()
            local tbl = dfa:to_table()
            assert.are.equal(3, #tbl)  -- header + 2 states
            assert.are.equal("State", tbl[1][1])
            assert.are.equal("coin", tbl[1][2])
            assert.are.equal("push", tbl[1][3])
        end)

        it("uses em-dash for missing transitions", function()
            local dfa = DFA.new(
                {"q0", "q1"}, {"a", "b"},
                make_transitions({"q0", "a"}, "q1"),
                "q0", {}, nil
            )
            local tbl = dfa:to_table()
            -- q0 row: a -> q1, b -> em-dash
            local q0_row = tbl[2]
            assert.are.equal("q0", q0_row[1])
            assert.are.equal("q1", q0_row[2])  -- a -> q1
            assert.are.equal("\u{2014}", q0_row[3])  -- b -> em-dash
        end)

    end)

end)


-- =========================================================================
-- NFA Tests
-- =========================================================================

describe("NFA", function()

    -- Build an NFA that accepts strings containing "ab"
    local function build_ab_nfa()
        return NFA.new(
            {"q0", "q1", "q2"},
            {"a", "b"},
            make_transitions(
                {"q0", "a"}, {"q0", "q1"},
                {"q0", "b"}, {"q0"},
                {"q1", "b"}, {"q2"},
                {"q2", "a"}, {"q2"},
                {"q2", "b"}, {"q2"}
            ),
            "q0",
            {"q2"}
        )
    end

    describe("construction", function()

        it("creates a valid NFA", function()
            local nfa = build_ab_nfa()
            assert.are.same({"q0", "q1", "q2"}, nfa:states())
            assert.are.same({"a", "b"}, nfa:alphabet())
            assert.are.equal("q0", nfa:initial())
            assert.are.same({"q2"}, nfa:accepting())
        end)

        it("errors on empty states", function()
            assert.has_error(function()
                NFA.new({}, {"a"}, {}, "q0", {})
            end)
        end)

        it("errors on epsilon in alphabet", function()
            assert.has_error(function()
                NFA.new({"q0"}, {""}, {}, "q0", {})
            end)
        end)

        it("errors on invalid initial state", function()
            assert.has_error(function()
                NFA.new({"q0"}, {"a"}, {}, "q1", {})
            end)
        end)

        it("errors on invalid accepting state", function()
            assert.has_error(function()
                NFA.new({"q0"}, {"a"}, {}, "q0", {"q1"})
            end)
        end)

        it("errors on transition with unknown source", function()
            assert.has_error(function()
                NFA.new(
                    {"q0"}, {"a"},
                    make_transitions({"q1", "a"}, {"q0"}),
                    "q0", {}
                )
            end)
        end)

        it("errors on transition with unknown event", function()
            assert.has_error(function()
                NFA.new(
                    {"q0"}, {"a"},
                    make_transitions({"q0", "b"}, {"q0"}),
                    "q0", {}
                )
            end)
        end)

        it("errors on transition with unknown target", function()
            assert.has_error(function()
                NFA.new(
                    {"q0"}, {"a"},
                    make_transitions({"q0", "a"}, {"q1"}),
                    "q0", {}
                )
            end)
        end)

    end)

    describe("epsilon closure", function()

        it("computes closure with epsilon transitions", function()
            local nfa = NFA.new(
                {"q0", "q1", "q2"},
                {"a"},
                make_transitions(
                    {"q0", ""}, {"q1"},
                    {"q1", ""}, {"q2"},
                    {"q2", "a"}, {"q2"}
                ),
                "q0",
                {"q2"}
            )
            -- Initial state should include epsilon closure
            local current = nfa:current_states()
            assert.is_true(current["q0"])
            assert.is_true(current["q1"])
            assert.is_true(current["q2"])
        end)

        it("handles empty epsilon closure", function()
            local nfa = NFA.new(
                {"q0", "q1"},
                {"a"},
                make_transitions(
                    {"q0", "a"}, {"q1"}
                ),
                "q0",
                {"q1"}
            )
            -- No epsilon transitions — closure of {q0} is just {q0}
            local closure = nfa:epsilon_closure({ q0 = true })
            assert.is_true(closure["q0"])
            assert.is_nil(closure["q1"])
        end)

    end)

    describe("process", function()

        it("transitions correctly through NFA", function()
            local nfa = build_ab_nfa()
            local states = nfa:process("a")
            assert.is_true(states["q0"])
            assert.is_true(states["q1"])
        end)

        it("errors on unknown event", function()
            local nfa = build_ab_nfa()
            assert.has_error(function()
                nfa:process("c")
            end)
        end)

    end)

    describe("accepts", function()

        it("accepts strings containing 'ab'", function()
            local nfa = build_ab_nfa()
            assert.is_true(nfa:accepts({"a", "b"}))
            assert.is_true(nfa:accepts({"a", "a", "b"}))
            assert.is_true(nfa:accepts({"b", "a", "b"}))
            assert.is_true(nfa:accepts({"a", "b", "a", "b"}))
        end)

        it("rejects strings not containing 'ab'", function()
            local nfa = build_ab_nfa()
            assert.is_false(nfa:accepts({}))
            assert.is_false(nfa:accepts({"a"}))
            assert.is_false(nfa:accepts({"b"}))
            assert.is_false(nfa:accepts({"b", "a"}))
            assert.is_false(nfa:accepts({"b", "b", "a"}))
        end)

        it("does not modify current state", function()
            local nfa = build_ab_nfa()
            nfa:accepts({"a", "b"})
            local current = nfa:current_states()
            -- Should still be at initial state
            assert.is_true(current["q0"])
        end)

        it("errors on unknown event in accepts", function()
            local nfa = build_ab_nfa()
            assert.has_error(function()
                nfa:accepts({"c"})
            end)
        end)

        it("returns false when NFA dies (no active states)", function()
            -- NFA where all paths die after certain inputs
            local nfa = NFA.new(
                {"q0", "q1"},
                {"a", "b"},
                make_transitions(
                    {"q0", "a"}, {"q1"}
                    -- no transitions from q1 and no transition on "b" from q0
                ),
                "q0",
                {"q1"}
            )
            assert.is_false(nfa:accepts({"a", "b"}))
        end)

    end)

    describe("reset", function()

        it("returns to initial epsilon closure", function()
            local nfa = build_ab_nfa()
            nfa:process("a")
            nfa:process("b")
            nfa:reset()
            local current = nfa:current_states()
            assert.is_true(current["q0"])
        end)

    end)

    describe("to_dfa", function()

        it("converts NFA to equivalent DFA", function()
            local nfa = build_ab_nfa()
            local dfa = nfa:to_dfa()

            -- The converted DFA should accept the same language
            assert.is_true(dfa:accepts({"a", "b"}))
            assert.is_true(dfa:accepts({"a", "a", "b"}))
            assert.is_false(dfa:accepts({}))
            assert.is_false(dfa:accepts({"a"}))
            assert.is_false(dfa:accepts({"b", "a"}))
        end)

        it("handles epsilon transitions in conversion", function()
            local nfa = NFA.new(
                {"q0", "q1", "q2"},
                {"a"},
                make_transitions(
                    {"q0", ""}, {"q1"},
                    {"q1", "a"}, {"q2"}
                ),
                "q0",
                {"q2"}
            )
            local dfa = nfa:to_dfa()
            assert.is_true(dfa:accepts({"a"}))
            assert.is_false(dfa:accepts({}))
        end)

    end)

    describe("to_dot", function()

        it("produces valid DOT output", function()
            local nfa = build_ab_nfa()
            local dot = nfa:to_dot()
            assert.is_truthy(dot:find("digraph NFA"))
            assert.is_truthy(dot:find("__start"))
        end)

        it("uses epsilon character for epsilon transitions", function()
            local nfa = NFA.new(
                {"q0", "q1"},
                {"a"},
                make_transitions(
                    {"q0", ""}, {"q1"},
                    {"q1", "a"}, {"q1"}
                ),
                "q0",
                {"q1"}
            )
            local dot = nfa:to_dot()
            -- Should contain the epsilon unicode character
            assert.is_truthy(dot:find("\u{03b5}"))
        end)

    end)

end)


-- =========================================================================
-- PDA Tests
-- =========================================================================

describe("PDA", function()

    -- Build a PDA for balanced parentheses
    local function build_paren_pda()
        return PDA.new(
            {"q0", "accept"},
            {"(", ")"},
            {"(", "$"},
            {
                { source = "q0", event = "(", stack_read = "$",
                  target = "q0", stack_push = {"$", "("} },
                { source = "q0", event = "(", stack_read = "(",
                  target = "q0", stack_push = {"(", "("} },
                { source = "q0", event = ")", stack_read = "(",
                  target = "q0", stack_push = {} },
                { source = "q0", event = nil, stack_read = "$",
                  target = "accept", stack_push = {} },
            },
            "q0", "$", {"accept"}
        )
    end

    describe("construction", function()

        it("creates a valid PDA", function()
            local pda = build_paren_pda()
            assert.are.equal("q0", pda:current_state())
            assert.are.same({"$"}, pda:stack())
            assert.are.equal("$", pda:stack_top())
        end)

        it("errors on empty states", function()
            assert.has_error(function()
                PDA.new({}, {"a"}, {"$"}, {}, "q0", "$", {})
            end)
        end)

        it("errors on invalid initial state", function()
            assert.has_error(function()
                PDA.new({"q0"}, {"a"}, {"$"}, {}, "q1", "$", {})
            end)
        end)

        it("errors on invalid initial stack symbol", function()
            assert.has_error(function()
                PDA.new({"q0"}, {"a"}, {"$"}, {}, "q0", "X", {})
            end)
        end)

        it("errors on invalid accepting state", function()
            assert.has_error(function()
                PDA.new({"q0"}, {"a"}, {"$"}, {}, "q0", "$", {"q1"})
            end)
        end)

        it("errors on duplicate transitions", function()
            assert.has_error(function()
                PDA.new(
                    {"q0"}, {"a"}, {"$"},
                    {
                        { source = "q0", event = "a", stack_read = "$",
                          target = "q0", stack_push = {"$"} },
                        { source = "q0", event = "a", stack_read = "$",
                          target = "q0", stack_push = {} },
                    },
                    "q0", "$", {}
                )
            end)
        end)

    end)

    describe("process", function()

        it("processes balanced parentheses", function()
            local pda = build_paren_pda()
            pda:process("(")
            assert.are.equal("q0", pda:current_state())
            assert.are.same({"$", "("}, pda:stack())

            pda:process(")")
            assert.are.equal("q0", pda:current_state())
            assert.are.same({"$"}, pda:stack())
        end)

        it("errors on missing transition", function()
            local pda = build_paren_pda()
            assert.has_error(function()
                pda:process(")")  -- can't close when stack top is "$"
            end)
        end)

    end)

    describe("process_sequence", function()

        it("processes full sequence and tries epsilon at end", function()
            local pda = build_paren_pda()
            local trace = pda:process_sequence({"(", ")"})
            -- Should have processed "(", ")", and then epsilon to "accept"
            assert.are.equal("accept", pda:current_state())
            assert.are.equal(3, #trace)
        end)

        it("handles nested parentheses", function()
            local pda = build_paren_pda()
            pda:process_sequence({"(", "(", ")", ")"})
            assert.are.equal("accept", pda:current_state())
        end)

    end)

    describe("accepts", function()

        it("accepts balanced parentheses", function()
            local pda = build_paren_pda()
            assert.is_true(pda:accepts({"(", ")"}))
            assert.is_true(pda:accepts({"(", "(", ")", ")"}))
            assert.is_true(pda:accepts({"(", ")", "(", ")"}))
            assert.is_true(pda:accepts({}))
        end)

        it("rejects unbalanced parentheses", function()
            local pda = build_paren_pda()
            assert.is_false(pda:accepts({"("}))
            assert.is_false(pda:accepts({"(", "(", ")"}))
            assert.is_false(pda:accepts({")"}))
            assert.is_false(pda:accepts({")", "("}))
        end)

        it("does not modify current state", function()
            local pda = build_paren_pda()
            pda:accepts({"(", ")"})
            assert.are.equal("q0", pda:current_state())
            assert.are.same({"$"}, pda:stack())
        end)

    end)

    describe("reset", function()

        it("restores initial state and stack", function()
            local pda = build_paren_pda()
            pda:process("(")
            pda:process("(")
            assert.are.same({"$", "(", "("}, pda:stack())

            pda:reset()
            assert.are.equal("q0", pda:current_state())
            assert.are.same({"$"}, pda:stack())
            assert.are.equal(0, #pda:trace())
        end)

    end)

    describe("stack operations", function()

        it("stack_top returns nil for empty stack", function()
            -- Build a PDA that empties the stack
            local pda = PDA.new(
                {"q0"}, {"a"}, {"$"},
                {
                    { source = "q0", event = "a", stack_read = "$",
                      target = "q0", stack_push = {} },
                },
                "q0", "$", {}
            )
            pda:process("a")
            assert.is_nil(pda:stack_top())
            assert.are.same({}, pda:stack())
        end)

    end)

    describe("trace", function()

        it("records full trace with stack snapshots", function()
            local pda = build_paren_pda()
            pda:process("(")
            local trace = pda:trace()
            assert.are.equal(1, #trace)
            assert.are.equal("q0", trace[1].source)
            assert.are.equal("(", trace[1].event)
            assert.are.equal("$", trace[1].stack_read)
            assert.are.equal("q0", trace[1].target)
            assert.are.same({"$", "("}, trace[1].stack_push)
            assert.are.same({"$", "("}, trace[1].stack_after)
        end)

    end)

    describe("a^n b^n language", function()

        -- Build a PDA that accepts a^n b^n (n >= 1)
        local function build_anbn_pda()
            return PDA.new(
                {"q0", "q1", "accept"},
                {"a", "b"},
                {"A", "$"},
                {
                    -- Push A for each 'a'
                    { source = "q0", event = "a", stack_read = "$",
                      target = "q0", stack_push = {"$", "A"} },
                    { source = "q0", event = "a", stack_read = "A",
                      target = "q0", stack_push = {"A", "A"} },
                    -- Switch to popping on first 'b'
                    { source = "q0", event = "b", stack_read = "A",
                      target = "q1", stack_push = {} },
                    -- Pop A for each 'b'
                    { source = "q1", event = "b", stack_read = "A",
                      target = "q1", stack_push = {} },
                    -- Accept when stack is empty (just $)
                    { source = "q1", event = nil, stack_read = "$",
                      target = "accept", stack_push = {} },
                },
                "q0", "$", {"accept"}
            )
        end

        it("accepts a^n b^n", function()
            local pda = build_anbn_pda()
            assert.is_true(pda:accepts({"a", "b"}))
            assert.is_true(pda:accepts({"a", "a", "b", "b"}))
            assert.is_true(pda:accepts({"a", "a", "a", "b", "b", "b"}))
        end)

        it("rejects unequal a/b counts", function()
            local pda = build_anbn_pda()
            assert.is_false(pda:accepts({"a"}))
            assert.is_false(pda:accepts({"b"}))
            assert.is_false(pda:accepts({"a", "a", "b"}))
            assert.is_false(pda:accepts({"a", "b", "b"}))
            assert.is_false(pda:accepts({}))
        end)

    end)

end)


-- =========================================================================
-- ModalStateMachine Tests
-- =========================================================================

describe("ModalStateMachine", function()

    -- Build a simple two-mode machine: "normal" and "insert"
    local function build_editor_modes()
        local normal_dfa = DFA.new(
            {"idle", "command"},
            {"key", "enter"},
            make_transitions(
                {"idle", "key"}, "command",
                {"idle", "enter"}, "idle",
                {"command", "key"}, "command",
                {"command", "enter"}, "idle"
            ),
            "idle",
            {"idle"},
            nil
        )

        local insert_dfa = DFA.new(
            {"typing", "done"},
            {"char", "escape"},
            make_transitions(
                {"typing", "char"}, "typing",
                {"typing", "escape"}, "done",
                {"done", "char"}, "typing",
                {"done", "escape"}, "done"
            ),
            "typing",
            {"done"},
            nil
        )

        return ModalStateMachine.new(
            { normal = normal_dfa, insert = insert_dfa },
            make_transitions(
                {"normal", "enter_insert"}, "insert",
                {"insert", "exit_insert"}, "normal"
            ),
            "normal"
        )
    end

    describe("construction", function()

        it("creates a valid modal machine", function()
            local msm = build_editor_modes()
            assert.are.equal("normal", msm:current_mode())
            assert.are.same({"insert", "normal"}, msm:modes())
        end)

        it("errors on empty modes", function()
            assert.has_error(function()
                ModalStateMachine.new({}, {}, "normal")
            end)
        end)

        it("errors on invalid initial mode", function()
            assert.has_error(function()
                local dfa = build_turnstile()
                ModalStateMachine.new({ normal = dfa }, {}, "unknown")
            end)
        end)

        it("errors on invalid mode transition source", function()
            assert.has_error(function()
                local dfa = build_turnstile()
                ModalStateMachine.new(
                    { normal = dfa },
                    make_transitions({"unknown", "go"}, "normal"),
                    "normal"
                )
            end)
        end)

        it("errors on invalid mode transition target", function()
            assert.has_error(function()
                local dfa = build_turnstile()
                ModalStateMachine.new(
                    { normal = dfa },
                    make_transitions({"normal", "go"}, "unknown"),
                    "normal"
                )
            end)
        end)

    end)

    describe("switch_mode", function()

        it("switches between modes", function()
            local msm = build_editor_modes()
            assert.are.equal("normal", msm:current_mode())

            local new_mode = msm:switch_mode("enter_insert")
            assert.are.equal("insert", new_mode)
            assert.are.equal("insert", msm:current_mode())

            new_mode = msm:switch_mode("exit_insert")
            assert.are.equal("normal", new_mode)
        end)

        it("resets target DFA on switch", function()
            local msm = build_editor_modes()

            -- Process some events in normal mode
            msm:process("key")
            assert.are.equal("command", msm:active_machine():current_state())

            -- Switch to insert, then back to normal
            msm:switch_mode("enter_insert")
            msm:switch_mode("exit_insert")

            -- Normal mode DFA should be reset to initial state
            assert.are.equal("idle", msm:active_machine():current_state())
        end)

        it("errors on invalid trigger", function()
            local msm = build_editor_modes()
            assert.has_error(function()
                msm:switch_mode("nonexistent")
            end)
        end)

        it("records mode trace", function()
            local msm = build_editor_modes()
            msm:switch_mode("enter_insert")
            msm:switch_mode("exit_insert")

            local trace = msm:mode_trace()
            assert.are.equal(2, #trace)
            assert.are.equal("normal", trace[1].from_mode)
            assert.are.equal("enter_insert", trace[1].trigger)
            assert.are.equal("insert", trace[1].to_mode)
            assert.are.equal("insert", trace[2].from_mode)
            assert.are.equal("exit_insert", trace[2].trigger)
            assert.are.equal("normal", trace[2].to_mode)
        end)

    end)

    describe("process", function()

        it("delegates to active DFA", function()
            local msm = build_editor_modes()
            local new_state = msm:process("key")
            assert.are.equal("command", new_state)
        end)

        it("processes in correct mode after switching", function()
            local msm = build_editor_modes()
            msm:switch_mode("enter_insert")
            local new_state = msm:process("char")
            assert.are.equal("typing", new_state)
        end)

    end)

    describe("reset", function()

        it("resets to initial mode and clears all traces", function()
            local msm = build_editor_modes()
            msm:process("key")
            msm:switch_mode("enter_insert")
            msm:process("char")

            msm:reset()
            assert.are.equal("normal", msm:current_mode())
            assert.are.equal(0, #msm:mode_trace())
            assert.are.equal("idle", msm:active_machine():current_state())
        end)

    end)

    describe("active_machine", function()

        it("returns the correct DFA for current mode", function()
            local msm = build_editor_modes()
            local active = msm:active_machine()
            -- Normal mode DFA should have "idle" and "command" states
            assert.are.same({"command", "idle"}, active:states())
        end)

    end)

end)


-- =========================================================================
-- Minimize Tests
-- =========================================================================

describe("Minimize", function()

    it("minimizes a simple DFA with equivalent states", function()
        -- DFA with states A, B, C where B and C are equivalent
        -- (both are accepting states with the same transitions)
        local dfa = DFA.new(
            {"A", "B", "C"},
            {"0", "1"},
            make_transitions(
                {"A", "0"}, "B",
                {"A", "1"}, "C",
                {"B", "0"}, "B",
                {"B", "1"}, "B",
                {"C", "0"}, "C",
                {"C", "1"}, "C"
            ),
            "A",
            {"B", "C"},
            nil
        )

        local min_dfa = Minimize(dfa)
        -- B and C should be merged into one state
        local min_states = min_dfa:states()
        assert.is_true(#min_states < 3)
        -- The language should be preserved: accepts any non-empty string
        assert.is_true(min_dfa:accepts({"0"}))
        assert.is_true(min_dfa:accepts({"1"}))
        assert.is_true(min_dfa:accepts({"0", "1"}))
        assert.is_false(min_dfa:accepts({}))
    end)

    it("removes unreachable states", function()
        local dfa = DFA.new(
            {"q0", "q1", "orphan"},
            {"a"},
            make_transitions(
                {"q0", "a"}, "q1",
                {"q1", "a"}, "q0",
                {"orphan", "a"}, "orphan"
            ),
            "q0",
            {},
            nil
        )

        local min_dfa = Minimize(dfa)
        local states = min_dfa:states()
        -- "orphan" should be removed
        for _, s in ipairs(states) do
            assert.is_not_equal("orphan", s)
        end
    end)

    it("preserves already-minimal DFA", function()
        local dfa = build_turnstile()
        local min_dfa = Minimize(dfa)
        -- Turnstile is already minimal — should have same number of states
        assert.are.equal(#dfa:states(), #min_dfa:states())
    end)

    it("minimizes NFA->DFA result", function()
        -- NFA that accepts strings ending in "ab"
        local nfa = NFA.new(
            {"q0", "q1", "q2"},
            {"a", "b"},
            make_transitions(
                {"q0", "a"}, {"q0", "q1"},
                {"q0", "b"}, {"q0"},
                {"q1", "b"}, {"q2"}
            ),
            "q0",
            {"q2"}
        )

        local dfa = nfa:to_dfa()
        local min_dfa = Minimize(dfa)

        -- Both should accept the same language
        assert.is_true(min_dfa:accepts({"a", "b"}))
        assert.is_true(min_dfa:accepts({"a", "a", "b"}))
        assert.is_false(min_dfa:accepts({"a"}))
        assert.is_false(min_dfa:accepts({"b"}))
    end)

end)


-- =========================================================================
-- Module-level Tests
-- =========================================================================

describe("state_machine module", function()

    it("has a version", function()
        assert.are.equal("0.1.0", sm.VERSION)
    end)

    it("exports EPSILON constant", function()
        assert.are.equal("", sm.EPSILON)
    end)

    it("exports all classes", function()
        assert.is_not_nil(sm.DFA)
        assert.is_not_nil(sm.NFA)
        assert.is_not_nil(sm.PDA)
        assert.is_not_nil(sm.ModalStateMachine)
        assert.is_not_nil(sm.Minimize)
        assert.is_not_nil(sm.TransitionRecord)
    end)

end)


-- =========================================================================
-- Edge Case Tests
-- =========================================================================

describe("edge cases", function()

    describe("DFA with self-loops", function()

        it("handles states that transition to themselves", function()
            local dfa = DFA.new(
                {"q0"},
                {"a"},
                make_transitions({"q0", "a"}, "q0"),
                "q0",
                {"q0"},
                nil
            )
            assert.is_true(dfa:accepts({}))
            assert.is_true(dfa:accepts({"a"}))
            assert.is_true(dfa:accepts({"a", "a", "a"}))
        end)

    end)

    describe("DFA with single state", function()

        it("works with one accepting state", function()
            local dfa = DFA.new(
                {"only"},
                {"x"},
                make_transitions({"only", "x"}, "only"),
                "only",
                {"only"},
                nil
            )
            assert.is_true(dfa:accepts({}))
            assert.is_true(dfa:accepts({"x", "x"}))
        end)

    end)

    describe("NFA with only epsilon transitions", function()

        it("accepts immediately via epsilon closure", function()
            local nfa = NFA.new(
                {"q0", "q1"},
                {"a"},
                make_transitions(
                    {"q0", ""}, {"q1"},
                    {"q1", "a"}, {"q1"}
                ),
                "q0",
                {"q1"}
            )
            -- q1 is in epsilon closure of q0, and q1 is accepting
            assert.is_true(nfa:accepts({}))
        end)

    end)

    describe("PDA process errors", function()

        it("errors when processing with empty stack", function()
            local pda = PDA.new(
                {"q0"}, {"a"}, {"$"},
                {
                    { source = "q0", event = "a", stack_read = "$",
                      target = "q0", stack_push = {} },
                },
                "q0", "$", {}
            )
            pda:process("a")  -- empties the stack
            assert.has_error(function()
                pda:process("a")  -- no transition with empty stack
            end)
        end)

    end)

    describe("branch predictor as DFA", function()

        -- The 2-bit saturating counter branch predictor is a DFA
        it("models a 2-bit branch predictor", function()
            local dfa = DFA.new(
                {"SNT", "WNT", "WT", "ST"},
                {"taken", "not_taken"},
                make_transitions(
                    {"SNT", "taken"}, "WNT",
                    {"SNT", "not_taken"}, "SNT",
                    {"WNT", "taken"}, "WT",
                    {"WNT", "not_taken"}, "SNT",
                    {"WT", "taken"}, "ST",
                    {"WT", "not_taken"}, "WNT",
                    {"ST", "taken"}, "ST",
                    {"ST", "not_taken"}, "WT"
                ),
                "WNT",
                {"WT", "ST"},  -- predict "taken" in these states
                nil
            )

            -- After two "taken" events, should predict taken
            assert.is_true(dfa:accepts({"taken", "taken"}))

            -- After "taken" then "not_taken", back to WNT (not accepting)
            assert.is_false(dfa:accepts({"taken", "not_taken"}))
        end)

    end)

end)
