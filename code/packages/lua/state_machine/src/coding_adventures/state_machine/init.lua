-- state_machine — Finite automata: DFA, NFA, PDA, Modal, and Minimization
-- =========================================================================
--
-- This package implements finite automata — the theoretical foundation of
-- all computation. It is a Lua 5.4 port of the Go state-machine package
-- in the coding-adventures monorepo.
--
-- # What are state machines?
--
-- Every state machine — whether it is a simple traffic light controller or
-- a complex HTML tokenizer — is built from the same fundamental concepts:
--
--   - State:  where the machine is right now (e.g., "locked", "red", "q0")
--   - Event:  what input the machine just received (e.g., "coin", "timer")
--   - Transition: the rule "in state X, on event Y, go to state Z"
--   - Action: an optional side effect that fires when a transition occurs
--
-- # The Chomsky Hierarchy
--
-- State machines sit at the base of the Chomsky hierarchy of formal languages:
--
--   Level   | Machine              | Language Class
--   --------|----------------------|--------------------
--   Type 3  | DFA / NFA            | Regular
--   Type 2  | PDA (pushdown)       | Context-Free
--   Type 1  | LBA                  | Context-Sensitive
--   Type 0  | Turing Machine       | Recursively Enumerable
--
-- This package implements DFA (Type 3), NFA (Type 3), PDA (Type 2), and
-- modal state machines (a practical extension for context-sensitive tokenizing).
--
-- # OOP pattern
--
-- We use the standard Lua metatable OOP pattern:
--
--     local DFA = {}
--     DFA.__index = DFA
--     function DFA.new(...) ... end
--
-- This gives us method dispatch via the : operator:
--
--     local m = DFA.new(...)
--     m:process("coin")
--
-- # Dependencies
--
-- This package depends on coding_adventures.directed_graph for its
-- LabeledGraph implementation, which provides structural queries
-- like transitive_closure for reachability analysis.

local dg = require("coding_adventures.directed_graph")
local LabeledGraph = dg.LabeledGraph

-- =========================================================================
-- Helper functions
-- =========================================================================

--- Return a sorted array of keys from a set (table with boolean values).
--
-- This ensures deterministic output regardless of Lua's table iteration
-- order — critical for tests and for algorithms that need reproducible
-- results.
--
-- @param set table A table mapping strings to true.
-- @return table A sorted array of key strings.
local function sorted_keys(set)
    local keys = {}
    for k in pairs(set) do
        keys[#keys + 1] = k
    end
    table.sort(keys)
    return keys
end

--- Make a set (table with boolean values) from an array.
--
-- @param arr table An array of strings.
-- @return table A table mapping each string to true.
local function make_set(arr)
    local set = {}
    for _, v in ipairs(arr) do
        set[v] = true
    end
    return set
end

--- Check whether two sets share at least one element.
--
-- @param a table A set (table with boolean values).
-- @param b table A set (table with boolean values).
-- @return boolean True if the sets intersect.
local function sets_intersect(a, b)
    for k in pairs(a) do
        if b[k] then
            return true
        end
    end
    return false
end

--- Make a transition key string from source and event.
--
-- Lua tables cannot use arrays as keys (they compare by reference, not
-- value). We concatenate source and event with a null byte separator
-- to create a unique string key.
--
-- @param source string The source state.
-- @param event string The event/input symbol.
-- @return string A unique key for lookups.
local function trans_key(source, event)
    return source .. "\0" .. event
end

--- Copy a table (shallow).
--
-- @param t table The table to copy.
-- @return table A shallow copy.
local function shallow_copy(t)
    local result = {}
    for k, v in pairs(t) do
        result[k] = v
    end
    return result
end

--- Copy an array (shallow).
--
-- @param arr table The array to copy.
-- @return table A shallow copy.
local function array_copy(arr)
    local result = {}
    for i, v in ipairs(arr) do
        result[i] = v
    end
    return result
end


-- =========================================================================
-- TransitionRecord
-- =========================================================================
--
-- Captures one step in a state machine's execution trace. Every time a
-- machine processes an input and transitions from one state to another,
-- a TransitionRecord is created. This provides complete visibility into
-- the machine's execution history.
--
-- Fields:
--   - source: the state before the transition
--   - event: the input that triggered it (empty string for epsilon)
--   - target: the state after the transition
--   - action_name: the name of the action that fired, if any ("" if none)

--- Create a TransitionRecord.
--
-- @param source string The source state.
-- @param event string The triggering event.
-- @param target string The target state.
-- @param action_name string The name of the fired action (or "").
-- @return table A TransitionRecord table.
local function TransitionRecord(source, event, target, action_name)
    return {
        source = source,
        event = event,
        target = target,
        action_name = action_name or "",
    }
end


-- =========================================================================
-- DFA — Deterministic Finite Automaton
-- =========================================================================
--
-- # What is a DFA?
--
-- A DFA is the simplest kind of state machine. It has a fixed set of states,
-- reads input symbols one at a time, and follows exactly one transition for
-- each (state, input) pair. There is no ambiguity, no guessing, no backtracking.
--
-- Formally, a DFA is a 5-tuple (Q, Sigma, delta, q0, F):
--
--   Q     = a finite set of states
--   Sigma = a finite set of input symbols (the "alphabet")
--   delta = a transition function: Q x Sigma -> Q
--   q0    = the initial state (q0 in Q)
--   F     = a set of accepting/final states (F subset Q)
--
-- # Why "deterministic"?
--
-- "Deterministic" means there is exactly ONE next state for every
-- (state, input) combination. Given the same starting state and the same
-- input sequence, a DFA always follows the same path and reaches the same
-- final state. This makes DFAs predictable, efficient, and easy to
-- implement in hardware.
--
-- # Example: a turnstile
--
--   States:      {locked, unlocked}
--   Alphabet:    {coin, push}
--   Transitions: (locked, coin) -> unlocked
--                (locked, push) -> locked
--                (unlocked, coin) -> unlocked
--                (unlocked, push) -> locked
--   Initial:     locked
--   Accepting:   {unlocked}

local DFA = {}
DFA.__index = DFA

--- Create a new Deterministic Finite Automaton.
--
-- All inputs are validated eagerly so that errors are caught at definition
-- time, not at runtime when the machine processes its first input. This is
-- the "fail fast" principle.
--
-- @param states table Array of state name strings. Must be non-empty.
-- @param alphabet table Array of input symbol strings.
-- @param transitions table Mapping from {source, event} arrays to target
--   state strings. Keys should be two-element arrays.
-- @param initial string The starting state. Must be in states.
-- @param accepting table Array of accepting state name strings.
-- @param actions table|nil Optional mapping from {source, event} to callback
--   functions. Each callback receives (source, event, target). Pass nil if
--   no actions are needed.
-- @return DFA A new DFA instance.
function DFA.new(states, alphabet, transitions, initial, accepting, actions)
    -- --- Validate states ---
    if #states == 0 then
        error("statemachine: states set must be non-empty")
    end

    local state_set = make_set(states)

    -- --- Validate alphabet ---
    local alpha_set = make_set(alphabet)

    -- --- Validate initial state ---
    if not state_set[initial] then
        error(string.format(
            "statemachine: initial state %q is not in the states set",
            initial
        ))
    end

    -- --- Validate accepting states ---
    local accept_set = {}
    for _, a in ipairs(accepting) do
        if not state_set[a] then
            error(string.format(
                "statemachine: accepting state %q is not in the states set",
                a
            ))
        end
        accept_set[a] = true
    end

    -- --- Build and validate transitions ---
    --
    -- transitions is a table mapping {source, event} -> target.
    -- We convert it to string-keyed map for O(1) lookups.
    local trans = {}
    for key, target in pairs(transitions) do
        local source, event = key[1], key[2]
        if not state_set[source] then
            error(string.format(
                "statemachine: transition source %q is not in the states set",
                source
            ))
        end
        if not alpha_set[event] then
            error(string.format(
                "statemachine: transition event %q is not in the alphabet",
                event
            ))
        end
        if not state_set[target] then
            error(string.format(
                "statemachine: transition target %q (from (%s, %s)) is not in the states set",
                target, source, event
            ))
        end
        trans[trans_key(source, event)] = target
    end

    -- --- Validate actions ---
    local acts = {}
    if actions then
        for key, action in pairs(actions) do
            local tk = trans_key(key[1], key[2])
            if not trans[tk] then
                error(string.format(
                    "statemachine: action defined for (%s, %s) but no transition exists for that pair",
                    key[1], key[2]
                ))
            end
            acts[tk] = action
        end
    end

    -- --- Build internal graph representation ---
    --
    -- We build a LabeledGraph from states and transitions so that
    -- structural queries like reachable_states() can delegate to the
    -- graph's transitive_closure algorithm instead of hand-rolling BFS.
    local graph = LabeledGraph.new_allow_self_loops()
    for s in pairs(state_set) do
        graph:add_node(s)
    end
    for key, target in pairs(transitions) do
        local source, event = key[1], key[2]
        graph:add_edge(source, target, event)
    end

    local self = setmetatable({}, DFA)
    self._states = state_set
    self._alphabet = alpha_set
    self._transitions = trans
    self._initial = initial
    self._accepting = accept_set
    self._actions = acts
    self._graph = graph
    self._current = initial
    self._trace = {}
    return self
end

-- =========================================================================
-- Getters
-- =========================================================================

--- Return a sorted array of all state names.
-- @return table Sorted array of state strings.
function DFA:states()
    return sorted_keys(self._states)
end

--- Return a sorted array of all input symbols.
-- @return table Sorted array of alphabet strings.
function DFA:alphabet()
    return sorted_keys(self._alphabet)
end

--- Return the initial state name.
-- @return string The initial state.
function DFA:initial()
    return self._initial
end

--- Return a sorted array of accepting state names.
-- @return table Sorted array of accepting state strings.
function DFA:accepting()
    return sorted_keys(self._accepting)
end

--- Return the state the machine is currently in.
-- @return string The current state.
function DFA:current_state()
    return self._current
end

--- Return a copy of the execution trace.
-- @return table Array of TransitionRecord tables.
function DFA:trace()
    local result = {}
    for i, rec in ipairs(self._trace) do
        result[i] = {
            source = rec.source,
            event = rec.event,
            target = rec.target,
            action_name = rec.action_name,
        }
    end
    return result
end

--- Return a copy of the transitions as an array of {source, event, target}.
--
-- This is more Lua-idiomatic than the Go approach of returning a map
-- with [2]string keys.
--
-- @return table Array of {source, event, target} arrays.
function DFA:transitions()
    local result = {}
    for key, target in pairs(self._transitions) do
        local sep = key:find("\0", 1, true)
        local source = key:sub(1, sep - 1)
        local event = key:sub(sep + 1)
        result[#result + 1] = {source, event, target}
    end
    table.sort(result, function(a, b)
        if a[1] ~= b[1] then return a[1] < b[1] end
        return a[2] < b[2]
    end)
    return result
end

-- =========================================================================
-- Processing
-- =========================================================================

--- Process a single input event and return the new state.
--
-- Looks up the transition for (current_state, event), moves to the target
-- state, executes the action (if defined), logs a TransitionRecord, and
-- returns the new current state.
--
-- Raises an error if:
--   - the event is not in the alphabet
--   - no transition is defined for (current_state, event)
--
-- @param event string The input event to process.
-- @return string The new current state.
function DFA:process(event)
    -- Validate the event
    if not self._alphabet[event] then
        error(string.format(
            "statemachine: event %q is not in the alphabet",
            event
        ))
    end

    -- Look up the transition
    local key = trans_key(self._current, event)
    local target = self._transitions[key]
    if not target then
        error(string.format(
            "statemachine: no transition defined for (state=%q, event=%q)",
            self._current, event
        ))
    end

    -- Execute the action if one exists
    local action_name = ""
    local action = self._actions[key]
    if action then
        action(self._current, event, target)
        action_name = "action"
    end

    -- Log the transition
    self._trace[#self._trace + 1] = TransitionRecord(
        self._current, event, target, action_name
    )

    -- Move to the new state
    self._current = target
    return target
end

--- Process a sequence of inputs and return the new trace entries.
--
-- Each input is processed in order. The machine's state is updated after
-- each input.
--
-- @param events table Array of event strings.
-- @return table Array of TransitionRecord tables generated during this call.
function DFA:process_sequence(events)
    local trace_start = #self._trace
    for _, event in ipairs(events) do
        self:process(event)
    end
    local result = {}
    for i = trace_start + 1, #self._trace do
        result[#result + 1] = self._trace[i]
    end
    return result
end

--- Check if the machine accepts the input sequence.
--
-- Processes the entire sequence starting from the initial state and returns
-- true if the machine ends in an accepting state.
--
-- IMPORTANT: This method does NOT modify the machine's current state or
-- trace. It runs a simulation starting from the initial state.
--
-- Raises an error if an event is not in the alphabet.
-- Returns false (does not error) if a transition is missing — the machine
-- is considered to have "died" at that point.
--
-- @param events table Array of event strings.
-- @return boolean True if the sequence is accepted.
function DFA:accepts(events)
    local state = self._initial
    for _, event in ipairs(events) do
        if not self._alphabet[event] then
            error(string.format(
                "statemachine: event %q is not in the alphabet",
                event
            ))
        end
        local key = trans_key(state, event)
        local target = self._transitions[key]
        if not target then
            return false
        end
        state = target
    end
    return self._accepting[state] == true
end

--- Reset the machine to its initial state and clear the trace.
function DFA:reset()
    self._current = self._initial
    self._trace = {}
end

-- =========================================================================
-- Introspection
-- =========================================================================

--- Return the set of states reachable from the initial state.
--
-- Uses BFS over the transition graph via the LabeledGraph's
-- transitive_closure. A state is reachable if there exists any sequence
-- of inputs that leads from the initial state to that state.
--
-- @return table A set (table with boolean values) of reachable states.
function DFA:reachable_states()
    local reachable, err = self._graph:transitive_closure(self._initial)
    if not reachable then
        return { [self._initial] = true }
    end
    reachable[self._initial] = true
    return reachable
end

--- Check if a transition is defined for every (state, input) pair.
--
-- A complete DFA never gets "stuck" — every state handles every input.
--
-- @return boolean True if the DFA is complete.
function DFA:is_complete()
    for s in pairs(self._states) do
        for e in pairs(self._alphabet) do
            if not self._transitions[trans_key(s, e)] then
                return false
            end
        end
    end
    return true
end

--- Check for common issues and return a list of warnings.
--
-- Checks performed:
--   - Unreachable states (defined but never entered)
--   - Missing transitions (incomplete DFA)
--   - Accepting states that are unreachable
--
-- @return table Array of warning strings (empty if no issues).
function DFA:validate()
    local warnings = {}

    -- Check for unreachable states
    local reachable = self:reachable_states()
    local unreachable = {}
    for s in pairs(self._states) do
        if not reachable[s] then
            unreachable[#unreachable + 1] = s
        end
    end
    if #unreachable > 0 then
        table.sort(unreachable)
        warnings[#warnings + 1] = string.format("Unreachable states: [%s]",
            table.concat(unreachable, ", "))
    end

    -- Check for unreachable accepting states
    local unreachable_accepting = {}
    for s in pairs(self._accepting) do
        if not reachable[s] then
            unreachable_accepting[#unreachable_accepting + 1] = s
        end
    end
    if #unreachable_accepting > 0 then
        table.sort(unreachable_accepting)
        warnings[#warnings + 1] = string.format("Unreachable accepting states: [%s]",
            table.concat(unreachable_accepting, ", "))
    end

    -- Check for missing transitions
    local missing = {}
    local sorted_states = sorted_keys(self._states)
    local sorted_alpha = sorted_keys(self._alphabet)
    for _, s in ipairs(sorted_states) do
        for _, e in ipairs(sorted_alpha) do
            if not self._transitions[trans_key(s, e)] then
                missing[#missing + 1] = string.format("(%s, %s)", s, e)
            end
        end
    end
    if #missing > 0 then
        warnings[#warnings + 1] = string.format("Missing transitions: %s",
            table.concat(missing, ", "))
    end

    return warnings
end

-- =========================================================================
-- Visualization
-- =========================================================================

--- Return a Graphviz DOT representation of this DFA.
--
-- Accepting states are drawn as double circles. The initial state has an
-- invisible node pointing to it (the standard convention for marking the
-- start state in automata diagrams).
--
-- @return string A DOT language string.
function DFA:to_dot()
    local lines = {}
    lines[#lines + 1] = "digraph DFA {"
    lines[#lines + 1] = "    rankdir=LR;"
    lines[#lines + 1] = ""

    -- Invisible start node
    lines[#lines + 1] = "    __start [shape=point, width=0.2];"
    lines[#lines + 1] = string.format('    __start -> %q;', self._initial)
    lines[#lines + 1] = ""

    -- State shapes
    for _, state in ipairs(sorted_keys(self._states)) do
        local shape = "circle"
        if self._accepting[state] then
            shape = "doublecircle"
        end
        lines[#lines + 1] = string.format('    %q [shape=%s];', state, shape)
    end
    lines[#lines + 1] = ""

    -- Group transitions with same source and target to combine labels
    local edge_labels = {}  -- "source\0target" -> array of event strings
    for key, target in pairs(self._transitions) do
        local sep = key:find("\0", 1, true)
        local source = key:sub(1, sep - 1)
        local event = key:sub(sep + 1)
        local ek = source .. "\0" .. target
        if not edge_labels[ek] then
            edge_labels[ek] = {}
        end
        edge_labels[ek][#edge_labels[ek] + 1] = event
    end

    -- Sort edge keys for deterministic output
    local ekeys = {}
    for ek in pairs(edge_labels) do
        ekeys[#ekeys + 1] = ek
    end
    table.sort(ekeys)

    for _, ek in ipairs(ekeys) do
        local sep = ek:find("\0", 1, true)
        local source = ek:sub(1, sep - 1)
        local target = ek:sub(sep + 1)
        local labels = edge_labels[ek]
        table.sort(labels)
        local label = table.concat(labels, ", ")
        lines[#lines + 1] = string.format('    %q -> %q [label=%q];', source, target, label)
    end

    lines[#lines + 1] = "}"
    return table.concat(lines, "\n")
end

--- Return an ASCII transition table.
--
-- Accepting states are marked with (*). The initial state is marked with (>).
--
-- @return string An ASCII table string.
function DFA:to_ascii()
    local sorted_events = sorted_keys(self._alphabet)
    local sorted_states_list = sorted_keys(self._states)

    -- Calculate column widths
    local state_width = 0
    for _, s in ipairs(sorted_states_list) do
        local w = #s + 4  -- +4 for markers like ">*"
        if w > state_width then
            state_width = w
        end
    end

    local event_width = 5  -- minimum
    for _, e in ipairs(sorted_events) do
        if #e > event_width then
            event_width = #e
        end
    end
    for _, s in ipairs(sorted_states_list) do
        for _, e in ipairs(sorted_events) do
            local target = self._transitions[trans_key(s, e)]
            if target and #target > event_width then
                event_width = #target
            end
        end
    end

    local lines = {}

    -- Header row
    local header = string.rep(" ", state_width) .. "|"
    for _, event in ipairs(sorted_events) do
        header = header .. string.format(" %-" .. event_width .. "s |", event)
    end
    lines[#lines + 1] = header

    -- Separator
    local sep = string.rep("-", state_width) .. "+"
    for _ in ipairs(sorted_events) do
        sep = sep .. string.rep("-", event_width + 2) .. "+"
    end
    sep = sep:sub(1, #sep - 1)  -- remove trailing +
    lines[#lines + 1] = sep

    -- Data rows
    for _, state in ipairs(sorted_states_list) do
        local markers = ""
        if state == self._initial then
            markers = markers .. ">"
        end
        if self._accepting[state] then
            markers = markers .. "*"
        end
        local label
        if markers ~= "" then
            label = markers .. " " .. state
        else
            label = "  " .. state
        end

        local row = string.format("%-" .. state_width .. "s|", label)
        for _, event in ipairs(sorted_events) do
            local target = self._transitions[trans_key(state, event)]
            if not target then
                target = "\u{2014}"  -- em-dash
            end
            row = row .. string.format(" %-" .. event_width .. "s |", target)
        end
        lines[#lines + 1] = row
    end

    return table.concat(lines, "\n")
end

--- Return the transition table as a list of rows.
--
-- First row is the header: {"State", event1, event2, ...}.
-- Subsequent rows: {state_name, target1, target2, ...}.
-- Missing transitions are represented as em-dash.
--
-- @return table Array of arrays of strings.
function DFA:to_table()
    local sorted_events = sorted_keys(self._alphabet)
    local sorted_states_list = sorted_keys(self._states)

    local rows = {}
    local header = {"State"}
    for _, e in ipairs(sorted_events) do
        header[#header + 1] = e
    end
    rows[#rows + 1] = header

    for _, state in ipairs(sorted_states_list) do
        local row = {state}
        for _, event in ipairs(sorted_events) do
            local target = self._transitions[trans_key(state, event)]
            if not target then
                target = "\u{2014}"
            end
            row[#row + 1] = target
        end
        rows[#rows + 1] = row
    end

    return rows
end


-- =========================================================================
-- NFA — Non-deterministic Finite Automaton
-- =========================================================================
--
-- # What is an NFA?
--
-- An NFA relaxes the deterministic constraint of a DFA in two ways:
--
-- 1. Multiple transitions: A single (state, input) pair can lead to
--    multiple target states. The machine explores all possibilities
--    simultaneously — like spawning parallel universes.
--
-- 2. Epsilon transitions: The machine can jump to another state without
--    consuming any input. These are "free" moves.
--
-- # The "parallel universes" model
--
-- Think of an NFA as a machine that clones itself at every non-deterministic
-- choice point. All clones run in parallel:
--
--   - A clone that reaches a dead end simply vanishes.
--   - A clone that reaches an accepting state means the whole NFA accepts.
--   - If ALL clones die without reaching an accepting state, the NFA rejects.
--
-- # Formal definition
--
--   NFA = (Q, Sigma, delta, q0, F)
--   delta = transition function: Q x (Sigma union {epsilon}) -> P(Q)
--           maps (state, input_or_epsilon) to a SET of states

local NFA = {}
NFA.__index = NFA

--- The sentinel value for epsilon transitions.
--
-- We use the empty string "" as the epsilon symbol. This works because
-- no real input alphabet should contain the empty string — input symbols
-- are always at least one character long.
local EPSILON = ""

--- Create a new Non-deterministic Finite Automaton.
--
-- @param states table Array of state name strings. Must be non-empty.
-- @param alphabet table Array of input symbol strings. Must not contain "".
-- @param transitions table Mapping from {state, event_or_EPSILON} arrays
--   to arrays of target state strings.
-- @param initial string The starting state. Must be in states.
-- @param accepting table Array of accepting state names.
-- @return NFA A new NFA instance.
function NFA.new(states, alphabet, transitions, initial, accepting)
    if #states == 0 then
        error("statemachine: states set must be non-empty")
    end

    local state_set = make_set(states)

    local alpha_set = {}
    for _, a in ipairs(alphabet) do
        if a == EPSILON then
            error("statemachine: alphabet must not contain the empty string (reserved for epsilon)")
        end
        alpha_set[a] = true
    end

    if not state_set[initial] then
        error(string.format(
            "statemachine: initial state %q is not in the states set",
            initial
        ))
    end

    local accept_set = {}
    for _, a in ipairs(accepting) do
        if not state_set[a] then
            error(string.format(
                "statemachine: accepting state %q is not in the states set",
                a
            ))
        end
        accept_set[a] = true
    end

    -- Validate and copy transitions
    local trans = {}
    for key, targets in pairs(transitions) do
        local source, event = key[1], key[2]
        if not state_set[source] then
            error(string.format(
                "statemachine: transition source %q is not in the states set",
                source
            ))
        end
        if event ~= EPSILON and not alpha_set[event] then
            error(string.format(
                "statemachine: transition event %q is not in the alphabet and is not epsilon",
                event
            ))
        end
        for _, t in ipairs(targets) do
            if not state_set[t] then
                error(string.format(
                    "statemachine: transition target %q (from (%s, %q)) is not in the states set",
                    t, source, event
                ))
            end
        end
        trans[trans_key(source, event)] = array_copy(targets)
    end

    -- Build internal graph
    local graph = LabeledGraph.new_allow_self_loops()
    for s in pairs(state_set) do
        graph:add_node(s)
    end
    for key, targets in pairs(transitions) do
        local source, event = key[1], key[2]
        for _, target in ipairs(targets) do
            graph:add_edge(source, target, event)
        end
    end

    local self = setmetatable({}, NFA)
    self._states = state_set
    self._alphabet = alpha_set
    self._transitions = trans
    self._initial = initial
    self._accepting = accept_set
    self._graph = graph

    -- Start in the epsilon closure of the initial state
    self._current = self:epsilon_closure({ [initial] = true })

    return self
end

-- =========================================================================
-- NFA Getters
-- =========================================================================

--- Return a sorted array of all state names.
function NFA:states() return sorted_keys(self._states) end

--- Return a sorted array of all input symbols.
function NFA:alphabet() return sorted_keys(self._alphabet) end

--- Return the initial state name.
function NFA:initial() return self._initial end

--- Return a sorted array of accepting state names.
function NFA:accepting() return sorted_keys(self._accepting) end

--- Return a copy of the current active state set.
-- @return table A set (table with boolean values) of active states.
function NFA:current_states()
    return shallow_copy(self._current)
end

-- =========================================================================
-- Epsilon Closure
-- =========================================================================

--- Compute the epsilon closure of a set of states.
--
-- Starting from the given states, follow ALL epsilon transitions recursively.
-- Return the full set of states reachable via zero or more epsilon transitions.
--
-- The algorithm is BFS over epsilon edges:
--
--  1. Start with the input set
--  2. For each state, find epsilon transitions
--  3. Add all targets to the set
--  4. Repeat until no new states are found
--
-- @param state_set table A set of states (table with boolean values).
-- @return table The epsilon closure set.
function NFA:epsilon_closure(state_set)
    local closure = {}
    local worklist = {}
    for s in pairs(state_set) do
        closure[s] = true
        worklist[#worklist + 1] = s
    end

    while #worklist > 0 do
        local state = worklist[#worklist]
        worklist[#worklist] = nil

        local targets = self._transitions[trans_key(state, EPSILON)]
        if targets then
            for _, target in ipairs(targets) do
                if not closure[target] then
                    closure[target] = true
                    worklist[#worklist + 1] = target
                end
            end
        end
    end

    return closure
end

-- =========================================================================
-- NFA Processing
-- =========================================================================

--- Process one input event and return the new set of states.
--
-- For each current state, find all transitions on this event. Take the
-- union of all target states, then compute the epsilon closure.
--
-- Raises an error if the event is not in the alphabet.
--
-- @param event string The input event.
-- @return table A set of the new active states.
function NFA:process(event)
    if not self._alphabet[event] then
        error(string.format(
            "statemachine: event %q is not in the alphabet",
            event
        ))
    end

    local next_states = {}
    for state in pairs(self._current) do
        local targets = self._transitions[trans_key(state, event)]
        if targets then
            for _, t in ipairs(targets) do
                next_states[t] = true
            end
        end
    end

    self._current = self:epsilon_closure(next_states)
    return self:current_states()
end

--- Check if the NFA accepts the input sequence.
--
-- Does NOT modify the NFA's current state — runs on a simulation copy.
--
-- @param events table Array of event strings.
-- @return boolean True if the sequence is accepted.
function NFA:accepts(events)
    local current = self:epsilon_closure({ [self._initial] = true })

    for _, event in ipairs(events) do
        if not self._alphabet[event] then
            error(string.format(
                "statemachine: event %q is not in the alphabet",
                event
            ))
        end

        local next_states = {}
        for state in pairs(current) do
            local targets = self._transitions[trans_key(state, event)]
            if targets then
                for _, t in ipairs(targets) do
                    next_states[t] = true
                end
            end
        end

        current = self:epsilon_closure(next_states)

        -- If no states are active, the NFA is dead — reject early
        local has_any = false
        for _ in pairs(current) do
            has_any = true
            break
        end
        if not has_any then
            return false
        end
    end

    for s in pairs(current) do
        if self._accepting[s] then
            return true
        end
    end
    return false
end

--- Reset the NFA to its initial state (with epsilon closure).
function NFA:reset()
    self._current = self:epsilon_closure({ [self._initial] = true })
end

-- =========================================================================
-- NFA to DFA Conversion — Subset Construction
-- =========================================================================

--- Convert this NFA to an equivalent DFA using subset construction.
--
-- The key insight: if an NFA can be in states {q0, q1, q3} simultaneously,
-- we create a single DFA state representing that entire set. The DFA's
-- states are sets of NFA states.
--
-- Algorithm:
--  1. Start with d0 = epsilon-closure({q0})
--  2. For each DFA state D and each input symbol a:
--     - For each NFA state q in D, find delta(q, a)
--     - Take the union of all targets
--     - Compute epsilon-closure of the union
--     - That is the new DFA state D'
--  3. Repeat until no new DFA states are discovered
--  4. A DFA state is accepting if it contains ANY NFA accepting state
--
-- @return DFA An equivalent DFA.
function NFA:to_dfa()
    -- Step 1: initial DFA state = epsilon-closure of NFA initial state
    local start_closure = self:epsilon_closure({ [self._initial] = true })
    local dfa_start = "{" .. table.concat(sorted_keys(start_closure), ",") .. "}"

    -- Track DFA states and transitions as we discover them
    local dfa_states = { [dfa_start] = true }
    local dfa_transitions = {}
    local dfa_accepting = {}

    -- Map from DFA state name -> set of NFA states
    local state_map = { [dfa_start] = start_closure }

    -- Check if start state is accepting
    if sets_intersect(start_closure, self._accepting) then
        dfa_accepting[dfa_start] = true
    end

    -- Step 2-3: BFS over DFA states
    local worklist = { dfa_start }
    local sorted_alpha = sorted_keys(self._alphabet)

    while #worklist > 0 do
        local current_name = worklist[1]
        table.remove(worklist, 1)
        local current_nfa_states = state_map[current_name]

        for _, event in ipairs(sorted_alpha) do
            -- Collect all NFA states reachable via this event
            local next_nfa = {}
            for nfa_state in pairs(current_nfa_states) do
                local targets = self._transitions[trans_key(nfa_state, event)]
                if targets then
                    for _, t in ipairs(targets) do
                        next_nfa[t] = true
                    end
                end
            end

            -- Epsilon closure of the result
            local next_closure = self:epsilon_closure(next_nfa)

            local has_any = false
            for _ in pairs(next_closure) do
                has_any = true
                break
            end
            if not has_any then
                goto continue
            end

            local next_name = "{" .. table.concat(sorted_keys(next_closure), ",") .. "}"

            -- Record this DFA transition
            dfa_transitions[#dfa_transitions + 1] = {current_name, event, next_name}

            -- If this is a new DFA state, add it
            if not dfa_states[next_name] then
                dfa_states[next_name] = true
                state_map[next_name] = next_closure
                worklist[#worklist + 1] = next_name

                if sets_intersect(next_closure, self._accepting) then
                    dfa_accepting[next_name] = true
                end
            end

            ::continue::
        end
    end

    -- Convert to DFA.new format
    local trans_map = {}
    for _, t in ipairs(dfa_transitions) do
        trans_map[{t[1], t[2]}] = t[3]
    end

    return DFA.new(
        sorted_keys(dfa_states),
        sorted_keys(self._alphabet),
        trans_map,
        dfa_start,
        sorted_keys(dfa_accepting),
        nil
    )
end

--- Return a Graphviz DOT representation of this NFA.
--
-- Epsilon transitions are labeled with the epsilon Unicode character.
--
-- @return string A DOT language string.
function NFA:to_dot()
    local lines = {}
    lines[#lines + 1] = "digraph NFA {"
    lines[#lines + 1] = "    rankdir=LR;"
    lines[#lines + 1] = ""

    lines[#lines + 1] = "    __start [shape=point, width=0.2];"
    lines[#lines + 1] = string.format('    __start -> %q;', self._initial)
    lines[#lines + 1] = ""

    for _, state in ipairs(sorted_keys(self._states)) do
        local shape = "circle"
        if self._accepting[state] then
            shape = "doublecircle"
        end
        lines[#lines + 1] = string.format('    %q [shape=%s];', state, shape)
    end
    lines[#lines + 1] = ""

    -- Collect edges: group by (source, target)
    local edge_labels = {}
    for key, targets in pairs(self._transitions) do
        local sep = key:find("\0", 1, true)
        local source = key:sub(1, sep - 1)
        local event = key:sub(sep + 1)
        local label = event == EPSILON and "\u{03b5}" or event
        for _, target in ipairs(targets) do
            local ek = source .. "\0" .. target
            if not edge_labels[ek] then
                edge_labels[ek] = {}
            end
            edge_labels[ek][#edge_labels[ek] + 1] = label
        end
    end

    local ekeys = {}
    for ek in pairs(edge_labels) do
        ekeys[#ekeys + 1] = ek
    end
    table.sort(ekeys)

    for _, ek in ipairs(ekeys) do
        local sep = ek:find("\0", 1, true)
        local source = ek:sub(1, sep - 1)
        local target = ek:sub(sep + 1)
        local labels = edge_labels[ek]
        table.sort(labels)
        local label = table.concat(labels, ", ")
        lines[#lines + 1] = string.format('    %q -> %q [label=%q];', source, target, label)
    end

    lines[#lines + 1] = "}"
    return table.concat(lines, "\n")
end


-- =========================================================================
-- PDA — Pushdown Automaton
-- =========================================================================
--
-- # What is a PDA?
--
-- A PDA is a state machine augmented with a stack — an unbounded LIFO
-- (last-in, first-out) data structure. The stack gives the PDA the ability
-- to "remember" things that a finite automaton cannot, like how many open
-- parentheses it has seen.
--
-- This extra memory is exactly what is needed to recognize context-free
-- languages — the class of languages that includes balanced parentheses,
-- nested HTML tags, arithmetic expressions, and most programming language
-- syntax.
--
-- # Formal Definition
--
--   PDA = (Q, Sigma, Gamma, delta, q0, Z0, F)
--
--   Q     = finite set of states
--   Sigma = input alphabet
--   Gamma = stack alphabet (may differ from Sigma)
--   delta = transition function
--   q0    = initial state
--   Z0    = initial stack symbol (bottom marker)
--   F     = accepting states
--
-- Our implementation is deterministic (DPDA): at most one transition
-- applies at any time.

local PDA = {}
PDA.__index = PDA

--- PDA transition key: encodes (state, event_or_nil, stack_top).
--
-- For epsilon transitions, event is nil. We use a sentinel to distinguish
-- nil from the empty string.
--
-- @param state string The current state.
-- @param event string|nil The input event, or nil for epsilon.
-- @param stack_top string The top of the stack.
-- @return string A unique lookup key.
local function pda_key(state, event, stack_top)
    local event_str = event == nil and "\1EPSILON" or event
    return state .. "\0" .. event_str .. "\0" .. stack_top
end

--- Create a PDA trace entry table.
--
-- @param source string Source state.
-- @param event string|nil The triggering event.
-- @param stack_read string The stack symbol that was read.
-- @param target string The target state.
-- @param stack_push table Array of symbols pushed onto the stack.
-- @param stack_after table Array of stack contents after the transition.
-- @return table A PDA trace entry.
local function PDATraceEntry(source, event, stack_read, target, stack_push, stack_after)
    return {
        source = source,
        event = event,
        stack_read = stack_read,
        target = target,
        stack_push = array_copy(stack_push),
        stack_after = array_copy(stack_after),
    }
end

--- Create a new Deterministic Pushdown Automaton.
--
-- @param states table Array of state strings. Must be non-empty.
-- @param input_alphabet table Array of input symbol strings.
-- @param stack_alphabet table Array of stack symbol strings.
-- @param transitions table Array of transition tables, each with fields:
--   - source: string
--   - event: string|nil (nil for epsilon)
--   - stack_read: string
--   - target: string
--   - stack_push: table (array of strings)
-- @param initial string The starting state.
-- @param initial_stack_symbol string Symbol placed on the stack initially.
-- @param accepting table Array of accepting state strings.
-- @return PDA A new PDA instance.
function PDA.new(states, input_alphabet, stack_alphabet, transitions, initial, initial_stack_symbol, accepting)
    if #states == 0 then
        error("statemachine: states set must be non-empty")
    end

    local state_set = make_set(states)

    if not state_set[initial] then
        error(string.format(
            "statemachine: initial state %q is not in the states set",
            initial
        ))
    end

    local input_set = make_set(input_alphabet)
    local stack_set = make_set(stack_alphabet)

    if not stack_set[initial_stack_symbol] then
        error(string.format(
            "statemachine: initial stack symbol %q is not in the stack alphabet",
            initial_stack_symbol
        ))
    end

    local accept_set = {}
    for _, a in ipairs(accepting) do
        if not state_set[a] then
            error(string.format(
                "statemachine: accepting state %q is not in the states set",
                a
            ))
        end
        accept_set[a] = true
    end

    -- Build transition index
    local index = {}
    local trans_copy = {}
    for i, t in ipairs(transitions) do
        -- Deep copy the transition
        trans_copy[i] = {
            source = t.source,
            event = t.event,
            stack_read = t.stack_read,
            target = t.target,
            stack_push = array_copy(t.stack_push),
        }

        local key = pda_key(t.source, t.event, t.stack_read)
        if index[key] then
            error(string.format(
                "statemachine: duplicate transition for (state=%q, event=%s, stack_top=%q) — this PDA must be deterministic",
                t.source, t.event == nil and "nil" or string.format("%q", t.event), t.stack_read
            ))
        end
        index[key] = trans_copy[i]
    end

    local self = setmetatable({}, PDA)
    self._states = state_set
    self._input_alphabet = input_set
    self._stack_alphabet = stack_set
    self._transitions = trans_copy
    self._initial = initial
    self._initial_stack_sym = initial_stack_symbol
    self._accepting = accept_set
    self._transition_index = index
    self._current = initial
    self._stack = { initial_stack_symbol }
    self._trace = {}
    return self
end

-- =========================================================================
-- PDA Internal helpers
-- =========================================================================

--- Find a matching transition for the current state, event, and stack top.
--
-- @param event string|nil The input event, or nil for epsilon.
-- @return table|nil The matching transition, or nil.
function PDA:_find_transition(event)
    if #self._stack == 0 then
        return nil
    end
    local top = self._stack[#self._stack]
    local key = pda_key(self._current, event, top)
    return self._transition_index[key]
end

--- Apply a transition: change state and modify the stack.
--
-- @param t table The transition to apply.
function PDA:_apply_transition(t)
    -- Pop the stack top (it was "read" by the transition)
    self._stack[#self._stack] = nil

    -- Push new symbols (in order: first element goes deepest)
    for _, sym in ipairs(t.stack_push) do
        self._stack[#self._stack + 1] = sym
    end

    -- Record the trace
    self._trace[#self._trace + 1] = PDATraceEntry(
        t.source, t.event, t.stack_read, t.target,
        t.stack_push, self._stack
    )

    -- Change state
    self._current = t.target
end

--- Try to take an epsilon transition. Returns true if one was taken.
function PDA:_try_epsilon()
    local t = self:_find_transition(nil)
    if t then
        self:_apply_transition(t)
        return true
    end
    return false
end

-- =========================================================================
-- PDA Processing
-- =========================================================================

--- Process one input symbol and return the new current state.
--
-- @param event string The input event.
-- @return string The new current state.
function PDA:process(event)
    local t = self:_find_transition(event)
    if not t then
        local top_str
        if #self._stack > 0 then
            top_str = self._stack[#self._stack]
        else
            top_str = "<empty>"
        end
        error(string.format(
            "statemachine: no PDA transition for (state=%q, event=%q, stack_top=%q)",
            self._current, event, top_str
        ))
    end
    self:_apply_transition(t)
    return self._current
end

--- Process a sequence of inputs and return the trace entries generated.
--
-- After processing all inputs, tries epsilon transitions until none are
-- available (this handles acceptance transitions that fire at end-of-input).
--
-- @param events table Array of event strings.
-- @return table Array of PDA trace entries.
function PDA:process_sequence(events)
    local trace_start = #self._trace
    for _, event in ipairs(events) do
        self:process(event)
    end
    -- Try epsilon transitions at end of input
    while self:_try_epsilon() do end
    local result = {}
    for i = trace_start + 1, #self._trace do
        result[#result + 1] = self._trace[i]
    end
    return result
end

--- Check if the PDA accepts the input sequence.
--
-- Does NOT modify this PDA's state — runs on a simulation copy.
--
-- @param events table Array of event strings.
-- @return boolean True if the sequence is accepted.
function PDA:accepts(events)
    -- Simulate on copies of the mutable state
    local state = self._initial
    local stack = { self._initial_stack_sym }

    for _, event in ipairs(events) do
        if #stack == 0 then
            return false
        end
        local top = stack[#stack]
        local key = pda_key(state, event, top)
        local t = self._transition_index[key]
        if not t then
            return false
        end
        stack[#stack] = nil
        for _, sym in ipairs(t.stack_push) do
            stack[#stack + 1] = sym
        end
        state = t.target
    end

    -- Try epsilon transitions at end of input
    local max_epsilon = #self._transitions + 1
    for _ = 1, max_epsilon do
        if #stack == 0 then
            break
        end
        local top = stack[#stack]
        local key = pda_key(state, nil, top)
        local t = self._transition_index[key]
        if not t then
            break
        end
        stack[#stack] = nil
        for _, sym in ipairs(t.stack_push) do
            stack[#stack + 1] = sym
        end
        state = t.target
    end

    return self._accepting[state] == true
end

--- Reset the PDA to its initial state with the initial stack.
function PDA:reset()
    self._current = self._initial
    self._stack = { self._initial_stack_sym }
    self._trace = {}
end

--- Return the current state of the PDA.
function PDA:current_state()
    return self._current
end

--- Return a copy of the current stack contents (bottom to top).
function PDA:stack()
    return array_copy(self._stack)
end

--- Return the top of the stack, or nil if empty.
function PDA:stack_top()
    if #self._stack == 0 then
        return nil
    end
    return self._stack[#self._stack]
end

--- Return a copy of the PDA execution trace.
function PDA:trace()
    local result = {}
    for i, entry in ipairs(self._trace) do
        result[i] = {
            source = entry.source,
            event = entry.event,
            stack_read = entry.stack_read,
            target = entry.target,
            stack_push = array_copy(entry.stack_push),
            stack_after = array_copy(entry.stack_after),
        }
    end
    return result
end


-- =========================================================================
-- Modal State Machine
-- =========================================================================
--
-- # What is a Modal State Machine?
--
-- A modal state machine is a collection of named sub-machines (modes), each
-- a DFA, with transitions that switch between them. When a mode switch
-- occurs, the active sub-machine changes.
--
-- Think of it like a text editor with Normal, Insert, and Visual modes.
-- Each mode handles keystrokes differently, and certain keys switch modes.
--
-- # Why modal machines matter
--
-- The most important use case is context-sensitive tokenization. Consider
-- HTML: the characters `p > .foo { color: red; }` mean completely different
-- things depending on whether they appear inside a <style> tag (CSS) or
-- in normal text. A single set of token rules cannot handle both contexts.

local ModalStateMachine = {}
ModalStateMachine.__index = ModalStateMachine

--- Create a new Modal State Machine.
--
-- @param modes table A map from mode names (strings) to DFA instances.
-- @param mode_transitions table Mapping from {current_mode, trigger} arrays
--   to target mode strings.
-- @param initial_mode string The name of the starting mode.
-- @return ModalStateMachine A new modal state machine.
function ModalStateMachine.new(modes, mode_transitions, initial_mode)
    if not next(modes) then
        error("statemachine: at least one mode must be provided")
    end
    if not modes[initial_mode] then
        error(string.format(
            "statemachine: initial mode %q is not in the modes map",
            initial_mode
        ))
    end

    -- Validate mode transitions
    local trans_copy = {}
    for key, target in pairs(mode_transitions) do
        local from = key[1]
        if not modes[from] then
            error(string.format(
                "statemachine: mode transition source %q is not a valid mode",
                from
            ))
        end
        if not modes[target] then
            error(string.format(
                "statemachine: mode transition target %q is not a valid mode",
                target
            ))
        end
        trans_copy[trans_key(from, key[2])] = target
    end

    -- Copy modes map
    local modes_copy = {}
    for k, v in pairs(modes) do
        modes_copy[k] = v
    end

    -- Build internal graph of mode transitions
    local mode_graph = LabeledGraph.new_allow_self_loops()
    for mode in pairs(modes_copy) do
        mode_graph:add_node(mode)
    end
    for key, target in pairs(mode_transitions) do
        local from, trigger = key[1], key[2]
        mode_graph:add_edge(from, target, trigger)
    end

    local self = setmetatable({}, ModalStateMachine)
    self._modes = modes_copy
    self._mode_transitions = trans_copy
    self._initial_mode = initial_mode
    self._mode_graph = mode_graph
    self._current_mode = initial_mode
    self._mode_trace = {}
    return self
end

-- =========================================================================
-- Modal Getters
-- =========================================================================

--- Return the name of the currently active mode.
function ModalStateMachine:current_mode()
    return self._current_mode
end

--- Return the DFA for the current mode.
function ModalStateMachine:active_machine()
    return self._modes[self._current_mode]
end

--- Return a copy of the mode switch history.
-- @return table Array of {from_mode, trigger, to_mode} tables.
function ModalStateMachine:mode_trace()
    local result = {}
    for i, rec in ipairs(self._mode_trace) do
        result[i] = {
            from_mode = rec.from_mode,
            trigger = rec.trigger,
            to_mode = rec.to_mode,
        }
    end
    return result
end

--- Return a sorted array of all mode names.
function ModalStateMachine:modes()
    local names = {}
    for k in pairs(self._modes) do
        names[#names + 1] = k
    end
    table.sort(names)
    return names
end

-- =========================================================================
-- Modal Processing
-- =========================================================================

--- Switch to a different mode based on a trigger event.
--
-- Looks up (current_mode, trigger) in the mode transitions. If found,
-- switches to the target mode and resets its DFA to the initial state.
-- Returns the name of the new mode.
--
-- Raises an error if no mode transition exists for this trigger.
--
-- @param trigger string The trigger event.
-- @return string The new mode name.
function ModalStateMachine:switch_mode(trigger)
    local key = trans_key(self._current_mode, trigger)
    local new_mode = self._mode_transitions[key]
    if not new_mode then
        error(string.format(
            "statemachine: no mode transition for (mode=%q, trigger=%q)",
            self._current_mode, trigger
        ))
    end

    local old_mode = self._current_mode

    -- Reset the target mode's DFA to its initial state
    self._modes[new_mode]:reset()

    -- Record the switch
    self._mode_trace[#self._mode_trace + 1] = {
        from_mode = old_mode,
        trigger = trigger,
        to_mode = new_mode,
    }

    self._current_mode = new_mode
    return new_mode
end

--- Process an input event in the current mode's DFA.
--
-- Delegates to the active DFA's process() method.
--
-- @param event string The input event.
-- @return string The new state of the active DFA.
function ModalStateMachine:process(event)
    return self._modes[self._current_mode]:process(event)
end

--- Reset to the initial mode and reset all sub-machines.
function ModalStateMachine:reset()
    self._current_mode = self._initial_mode
    self._mode_trace = {}
    for _, dfa in pairs(self._modes) do
        dfa:reset()
    end
end


-- =========================================================================
-- DFA Minimization — Hopcroft's Algorithm
-- =========================================================================
--
-- # What is DFA minimization?
--
-- Two DFA states are "equivalent" if, for every possible input sequence,
-- they either both lead to acceptance or both lead to rejection. Equivalent
-- states can be merged without changing the language the DFA recognizes.
--
-- DFA minimization finds and merges all equivalent states, producing the
-- smallest possible DFA for a given regular language.
--
-- # Hopcroft's Algorithm
--
-- The algorithm works by partition refinement:
--
-- 1. Start with two groups: accepting states and non-accepting states.
-- 2. For each group and each input symbol, check: do all states in the
--    group go to the same group on that input? If not, split the group.
-- 3. Repeat until no group can be split.
-- 4. Each final group becomes one state in the minimized DFA.

--- Split a group of states based on transition targets.
--
-- Two states in the same group are equivalent only if, for every input
-- symbol, they transition to states in the same partition.
--
-- @param group table A set of states.
-- @param alphabet table Array of input symbols.
-- @param transitions table Transition map (trans_key -> target).
-- @param partitions table Array of partition sets.
-- @return table Array of subgroups.
local function split_group(group, alphabet_arr, transitions, partitions)
    -- Count members
    local count = 0
    for _ in pairs(group) do count = count + 1 end
    if count <= 1 then
        return { group }
    end

    -- Build state -> partition index lookup
    local state_to_partition = {}
    for idx, partition in ipairs(partitions) do
        for state in pairs(partition) do
            state_to_partition[state] = idx
        end
    end

    -- For each input symbol, compute a "signature" for each state
    for _, event in ipairs(alphabet_arr) do
        local signatures = {}  -- sig -> set of states
        for state in pairs(group) do
            local key = trans_key(state, event)
            local sig = -1
            local target = transitions[key]
            if target then
                sig = state_to_partition[target] or -1
            end
            if not signatures[sig] then
                signatures[sig] = {}
            end
            signatures[sig][state] = true
        end

        -- Check if we need to split
        local sig_count = 0
        for _ in pairs(signatures) do sig_count = sig_count + 1 end

        if sig_count > 1 then
            -- Split needed! Return the subgroups.
            local sig_keys = {}
            for k in pairs(signatures) do
                sig_keys[#sig_keys + 1] = k
            end
            table.sort(sig_keys)
            local result = {}
            for _, k in ipairs(sig_keys) do
                result[#result + 1] = signatures[k]
            end
            return result
        end
    end

    -- No split needed
    return { group }
end

--- Produce the minimal DFA equivalent to the given DFA.
--
-- Removes unreachable states first, then uses Hopcroft's partition
-- refinement algorithm to merge equivalent states.
--
-- @param dfa DFA The DFA to minimize.
-- @return DFA The minimized DFA.
local function Minimize(dfa)
    -- Step 0: Remove unreachable states
    local reachable = dfa:reachable_states()

    -- Filter transitions to only reachable states
    local transitions = {}
    for key, target in pairs(dfa._transitions) do
        local sep = key:find("\0", 1, true)
        local source = key:sub(1, sep - 1)
        if reachable[source] and reachable[target] then
            transitions[key] = target
        end
    end

    -- Step 1: Initial partition — accepting vs non-accepting
    local accepting_part = {}
    local non_accepting = {}
    for s in pairs(reachable) do
        if dfa._accepting[s] then
            accepting_part[s] = true
        else
            non_accepting[s] = true
        end
    end

    local partitions = {}
    local has_accepting = false
    for _ in pairs(accepting_part) do has_accepting = true; break end
    local has_non_accepting = false
    for _ in pairs(non_accepting) do has_non_accepting = true; break end

    if has_accepting then
        partitions[#partitions + 1] = accepting_part
    end
    if has_non_accepting then
        partitions[#partitions + 1] = non_accepting
    end

    if #partitions == 0 then
        return dfa
    end

    -- Step 2-3: Iteratively refine partitions
    local alphabet_arr = sorted_keys(dfa._alphabet)

    while true do
        local changed = false
        local new_partitions = {}

        for _, group in ipairs(partitions) do
            local splits = split_group(group, alphabet_arr, transitions, partitions)
            if #splits > 1 then
                changed = true
            end
            for _, s in ipairs(splits) do
                new_partitions[#new_partitions + 1] = s
            end
        end

        partitions = new_partitions
        if not changed then
            break
        end
    end

    -- Step 4: Build the minimized DFA
    local state_to_partition = {}
    for idx, partition in ipairs(partitions) do
        for state in pairs(partition) do
            state_to_partition[state] = idx
        end
    end

    local function partition_name(partition)
        local members = sorted_keys(partition)
        if #members == 1 then
            return members[1]
        end
        return "{" .. table.concat(members, ",") .. "}"
    end

    local new_states = {}
    local new_transitions = {}
    local new_accepting_set = {}

    for _, partition in ipairs(partitions) do
        local name = partition_name(partition)
        new_states[name] = true

        if sets_intersect(partition, accepting_part) then
            new_accepting_set[name] = true
        end

        -- Use any representative state from the partition
        local representative = sorted_keys(partition)[1]
        for _, event in ipairs(alphabet_arr) do
            local key = trans_key(representative, event)
            local target = transitions[key]
            if target then
                local target_idx = state_to_partition[target]
                local target_name = partition_name(partitions[target_idx])
                new_transitions[{name, event}] = target_name
            end
        end
    end

    -- Find the new initial state
    local initial_idx = state_to_partition[dfa._initial]
    local new_initial = partition_name(partitions[initial_idx])

    return DFA.new(
        sorted_keys(new_states),
        alphabet_arr,
        new_transitions,
        new_initial,
        sorted_keys(new_accepting_set),
        nil
    )
end


-- =========================================================================
-- Module export
-- =========================================================================

local state_machine = {}

state_machine.VERSION = "0.1.0"

-- Classes
state_machine.DFA = DFA
state_machine.NFA = NFA
state_machine.PDA = PDA
state_machine.ModalStateMachine = ModalStateMachine

-- Functions
state_machine.Minimize = Minimize

-- Constants
state_machine.EPSILON = EPSILON

-- Helper constructors (exported for tests)
state_machine.TransitionRecord = TransitionRecord

return state_machine
