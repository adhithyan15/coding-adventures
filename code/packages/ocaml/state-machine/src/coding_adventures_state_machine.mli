(** Bounded deterministic, nondeterministic, pushdown, and modal state
    machines.

    State and event strings are opaque names ordered with [String.compare].
    Machine definitions are immutable after construction, while runtime state
    and bounded traces are mutable. In event-bearing records, [None] denotes an
    epsilon transition; [Some ""] is an ordinary named event. *)

(** An opaque state name. *)
type state = string

(** An opaque event name. *)
type event = string

(** One chronological DFA transition trace entry. *)
type transition_record = {
  source : state;  (** State before the transition. *)
  event : event option;  (** Consumed event, or [None] for epsilon. *)
  target : state;  (** State after the transition. *)
  action_name : string option;  (** Bound action name, when present. *)
}

(** One chronological NFA transition trace entry. *)
type nfa_trace_record = {
  states_before : state list;  (** Sorted active states before the step. *)
  event : event option;  (** Consumed event, or [None] for epsilon. *)
  states_after : state list;  (** Sorted epsilon-closed states afterwards. *)
}

(** A named side effect attached to a DFA transition.

    Exceptions raised by [run] propagate to the caller and are not converted
    to {!error} values. *)
type action = {
  name : string;  (** Observable name recorded in the trace. *)
  run : state -> event -> state -> unit;  (** [run source event target]. *)
}

(** A deterministic transition from [source] to [target] on [event]. *)
type dfa_transition = { source : state; event : event; target : state }

(** Associates one action with one deterministic transition key. *)
type action_binding = { source : state; event : event; action : action }

(** An NFA transition row. [event = None] denotes epsilon. *)
type nfa_transition = {
  source : state;  (** Source state. *)
  event : event option;  (** Event key, or epsilon when [None]. *)
  targets : state list;  (** Destination states; duplicates are collapsed. *)
}

(** Construction, lookup, execution, and resource-limit failures. Constructor
    payloads identify the offending name or the configured limit. *)
type error =
  | Empty_states  (** A machine definition has no states. *)
  | Empty_modes  (** A modal machine definition has no modes. *)
  | Unknown_initial of state  (** The initial state is undeclared. *)
  | Unknown_accepting of state  (** An accepting state is undeclared. *)
  | Unknown_transition_source of state  (** A transition source is undeclared. *)
  | Unknown_transition_event of event  (** A transition event is undeclared. *)
  | Unknown_transition_target of state  (** A transition target is undeclared. *)
  | Duplicate_transition of state * event option
      (** More than one transition row has the same key. *)
  | Action_without_transition of state * event
      (** An action key has no matching DFA transition. *)
  | Invalid_limit of string * int  (** A named resource limit is invalid. *)
  | Unknown_event of event  (** A processed event is outside the alphabet. *)
  | Missing_transition of state * event
      (** A partial DFA has no transition for a runtime key. *)
  | Unknown_state of state  (** A queried state is undeclared. *)
  | Trace_limit_exceeded of int  (** The trace-entry ceiling was exceeded. *)
  | Trace_state_limit_exceeded of int
      (** The NFA trace state-cell ceiling was exceeded. *)
  | Subset_limit_exceeded of int  (** NFA determinization exceeded its ceiling. *)
  | Unknown_initial_stack_symbol of string
      (** The PDA initial stack symbol is undeclared. *)
  | Unknown_stack_read of string  (** A PDA pop symbol is undeclared. *)
  | Unknown_stack_push of string  (** A PDA push symbol is undeclared. *)
  | Duplicate_pda_transition of state * event option * string
      (** More than one PDA transition has the same lookup key. *)
  | Missing_pda_transition of state * event * string option
      (** No named PDA transition matches the state, event, and stack top. *)
  | Stack_limit_exceeded of int  (** A PDA stack-depth ceiling was exceeded. *)
  | Epsilon_limit_exceeded of int  (** A PDA epsilon-step ceiling was exceeded. *)
  | Duplicate_mode of string  (** A modal definition repeats a mode name. *)
  | Unknown_initial_mode of string  (** The initial mode is undeclared. *)
  | Unknown_mode_source of string  (** A mode transition source is undeclared. *)
  | Unknown_mode_target of string  (** A mode transition target is undeclared. *)
  | Duplicate_mode_transition of string * event
      (** More than one mode transition has the same key. *)
  | Missing_mode_transition of string * event
      (** No mode transition matches the current mode and trigger. *)

module Dfa : sig
  (** Mutable runtime for a deterministic, possibly partial finite automaton.
      Acceptance simulations always start from the declared initial state. *)

  (** An immutable DFA definition with mutable current state and trace. *)
  type t

  (** [create] validates and constructs a DFA. Duplicate states and alphabet
      events collapse, but duplicate transition keys fail. Action bindings
      require matching transitions; if repeated, the last binding wins.
      [max_trace_entries] defaults to [100_000] and may be zero. *)
  val create :
    ?actions:action_binding list ->
    ?max_trace_entries:int ->
    states:state list ->
    alphabet:event list ->
    transitions:dfa_transition list ->
    initial:state ->
    accepting:state list ->
    unit ->
    (t, error) result

  (** Declared states in sorted order. *)
  val states : t -> state list

  (** Declared alphabet in sorted order. *)
  val alphabet : t -> event list

  (** Transitions in deterministic key order. *)
  val transitions : t -> dfa_transition list

  (** Declared initial state. *)
  val initial : t -> state

  (** Accepting states in sorted order. *)
  val accepting : t -> state list

  (** Current mutable runtime state. *)
  val current_state : t -> state

  (** Retained transition records in chronological order. *)
  val trace : t -> transition_record list

  (** Consume one event, update the current state, run its action, and append a
      trace entry. Missing transitions fail. *)
  val process : t -> event -> (state, error) result

  (** Preflight and consume a sequence atomically with respect to machine state
      and trace. Actions run only after preflight. If an action raises, machine
      state and trace stay unchanged, although earlier external effects from
      actions may already have occurred. *)
  val process_sequence : t -> event list -> (transition_record list, error) result

  (** Simulate from the initial state without mutation. A missing transition
      rejects with [Ok false]. *)
  val accepts : t -> event list -> (bool, error) result

  (** Restore the initial state and clear the trace. *)
  val reset : t -> unit

  (** Sorted states reachable from the initial state, including the initial
      state itself. *)
  val reachable_states : t -> state list

  (** Whether every state/event pair has a transition. *)
  val is_complete : t -> bool

  (** Deterministic human-readable warnings. An unreachable accepting state may
      contribute more than one semantic warning. *)
  val validate : t -> string list

  (** A deterministic table whose missing-transition cell is ["-"]. *)
  val to_table : t -> string list list

  (** Render the transition table as a newline-delimited ASCII table. *)
  val to_ascii : t -> string

  (** Render deterministic Graphviz DOT, escaping names as needed. *)
  val to_dot : t -> string
end

module Nfa : sig
  (** Nondeterministic finite automata with iterative, cycle-safe epsilon
      closure. *)

  (** An immutable NFA definition with mutable active states and trace. *)
  type t

  (** Construct an NFA. Duplicate states, alphabet entries, and row targets
      collapse; duplicate [(source, event)] rows fail. Limits default to 4096
      generated DFA subsets, 100,000 trace entries, and 1,000,000 retained
      trace state cells. All may be zero. Runtime state begins at the epsilon
      closure of [initial]. *)
  val create :
    ?max_generated_states:int ->
    ?max_trace_entries:int ->
    ?max_trace_state_cells:int ->
    states:state list ->
    alphabet:event list ->
    transitions:nfa_transition list ->
    initial:state ->
    accepting:state list ->
    unit ->
    (t, error) result

  (** Declared states in sorted order. *)
  val states : t -> state list

  (** Declared alphabet in sorted order. *)
  val alphabet : t -> event list

  (** Transition rows in deterministic order. *)
  val transitions : t -> nfa_transition list

  (** Declared initial state. *)
  val initial : t -> state

  (** Accepting states in sorted order. *)
  val accepting : t -> state list

  (** Current epsilon-closed active states in sorted order. *)
  val current_states : t -> state list

  (** Retained trace records in chronological order. *)
  val trace : t -> nfa_trace_record list

  (** Compute the iterative epsilon closure of declared seed states. *)
  val epsilon_closure : t -> state list -> (state list, error) result

  (** Consume one named event and epsilon-close the result. The trace cell cost
      is the combined before/after cardinality; limit checks precede mutation. *)
  val process : t -> event -> (state list, error) result

  (** Consume events in order and return records added by this call. A later
      failure leaves earlier successful state and trace mutations in place. *)
  val process_sequence : t -> event list -> (nfa_trace_record list, error) result

  (** Simulate from the initial closure without mutation. Missing transitions
      reject rather than fail. *)
  val accepts : t -> event list -> (bool, error) result

  (** Restore the initial closure and clear trace entry and cell accounting. *)
  val reset : t -> unit

  (** Determinize by bounded subset construction without mutating the NFA. The
      result is complete, includes the empty-set dead state when reachable, and
      assigns deterministic opaque names [S0], [S1], and so on. *)
  val to_dfa : t -> (Dfa.t, error) result

  (** Render deterministic Graphviz DOT using [ε] for epsilon transitions. *)
  val to_dot : t -> string
end

(** Minimize the reachable part of a DFA while preserving its accepted language
    and partial transitions. Missing transitions share one synthetic refinement
    outcome. The result uses deterministic opaque names [M0], [M1], and so on,
    has reset runtime state and no actions or history, and uses the default
    100,000-entry trace ceiling rather than the source ceiling. *)
val minimize : Dfa.t -> Dfa.t

module Pda : sig
  (** Deterministic pushdown automata keyed by
      [(state, event option, stack_top)] and accepted by final state. *)

  (** One PDA transition definition. Stack lists are bottom-to-top; after the
      read symbol is popped, [stack_push] is appended, making its last item the
      new top. *)
  type transition = {
    source : state;  (** Source state. *)
    event : event option;  (** Named event or epsilon. *)
    stack_read : string;  (** Required and popped top symbol. *)
    target : state;  (** Destination state. *)
    stack_push : string list;  (** Symbols appended bottom-to-top. *)
  }

  (** One chronological PDA runtime trace entry. *)
  type trace_entry = {
    source : state;  (** State before the step. *)
    event : event option;  (** Named event or epsilon consumed. *)
    stack_read : string;  (** Symbol popped from the stack. *)
    target : state;  (** State after the step. *)
    stack_push : string list;  (** Symbols pushed by the transition. *)
    stack_after : string list;  (** Resulting stack, bottom-to-top. *)
  }

  (** An immutable PDA definition with mutable state, stack, and trace. *)
  type t

  (** Construct a PDA. Duplicate transition keys fail. Stack depth and epsilon
      limits default to 4096 and must be positive; the 100,000-entry trace
      limit may be zero. The initial stack contains only
      [initial_stack_symbol]. *)
  val create :
    ?max_stack_depth:int ->
    ?max_trace_entries:int ->
    ?max_epsilon_steps:int ->
    states:state list ->
    input_alphabet:event list ->
    stack_alphabet:string list ->
    transitions:transition list ->
    initial:state ->
    initial_stack_symbol:string ->
    accepting:state list ->
    unit ->
    (t, error) result

  (** Declared states in sorted order. *)
  val states : t -> state list

  (** Declared named input events in sorted order. *)
  val input_alphabet : t -> event list

  (** Declared stack symbols in sorted order. *)
  val stack_alphabet : t -> string list

  (** Transition definitions in deterministic key order. *)
  val transitions : t -> transition list

  (** Accepting final states in sorted order. *)
  val accepting : t -> state list

  (** Current mutable runtime state. *)
  val current_state : t -> state

  (** Current stack from bottom to top. *)
  val stack : t -> string list

  (** Last stack item, or [None] when the stack is empty. *)
  val stack_top : t -> string option

  (** Retained transition entries in chronological order. *)
  val trace : t -> trace_entry list

  (** Consume one named event only; epsilon transitions are not taken
      automatically. *)
  val process : t -> event -> (state, error) result

  (** Consume named events, then take deterministic epsilon transitions only at
      end-of-input. Successful prefix and epsilon mutations remain if a later
      error or resource limit is reached. *)
  val process_sequence : t -> event list -> (trace_entry list, error) result

  (** Simulate from the initial state and one-symbol stack without mutation.
      Missing transitions reject with [Ok false]; the same end-only epsilon
      phase and limits apply. Acceptance depends on final state, not an empty
      stack. *)
  val accepts : t -> event list -> (bool, error) result

  (** Restore the initial state and one-symbol stack and clear the trace. *)
  val reset : t -> unit
end

module Modal : sig
  (** A collection of named, cloned DFAs with explicit labeled mode switches. *)

  (** One chronological mode-switch record. Contained DFA transitions are kept
      only in the active DFA's trace. *)
  type trace_entry = {
    from_mode : string;  (** Mode before the switch. *)
    trigger : event;  (** Event labeling the switch. *)
    to_mode : string;  (** Mode activated after the switch. *)
  }

  (** Mutable modal runtime with independently mutable cloned DFAs. *)
  type t

  (** Construct a modal machine. Mode names must be unique and all input DFAs
      are cloned and reset. Self-mode transitions are allowed. The trace limit
      defaults to 100,000 and may be zero. *)
  val create :
    ?max_trace_entries:int ->
    modes:(string * Dfa.t) list ->
    mode_transitions:((string * event) * string) list ->
    initial_mode:string ->
    unit ->
    (t, error) result

  (** Mode names in sorted order. *)
  val mode_names : t -> string list

  (** Name of the current mode. *)
  val current_mode : t -> string

  (** A cloned snapshot of the active DFA. Mutating it does not mutate the
      modal machine. *)
  val active_machine : t -> Dfa.t

  (** Chronological mode-switch records, excluding contained DFA records. *)
  val mode_trace : t -> trace_entry list

  (** Send an event only to the active DFA; this never changes mode. *)
  val process : t -> event -> (state, error) result

  (** Follow an explicit mode transition and reset the target DFA before
      activation. A self-switch therefore resets the active DFA. Trace-limit
      failure occurs before reset or mutation. *)
  val switch_mode : t -> event -> (string, error) result

  (** Reset every contained DFA, restore the initial mode, and clear switch
      history. *)
  val reset : t -> unit
end
