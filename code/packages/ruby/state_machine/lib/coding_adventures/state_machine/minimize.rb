# frozen_string_literal: true

require "set"

# ---------------------------------------------------------------------------
# DFA Minimization using Hopcroft's algorithm.
# ---------------------------------------------------------------------------
#
# === What is DFA minimization? ===
#
# Two DFA states are **equivalent** if, for every possible input sequence,
# they either both lead to acceptance or both lead to rejection. Equivalent
# states can be merged without changing the language the DFA recognizes.
#
# DFA minimization finds and merges all equivalent states, producing the
# **smallest possible DFA** for a given regular language. This minimal DFA
# is unique (up to state renaming) -- no matter how you construct a DFA for
# a language, minimization always produces the same result.
#
# === Why minimize? ===
#
# 1. **Efficiency:** Fewer states = less memory, faster lookup tables.
# 2. **Canonical form:** Two DFAs recognize the same language if and only if
#    their minimal forms are identical (after renaming). This gives us a way
#    to test language equivalence.
# 3. **Clean up subset construction:** Converting an NFA to a DFA via subset
#    construction often produces many redundant states. Minimization removes
#    them.
#
# === Hopcroft's Algorithm ===
#
# The algorithm works by **partition refinement**:
#
# 1. Start with two groups: accepting states and non-accepting states.
#    (These are definitely NOT equivalent to each other.)
#
# 2. For each group and each input symbol, check: do all states in the
#    group go to the same group on that input? If not, split the group.
#
# 3. Repeat until no group can be split.
#
# 4. Each final group becomes one state in the minimized DFA.
#
# === Visualization of Partition Refinement ===
#
# Step 0: Start with two partitions
#
#     [accepting states] | [non-accepting states]
#
# Step 1: For each input symbol, check if a partition needs splitting.
#         If states in a group go to different groups on some input, split.
#
#     Example: states A and B are both accepting, but on input 'x':
#       A -> goes to a non-accepting group
#       B -> goes to an accepting group
#     Split! A and B are not equivalent.
#
# Step 2: Repeat until stable (no more splits possible).
#
# Step 3: Build minimized DFA where each partition = one state.
#
# Time complexity: O(n log n) where n = number of states.
#
# === Ruby Implementation Notes ===
#
# - Partitions are stored as Arrays of frozen Sets.
# - We use a module_function so minimize() can be called directly:
#     CodingAdventures::StateMachine.minimize(dfa)
# ---------------------------------------------------------------------------

module CodingAdventures
  module StateMachine
    # Minimize a DFA using Hopcroft's algorithm.
    #
    # Returns a new DFA with the minimum number of states that recognizes
    # the same language as the input DFA. Unreachable states are removed
    # first, then equivalent states are merged.
    #
    # The minimized DFA is unique (up to state naming) for any regular
    # language -- this is a fundamental theorem of automata theory.
    #
    # @param dfa [DFA] The DFA to minimize.
    # @return [DFA] A new, minimized DFA.
    def self.minimize(dfa)
      # Step 0: Remove unreachable states
      #
      # Any state that cannot be reached from the initial state is useless.
      # We remove them first to simplify the partition refinement.
      reachable = dfa.reachable_states
      reachable_accepting = dfa.accepting & reachable

      # Filter transitions to only reachable states
      transitions = {}
      dfa.transitions.each do |(source, event), target|
        if reachable.include?(source) && reachable.include?(target)
          transitions[[source, event]] = target
        end
      end

      # Step 1: Initial partition -- accepting vs non-accepting
      # Only include reachable states
      accepting = reachable_accepting
      non_accepting = reachable - accepting

      partitions = []
      partitions << accepting.freeze unless accepting.empty?
      partitions << non_accepting.freeze unless non_accepting.empty?

      # Edge case: no reachable states (shouldn't happen with valid DFA)
      return dfa if partitions.empty?

      # Step 2-3: Iteratively refine partitions
      #
      # Keep splitting groups until no more splits are possible.
      # Each iteration checks every group against every input symbol.
      # If any group can be split, we start over with the new partitions.
      alphabet = dfa.alphabet.to_a.sort
      changed = true

      while changed
        changed = false
        new_partitions = []

        partitions.each do |group|
          split = split_group(group, alphabet, transitions, partitions)
          changed = true if split.length > 1
          new_partitions.concat(split)
        end

        partitions = new_partitions
      end

      # Step 4: Build the minimized DFA
      #
      # Each partition becomes a state in the new DFA. We need to:
      # - Name each partition (use sorted member names)
      # - Map old transitions to new partition-based transitions
      # - Identify which partition is the initial state
      # - Identify which partitions are accepting
      state_to_partition = {}
      partitions.each do |partition|
        partition.each do |state|
          state_to_partition[state] = partition
        end
      end

      # Build new states, transitions, initial, accepting
      new_states = Set.new
      new_transitions = {}
      new_accepting = Set.new

      partitions.each do |partition|
        name = partition_name(partition)
        new_states.add(name)

        new_accepting.add(name) if !(partition & accepting).empty?

        # Use any representative state from the partition for transitions.
        # Since all states in the partition are equivalent, they all have
        # the same transition behavior -- so it doesn't matter which one
        # we pick.
        representative = partition.to_a.sort.first
        alphabet.each do |event|
          key = [representative, event]
          if transitions.key?(key)
            target = transitions[key]
            target_partition = state_to_partition[target]
            target_name = partition_name(target_partition)
            new_transitions[[name, event]] = target_name
          end
        end
      end

      # Find the new initial state
      initial_partition = state_to_partition[dfa.initial]
      new_initial = partition_name(initial_partition)

      DFA.new(
        states: new_states,
        alphabet: dfa.alphabet.dup,
        transitions: new_transitions,
        initial: new_initial,
        accepting: new_accepting
      )
    end

    # Attempt to split a group based on transition targets.
    #
    # Two states in the same group are equivalent only if, for every input
    # symbol, they transition to states in the same partition. If they
    # differ on any input, they must be in different groups.
    #
    # === How splitting works ===
    #
    # For each input symbol, compute a "signature" for each state in the group.
    # The signature is the index of the partition that the state transitions to.
    # States with different signatures are NOT equivalent and must be separated.
    #
    # @param group [Set<String>] The group of states to potentially split.
    # @param alphabet [Array<String>] The input alphabet (sorted).
    # @param transitions [Hash] The DFA's transition function.
    # @param partitions [Array<Set<String>>] The current partition set.
    # @return [Array<Set<String>>] Subgroups (may be just [group] if no split needed).
    def self.split_group(group, alphabet, transitions, partitions)
      return [group] if group.size <= 1

      # Build a lookup: state -> which partition index it belongs to
      state_to_partition_idx = {}
      partitions.each_with_index do |partition, idx|
        partition.each { |state| state_to_partition_idx[state] = idx }
      end

      # For each input symbol, check if the group needs splitting
      alphabet.each do |event|
        signatures = {}
        group.each do |state|
          key = [state, event]
          sig = if transitions.key?(key)
            target = transitions[key]
            state_to_partition_idx[target]
          end
          # nil sig means "no transition" (dead)
          signatures[sig] ||= Set.new
          signatures[sig].add(state)
        end

        if signatures.size > 1
          # Split needed! Return the subgroups.
          return signatures.values.map(&:freeze)
        end
      end

      # No split needed
      [group]
    end

    # Generate a name for a partition (group of equivalent states).
    #
    # Single-state partitions keep their original name. Multi-state
    # partitions get a name like "{q1,q2}" with sorted members.
    #
    # @param partition [Set<String>] A set of equivalent state names.
    # @return [String] A canonical name for this partition.
    def self.partition_name(partition)
      members = partition.to_a.sort
      return members.first if members.length == 1
      "{#{members.join(",")}}"
    end

    # Make helper methods private
    private_class_method :split_group, :partition_name
  end
end
