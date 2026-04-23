-- markov_chain — General-purpose Markov Chain data structure
-- ===========================================================
--
-- A Markov Chain is a mathematical model of a system that moves between
-- a finite set of STATES over time. The central insight — the "Markov
-- property" — is that the probability of transitioning to the next state
-- depends ONLY on the current state, not on any history of how the system
-- got there.
--
-- Imagine a simple weather model:
--
--   States: {Sunny, Cloudy, Rainy}
--
--   If today is Cloudy, tomorrow will be:
--     Sunny  with probability 0.3
--     Cloudy with probability 0.4
--     Rainy  with probability 0.3
--
-- That's it — the chain doesn't care whether it was Sunny last week.
-- Only the current state (Cloudy) determines tomorrow's distribution.
--
-- # Training
--
-- We learn probabilities by counting transitions in observed sequences:
--
--   Observations: [A, B, A, C, A, B, B, A]
--   Count: A→B: 2, A→C: 1, B→A: 2, B→B: 1, C→A: 1
--   Normalize rows: P(A→B) = 2/3,  P(A→C) = 1/3
--                   P(B→A) = 2/3,  P(B→B) = 1/3
--                   P(C→A) = 1/1
--
-- # Smoothing
--
-- If a transition was never observed, its probability is 0. That means
-- the chain can get "stuck" in a state if there are no known outgoing
-- transitions. Laplace (add-α) smoothing prevents this by pretending
-- every transition was observed α times:
--
--   Smoothed count(i→j) = raw_count(i→j) + α
--   Smoothed P(i→j) = (raw_count(i→j) + α) / (total_raw + α * |states|)
--
-- # Order-k Chains
--
-- A standard (order-1) chain conditions on only the last 1 state.
-- An order-k chain conditions on the last k states (a "k-gram context"):
--
--   P(next | last k states) = T["s_{n-k}…s_{n-1}"][s_n]
--
-- For text generation this gives much more realistic output:
--   Order 1: random-looking character soup
--   Order 2: word fragments (digram statistics)
--   Order 3: nearly verbatim reproductions of training text
--
-- We store contexts as null-separated strings:
--   context = table.concat({seq[i], seq[i+1], ..., seq[i+order-1]}, "\0")
--
-- # Topology via DirectedGraph
--
-- The set of observed transitions forms a directed graph: each state is
-- a node and each observed transition is an edge. We use the DirectedGraph
-- package for topology while storing actual probabilities in a separate
-- `transitions` map.
--
-- # Sampling / Generation
--
-- To generate a sequence of length n starting from state s:
--
--   1. Start with s (count it as the first element).
--   2. For each subsequent element, sample from T[current_context].
--   3. Sampling: pick random r in [0,1]; walk cumulative sum of probabilities
--      until r < cumulative; return that state.
--
-- # Stationary Distribution
--
-- For an ergodic chain (all states mutually reachable, no periodic traps),
-- there exists a unique stationary distribution π such that:
--
--   π · T = π
--
-- Interpretation: "In the long run, fraction π[s] of time is spent in state s."
-- We compute it via power iteration: start with a uniform distribution and
-- repeatedly multiply by T until convergence (change < 1e-10).
--
-- # OOP Pattern
--
-- We follow the coding-adventures standard Lua OOP pattern:
--
--   local MarkovChain = {}
--   MarkovChain.__index = MarkovChain
--   function MarkovChain.new(...) ... end
--
-- Methods are called with the : operator: chain:train(sequence)

-- Seed the random number generator once at module load time so that
-- generate() and next_state() produce different sequences each run.
-- In a reproducible test environment the caller can re-seed with a
-- fixed value before calling generate_string.
math.randomseed(os.time())

-- Require the directed graph package for topology tracking.
-- This tells us which states exist and which transitions have been observed,
-- without storing probabilities there (that lives in self._transitions).
local dg = require("coding_adventures.directed_graph")
local DirectedGraph = dg.DirectedGraph

-- =========================================================================
-- MarkovChain
-- =========================================================================
--
-- Internal representation:
--
--   _order       int                         — k (context window size)
--   _smoothing   float                       — Laplace/Lidstone α ≥ 0
--   _states      array of strings            — ordered unique state list
--   _state_set   {state → true}              — fast membership test
--   _graph       DirectedGraph               — tracks which transitions exist
--   _counts      {context → {state → int}}   — raw transition counts
--   _transitions {context → {state → float}} — normalised probabilities
--
-- For order-1 chains "context" is just the state string itself.
-- For order-k chains "context" is table.concat(k-gram, "\0").
--
-- The _graph is created with self-loops allowed because a state may
-- transition to itself (e.g., Markov chain on weather: Cloudy→Cloudy).
-- In topology terms that is a valid self-loop.

local MarkovChain = {}
MarkovChain.__index = MarkovChain

-- =========================================================================
-- Constructor
-- =========================================================================

--- Create a new, empty MarkovChain.
--
-- @param order    int    Context window size (default 1). Order 1 means
--                        "next state depends only on the current state."
--                        Order k means "next state depends on the last k states."
-- @param smoothing float Laplace/Lidstone smoothing coefficient α (default 0.0).
--                        0.0 = no smoothing (zero-probability transitions stay zero).
--                        1.0 = Laplace smoothing (every transition gets count +1).
--                        α > 0 = Lidstone smoothing.
-- @param states   table  Optional list of state names to pre-register as the
--                        alphabet. Pre-registering states is required when you
--                        want smoothing to cover transitions to states that may
--                        not appear in training data.
-- @return MarkovChain A new empty chain.
function MarkovChain.new(order, smoothing, states)
    -- Apply defaults. In Lua, unspecified arguments arrive as nil.
    order = order or 1
    smoothing = smoothing or 0.0

    local self = setmetatable({}, MarkovChain)

    self._order = order
    self._smoothing = smoothing

    -- The state registry. _states is a deterministically ordered array;
    -- _state_set is a hash for O(1) membership tests.
    self._states = {}
    self._state_set = {}

    -- The topology graph. Self-loops allowed because a state can
    -- transition to itself (e.g., weather: Rainy → Rainy).
    self._graph = DirectedGraph.new_allow_self_loops()

    -- Raw transition counts: _counts[context][target] = integer.
    -- Built up during train() calls; normalised into _transitions.
    self._counts = {}

    -- Normalised transition probabilities: _transitions[context][target] = float.
    -- Rebuilt from _counts whenever train() is called.
    self._transitions = {}

    -- Pre-register any states the caller provides.
    if states then
        for _, s in ipairs(states) do
            self:_register_state(s)
        end
    end

    return self
end

-- =========================================================================
-- Internal helpers
-- =========================================================================

--- Register a state in the alphabet (idempotent).
--
-- States must be registered before they can participate in transitions.
-- Training automatically registers any state it encounters, but pre-registering
-- states is necessary when smoothing must cover transitions to states that
-- do not appear in the training sequence.
--
-- @param state string The state to register.
function MarkovChain:_register_state(state)
    if not self._state_set[state] then
        self._state_set[state] = true
        self._states[#self._states + 1] = state
        -- Add to the topology graph so it appears in has_node queries.
        self._graph:add_node(state)
    end
end

--- Compute the context key for a given array slice.
--
-- For order-1 chains: the key is just the single state string.
-- For order-k chains: the key is the k states joined by the null byte "\0".
-- Null byte is chosen because it cannot appear in normal string state names,
-- making collisions impossible for typical use cases.
--
-- @param seq   table  The full sequence array.
-- @param start int    1-based starting index of the window.
-- @return string The context key.
local function context_key(seq, start, order)
    if order == 1 then
        -- Fast path for the common case.
        return seq[start]
    end
    local parts = {}
    for i = start, start + order - 1 do
        parts[#parts + 1] = seq[i]
    end
    return table.concat(parts, "\0")
end

--- Renormalise all transition rows from raw counts.
--
-- Called at the end of every train() call. Iterates over every context
-- in _counts and computes smoothed probabilities.
--
-- Smoothing formula (Lidstone / add-α):
--
--   P(context → target) = (count(context → target) + α)
--                         / (sum_k(count(context → k)) + α * |states|)
--
-- When α = 0 this reduces to simple maximum-likelihood estimation.
-- When α = 1 this is classic Laplace smoothing.
--
-- All states in the alphabet are considered as potential targets so that
-- smoothing properly spreads mass to unobserved transitions.
function MarkovChain:_normalise()
    local alpha = self._smoothing
    local n_states = #self._states

    self._transitions = {}

    for ctx, target_counts in pairs(self._counts) do
        -- Compute the total raw count for this context.
        local total_raw = 0
        for _, cnt in pairs(target_counts) do
            total_raw = total_raw + cnt
        end

        -- Total denominator with smoothing applied to all states.
        local denom = total_raw + alpha * n_states

        local row = {}
        for _, s in ipairs(self._states) do
            local raw = target_counts[s] or 0
            row[s] = (raw + alpha) / denom
        end
        self._transitions[ctx] = row
    end
end

--- Sample one state from a probability row using the inverse-CDF method.
--
-- The inverse-CDF (also called the roulette-wheel or categorical sampling)
-- works by:
--   1. Draw a uniform random number r in [0, 1).
--   2. Walk through each (state, probability) pair in the row,
--      accumulating a running sum.
--   3. As soon as the running sum exceeds r, return that state.
--
-- This is mathematically equivalent to carving the [0,1] interval into
-- segments proportional to the probabilities and asking "which segment
-- does r fall into?"
--
-- We iterate over self._states (in registration order) to guarantee
-- deterministic behaviour given the same random seed — Lua's pairs()
-- iteration order over tables is not guaranteed.
--
-- @param row {state → float} A probability distribution.
-- @return string The sampled state.
function MarkovChain:_sample(row)
    local r = math.random()
    local cumulative = 0.0
    -- Iterate in a fixed order (registration order) for reproducibility.
    for _, s in ipairs(self._states) do
        local p = row[s]
        if p then
            cumulative = cumulative + p
            if r < cumulative then
                return s
            end
        end
    end
    -- Floating-point rounding can leave cumulative just below 1.0 after
    -- summing all probabilities. Return the last state as a fallback.
    return self._states[#self._states]
end

-- =========================================================================
-- Training
-- =========================================================================

--- Train the chain on a sequence of states.
--
-- Slides a window of size (order + 1) over the sequence. For each window:
--   - The first `order` elements form the context key.
--   - The last element is the target state.
--   - We increment count[context][target].
--
-- After all windows are processed, all transition rows are renormalised
-- from counts to probabilities (including smoothing). Multiple calls to
-- train() accumulate counts before renormalising, so the resulting
-- probabilities reflect the combined training data.
--
-- Example (order=1):
--   train({"A", "B", "A", "C"})
--   Windows: [A,B], [B,A], [A,C]
--   Counts: A→B:1, B→A:1, A→C:1
--   After normalise: P(A→B)=0.5, P(A→C)=0.5, P(B→A)=1.0
--
-- @param sequence table An array of state strings.
function MarkovChain:train(sequence)
    -- Register every state we encounter so the alphabet is complete.
    for _, s in ipairs(sequence) do
        self:_register_state(s)
    end

    -- Slide the window. We need at least (order+1) elements to form
    -- one transition. Lua uses 1-based indexing.
    local n = #sequence
    for i = 1, n - self._order do
        local ctx = context_key(sequence, i, self._order)
        local target = sequence[i + self._order]

        -- Initialise the count row for this context if needed.
        if not self._counts[ctx] then
            self._counts[ctx] = {}
        end

        -- Increment the transition count.
        local old = self._counts[ctx][target] or 0
        self._counts[ctx][target] = old + 1

        -- Record the edge in the topology graph.
        -- For order-1 chains the context IS the state name.
        -- For order-k chains we only add an edge for the LAST state in
        -- the context to the target, so the graph shows reachability.
        local from_state = sequence[i + self._order - 1]
        if not self._graph:has_edge(from_state, target) then
            self._graph:add_edge(from_state, target)
        end
    end

    -- Renormalise after every training call so that probabilities are
    -- always up to date and reflect accumulated counts.
    self:_normalise()
end

--- Train the chain on a plain string, treating each character as a state.
--
-- Convenience wrapper around train(). Converts the string to an array
-- of single-character strings, then delegates to train().
--
-- Example:
--   chain:train_string("abcabc")
--   -- equivalent to: chain:train({"a","b","c","a","b","c"})
--
-- @param text string The training text.
function MarkovChain:train_string(text)
    local seq = {}
    for i = 1, #text do
        seq[#seq + 1] = text:sub(i, i)
    end
    self:train(seq)
end

-- =========================================================================
-- Querying
-- =========================================================================

--- Sample the next state given the current context.
--
-- For an order-1 chain, `current` is a single state string.
-- For an order-k chain, `current` is a string of k states joined by "\0"
-- (i.e., the context key as produced by context_key()).
--
-- Raises an error (via Lua's error()) if `current` is not a known context.
-- This matches the spec requirement: unknown states are a programming error,
-- not a recoverable condition.
--
-- @param current string The current state (or context key for order > 1).
-- @return string The sampled next state.
function MarkovChain:next_state(current)
    local row = self._transitions[current]
    if not row then
        error("Unknown state: " .. tostring(current))
    end
    return self:_sample(row)
end

--- Generate a sequence of exactly `length` states starting from `start`.
--
-- For an order-1 chain:
--   - start is a single state string.
--   - The generated list begins with start and grows by sampling.
--
-- For an order-k chain:
--   - start must be a string containing at least `order` states separated
--     by "\0" (e.g., "a\0b" for order 2). In practice callers use
--     generate_string() for character chains.
--   - The first `order` characters of the returned list are the seed states.
--
-- The result always has exactly `length` elements.
--
-- @param start  string  The starting state (or "\0"-joined context for order>1).
-- @param length int     Desired output length (number of states).
-- @return table An array of exactly `length` state strings.
function MarkovChain:generate(start, length)
    -- For order-1: result begins with the single start state.
    -- For order-k: result begins with the k seed states from start.
    local result = {}

    if self._order == 1 then
        -- Simple case: start is a single state.
        result[1] = start
        local ctx = start
        while #result < length do
            local next = self:next_state(ctx)
            result[#result + 1] = next
            ctx = next
        end
    else
        -- Order-k case: start is a "\0"-separated k-gram.
        -- Split the context back into individual states to seed the result.
        local seed_states = {}
        for part in (start .. "\0"):gmatch("([^\0]*)\0") do
            seed_states[#seed_states + 1] = part
        end
        -- Populate the result with seed states (up to `length` or `order`).
        for _, s in ipairs(seed_states) do
            if #result < length then
                result[#result + 1] = s
            end
        end
        -- Slide the context window forward.
        -- The window always contains the last `order` states in result.
        while #result < length do
            -- Build the current context from the last `order` states.
            local ctx_parts = {}
            for i = #result - self._order + 1, #result do
                ctx_parts[#ctx_parts + 1] = result[i]
            end
            local ctx = table.concat(ctx_parts, "\0")
            local next = self:next_state(ctx)
            result[#result + 1] = next
        end
    end

    return result
end

--- Generate a string of exactly `length` characters starting from `seed`.
--
-- Convenience wrapper for character-level chains. The seed must contain
-- at least `order` characters. The output always has exactly `length`
-- characters.
--
-- Example (order=2):
--   chain:train_string("abcabcabc")
--   chain:generate_string("ab", 9)  -- returns "abcabcabc"
--
-- How it works:
--   1. Convert seed characters into a "\0"-joined context key.
--   2. Call generate() to produce a list of characters.
--   3. Concatenate the list into a string.
--
-- @param seed   string The starting character(s). Length >= order.
-- @param length int    Desired output length in characters.
-- @return string A string of exactly `length` characters.
function MarkovChain:generate_string(seed, length)
    -- Split seed into individual character states.
    local seed_states = {}
    for i = 1, #seed do
        seed_states[#seed_states + 1] = seed:sub(i, i)
    end

    -- For order-1 chains the start is just the first character.
    -- For order-k chains build the "\0"-joined context key from the
    -- last `order` characters of the seed.
    local start
    if self._order == 1 then
        start = seed_states[1]
    else
        -- Use exactly the last `order` characters of the seed as context.
        local ctx_parts = {}
        local seed_len = #seed_states
        for i = seed_len - self._order + 1, seed_len do
            ctx_parts[#ctx_parts + 1] = seed_states[i]
        end
        start = table.concat(ctx_parts, "\0")
    end

    -- For order-k chains we need to pre-populate result with seed chars
    -- before the order-k context window, then generate the rest.
    if self._order > 1 then
        -- Build result by starting from the full seed and generating forward.
        local result = {}
        for _, c in ipairs(seed_states) do
            result[#result + 1] = c
        end
        -- If seed is longer than order, trim to the last `order` chars for
        -- the sliding window but keep all seed chars in output.
        while #result < length do
            local ctx_parts = {}
            for i = #result - self._order + 1, #result do
                ctx_parts[#ctx_parts + 1] = result[i]
            end
            local ctx = table.concat(ctx_parts, "\0")
            local next = self:next_state(ctx)
            result[#result + 1] = next
        end
        -- Trim or return exactly length characters.
        local out = {}
        for i = 1, length do
            out[#out + 1] = result[i]
        end
        return table.concat(out)
    else
        -- Order-1 fast path.
        local char_list = self:generate(start, length)
        return table.concat(char_list)
    end
end

--- Return the transition probability from `from` to `to`.
--
-- For order-1 chains, `from` is a state string.
-- For order-k chains, `from` is the "\0"-joined context key.
--
-- Returns 0.0 if the transition was never observed and smoothing is 0.
-- Returns a small positive value if smoothing is active.
--
-- This method never raises — an unknown from state simply returns 0.0.
--
-- @param from string The source state / context key.
-- @param to   string The target state.
-- @return float The transition probability in [0, 1].
function MarkovChain:probability(from, to)
    local row = self._transitions[from]
    if not row then
        return 0.0
    end
    return row[to] or 0.0
end

-- =========================================================================
-- Stationary distribution — power iteration
-- =========================================================================

--- Compute the stationary distribution via power iteration.
--
-- The stationary distribution π is the left eigenvector of the transition
-- matrix T with eigenvalue 1, satisfying:
--
--   π · T = π   and   ∑ π[s] = 1
--
-- Interpretation: in the long run, the chain spends fraction π[s] of time
-- in state s (if the chain is ergodic — all states mutually reachable).
--
-- Algorithm (power iteration):
--   1. Start with a uniform distribution: π[s] = 1 / |states| for all s.
--   2. Compute the next distribution: π'[s_j] = ∑_i π[s_i] * T[s_i][s_j]
--   3. Measure convergence: max |π'[s] - π[s]| over all s.
--   4. If max change < 1e-10, stop. Otherwise set π = π' and go to step 2.
--
-- Convergence is guaranteed for ergodic (irreducible, aperiodic) chains.
-- For non-ergodic chains the iteration may not converge or may converge
-- to a distribution that depends on the starting point.
--
-- For order-k chains we operate on the k-gram contexts as "states" in
-- the stationary distribution.
--
-- @return table {state → float} The stationary distribution.
function MarkovChain:stationary_distribution()
    -- Build the list of all known contexts (for order-1, these are
    -- the states themselves; for order-k, they are k-gram keys).
    -- We use the order-1 states list for the distribution.
    local states = self._states
    local n = #states
    if n == 0 then
        error("Cannot compute stationary distribution: no states")
    end

    -- Step 1: initialise a uniform distribution.
    local pi = {}
    for _, s in ipairs(states) do
        pi[s] = 1.0 / n
    end

    -- Step 2–4: iterate until convergence.
    -- We allow up to 10000 iterations as a safety guard.
    local MAX_ITER = 10000
    local TOLERANCE = 1e-10

    for _ = 1, MAX_ITER do
        -- Compute π' = π · T.
        -- π'[s_j] = ∑_i  π[s_i] * T[s_i → s_j]
        local pi_new = {}
        for _, sj in ipairs(states) do
            pi_new[sj] = 0.0
        end

        for _, si in ipairs(states) do
            local row = self._transitions[si]
            if row and pi[si] and pi[si] > 0 then
                for _, sj in ipairs(states) do
                    local p = row[sj]
                    if p then
                        pi_new[sj] = pi_new[sj] + pi[si] * p
                    end
                end
            end
        end

        -- Measure the maximum absolute change.
        local max_change = 0.0
        for _, s in ipairs(states) do
            local change = math.abs((pi_new[s] or 0.0) - (pi[s] or 0.0))
            if change > max_change then
                max_change = change
            end
        end

        pi = pi_new

        if max_change < TOLERANCE then
            break
        end
    end

    return pi
end

-- =========================================================================
-- Inspection
-- =========================================================================

--- Return the list of all registered states in registration order.
--
-- States are returned in the order they were first encountered during
-- training or pre-registration. This order is used for sampling
-- (iteration over self._states) to guarantee reproducible behaviour.
--
-- @return table An array of state strings.
function MarkovChain:states()
    -- Return a copy so callers cannot mutate internal state.
    local copy = {}
    for i, s in ipairs(self._states) do
        copy[i] = s
    end
    return copy
end

--- Return the full transition matrix as a nested table.
--
-- The outer keys are context strings (state names for order-1 chains,
-- or "\0"-joined k-grams for order-k chains). The inner keys are target
-- state strings and the values are probabilities.
--
-- Example return value (order-1, states A and B, trained on [A,B]):
--   {
--     ["A"] = {["A"] = 0.0, ["B"] = 1.0},
--     ["B"] = {},   -- B has no outgoing transitions
--   }
--
-- @return table {context → {state → float}} A copy of the transition table.
function MarkovChain:transition_matrix()
    -- Return a deep copy to prevent callers from mutating internal state.
    local result = {}
    for ctx, row in pairs(self._transitions) do
        local row_copy = {}
        for target, prob in pairs(row) do
            row_copy[target] = prob
        end
        result[ctx] = row_copy
    end
    return result
end

-- =========================================================================
-- Module export
-- =========================================================================

local markov_chain = {}
markov_chain.VERSION = "0.1.0"
markov_chain.MarkovChain = MarkovChain

return markov_chain
