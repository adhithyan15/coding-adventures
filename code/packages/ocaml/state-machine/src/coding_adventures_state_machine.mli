(** Bounded, deterministic finite, pushdown, and modal state machines. *)

type state = string
type event = string

type transition_record = {
  source : state;
  event : event option;
  target : state;
  action_name : string option;
}

type nfa_trace_record = {
  states_before : state list;
  event : event option;
  states_after : state list;
}

type action = {
  name : string;
  run : state -> event -> state -> unit;
}

type dfa_transition = { source : state; event : event; target : state }
type action_binding = { source : state; event : event; action : action }

type nfa_transition = {
  source : state;
  event : event option;
  targets : state list;
}

type error =
  | Empty_states
  | Empty_modes
  | Unknown_initial of state
  | Unknown_accepting of state
  | Unknown_transition_source of state
  | Unknown_transition_event of event
  | Unknown_transition_target of state
  | Duplicate_transition of state * event option
  | Action_without_transition of state * event
  | Invalid_limit of string * int
  | Unknown_event of event
  | Missing_transition of state * event
  | Unknown_state of state
  | Trace_limit_exceeded of int
  | Trace_state_limit_exceeded of int
  | Subset_limit_exceeded of int
  | Unknown_initial_stack_symbol of string
  | Unknown_stack_read of string
  | Unknown_stack_push of string
  | Duplicate_pda_transition of state * event option * string
  | Missing_pda_transition of state * event * string option
  | Stack_limit_exceeded of int
  | Epsilon_limit_exceeded of int
  | Duplicate_mode of string
  | Unknown_initial_mode of string
  | Unknown_mode_source of string
  | Unknown_mode_target of string
  | Duplicate_mode_transition of string * event
  | Missing_mode_transition of string * event

module Dfa : sig
  type t

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

  val states : t -> state list
  val alphabet : t -> event list
  val transitions : t -> dfa_transition list
  val initial : t -> state
  val accepting : t -> state list
  val current_state : t -> state
  val trace : t -> transition_record list
  val process : t -> event -> (state, error) result
  val process_sequence : t -> event list -> (transition_record list, error) result
  val accepts : t -> event list -> (bool, error) result
  val reset : t -> unit
  val reachable_states : t -> state list
  val is_complete : t -> bool
  val validate : t -> string list
  val to_table : t -> string list list
  val to_ascii : t -> string
  val to_dot : t -> string
end

module Nfa : sig
  type t

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

  val states : t -> state list
  val alphabet : t -> event list
  val transitions : t -> nfa_transition list
  val initial : t -> state
  val accepting : t -> state list
  val current_states : t -> state list
  val trace : t -> nfa_trace_record list
  val epsilon_closure : t -> state list -> (state list, error) result
  val process : t -> event -> (state list, error) result
  val process_sequence : t -> event list -> (nfa_trace_record list, error) result
  val accepts : t -> event list -> (bool, error) result
  val reset : t -> unit
  val to_dfa : t -> (Dfa.t, error) result
  val to_dot : t -> string
end

val minimize : Dfa.t -> Dfa.t

module Pda : sig
  type transition = {
    source : state;
    event : event option;
    stack_read : string;
    target : state;
    stack_push : string list;
  }

  type trace_entry = {
    source : state;
    event : event option;
    stack_read : string;
    target : state;
    stack_push : string list;
    stack_after : string list;
  }

  type t

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

  val states : t -> state list
  val input_alphabet : t -> event list
  val stack_alphabet : t -> string list
  val transitions : t -> transition list
  val accepting : t -> state list
  val current_state : t -> state
  val stack : t -> string list
  val stack_top : t -> string option
  val trace : t -> trace_entry list
  val process : t -> event -> (state, error) result
  val process_sequence : t -> event list -> (trace_entry list, error) result
  val accepts : t -> event list -> (bool, error) result
  val reset : t -> unit
end

module Modal : sig
  type trace_entry = {
    from_mode : string;
    trigger : event;
    to_mode : string;
  }

  type t

  val create :
    ?max_trace_entries:int ->
    modes:(string * Dfa.t) list ->
    mode_transitions:((string * event) * string) list ->
    initial_mode:string ->
    unit ->
    (t, error) result

  val mode_names : t -> string list
  val current_mode : t -> string
  val active_machine : t -> Dfa.t
  val mode_trace : t -> trace_entry list
  val process : t -> event -> (state, error) result
  val switch_mode : t -> event -> (string, error) result
  val reset : t -> unit
end
