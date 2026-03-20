# frozen_string_literal: true

# Entry point for the coding_adventures_state_machine gem.
#
# This gem implements formal automata theory in Ruby:
#
# - **DFA** (Deterministic Finite Automaton): the workhorse of state machines.
#   One state at a time, one transition per (state, event) pair. Used everywhere
#   from CPU branch predictors to network protocol handlers.
#
# - **NFA** (Non-deterministic Finite Automaton): can be in multiple states
#   simultaneously, with epsilon (free) transitions. Easier to construct than
#   DFAs for many problems. Can be converted to an equivalent DFA via subset
#   construction.
#
# - **Minimize**: Hopcroft's algorithm for DFA minimization. Finds the smallest
#   DFA that recognizes the same language.
#
# - **PDA** (Pushdown Automaton): a state machine with a stack, capable of
#   recognizing context-free languages like balanced parentheses and a^n b^n.
#
# - **Modal**: a collection of named DFA sub-machines with mode transitions,
#   like an HTML tokenizer switching between DATA, TAG, and SCRIPT modes.
#
# Usage:
#   require "coding_adventures_state_machine"
#
#   turnstile = CodingAdventures::StateMachine::DFA.new(
#     states: Set["locked", "unlocked"],
#     alphabet: Set["coin", "push"],
#     transitions: { ["locked", "coin"] => "unlocked", ... },
#     initial: "locked",
#     accepting: Set["unlocked"]
#   )
#   turnstile.process("coin")  # => "unlocked"

require_relative "coding_adventures/state_machine/version"
require_relative "coding_adventures/state_machine/types"
require_relative "coding_adventures/state_machine/dfa"
require_relative "coding_adventures/state_machine/nfa"
require_relative "coding_adventures/state_machine/minimize"
require_relative "coding_adventures/state_machine/pda"
require_relative "coding_adventures/state_machine/modal"
