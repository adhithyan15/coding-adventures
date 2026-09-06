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

module String_set = Set.Make (String)
module String_map = Map.Make (String)

module Event_key = struct
  type t = string * string

  let compare = Stdlib.compare
end

module Event_map = Map.Make (Event_key)

let sorted_unique values =
  List.fold_left (fun set value -> String_set.add value set) String_set.empty
    values

let first_missing set values =
  List.find_opt (fun value -> not (String_set.mem value set)) values

module Dfa = struct
  module Graph = Coding_adventures_directed_graph.Make (String)

  type t = {
    state_set : String_set.t;
    alphabet_set : String_set.t;
    transition_map : string Event_map.t;
    transition_rows : dfa_transition list;
    action_map : action Event_map.t;
    initial_state : state;
    accepting_set : String_set.t;
    max_trace_entries : int;
    mutable current : state;
    mutable trace_rev : transition_record list;
  }

  let states machine = String_set.elements machine.state_set
  let alphabet machine = String_set.elements machine.alphabet_set
  let transitions machine = List.map Fun.id machine.transition_rows
  let initial machine = machine.initial_state
  let accepting machine = String_set.elements machine.accepting_set
  let current_state machine = machine.current
  let trace machine = List.rev machine.trace_rev

  let add_transition state_set alphabet_set (map, rows)
      (row : dfa_transition) =
    if not (String_set.mem row.source state_set) then
      Error (Unknown_transition_source row.source)
    else if not (String_set.mem row.event alphabet_set) then
      Error (Unknown_transition_event row.event)
    else if not (String_set.mem row.target state_set) then
      Error (Unknown_transition_target row.target)
    else
      let key = row.source, row.event in
      if Event_map.mem key map then
        Error (Duplicate_transition (row.source, Some row.event))
      else Ok (Event_map.add key row.target map, row :: rows)

  let rec fold_result function_ accumulator = function
    | [] -> Ok accumulator
    | head :: tail -> (
        match function_ accumulator head with
        | Error error -> Error error
        | Ok next -> fold_result function_ next tail)

  let create ?(actions = []) ?(max_trace_entries = 100_000) ~states
      ~alphabet ~transitions ~initial ~accepting () =
    let state_set = sorted_unique states in
    let alphabet_set = sorted_unique alphabet in
    if String_set.is_empty state_set then Error Empty_states
    else if max_trace_entries < 0 then
      Error (Invalid_limit ("max_trace_entries", max_trace_entries))
    else if not (String_set.mem initial state_set) then
      Error (Unknown_initial initial)
    else
      match first_missing state_set accepting with
      | Some value -> Error (Unknown_accepting value)
      | None -> (
          match
            fold_result
              (add_transition state_set alphabet_set)
              (Event_map.empty, []) transitions
          with
          | Error error -> Error error
          | Ok (transition_map, transition_rows_rev) ->
              let add_action map (binding : action_binding) =
                let key = binding.source, binding.event in
                if not (Event_map.mem key transition_map) then
                  Error
                    (Action_without_transition
                       (binding.source, binding.event))
                else Ok (Event_map.add key binding.action map)
              in
              (match fold_result add_action Event_map.empty actions with
              | Error error -> Error error
              | Ok action_map ->
                  Ok
                    {
                      state_set;
                      alphabet_set;
                      transition_map;
                      transition_rows =
                        List.sort Stdlib.compare
                          (List.rev transition_rows_rev);
                      action_map;
                      initial_state = initial;
                      accepting_set = sorted_unique accepting;
                      max_trace_entries;
                      current = initial;
                      trace_rev = [];
                    }))

  (* Planning the whole input first makes sequence failures atomic even when
     actions have effects that cannot be rolled back. *)
  let plan_from machine start remaining events =
    let rec loop current records remaining = function
      | [] -> Ok (current, List.rev records)
      | event :: rest ->
          if not (String_set.mem event machine.alphabet_set) then
            Error (Unknown_event event)
          else (
            match Event_map.find_opt (current, event) machine.transition_map with
            | None -> Error (Missing_transition (current, event))
            | Some _ when remaining = 0 ->
                Error (Trace_limit_exceeded machine.max_trace_entries)
            | Some target ->
                let action_name =
                  Event_map.find_opt (current, event) machine.action_map
                  |> Option.map (fun action -> action.name)
                in
                loop target
                  ({ source = current; event = Some event; target; action_name }
                  :: records)
                  (remaining - 1)
                  rest)
    in
    loop start [] remaining events

  let run_actions machine records =
    List.iter
      (fun (record : transition_record) ->
        match record.event with
        | None -> ()
        | Some event ->
            Option.iter
              (fun action -> action.run record.source event record.target)
              (Event_map.find_opt (record.source, event) machine.action_map))
      records

  let process_sequence machine events =
    let remaining =
      machine.max_trace_entries - List.length machine.trace_rev
    in
    match plan_from machine machine.current remaining events with
    | Error error -> Error error
    | Ok (target, records) ->
        run_actions machine records;
        machine.current <- target;
        machine.trace_rev <- List.rev_append records machine.trace_rev;
        Ok records

  let process machine event =
    match process_sequence machine [ event ] with
    | Error error -> Error error
    | Ok [ record ] -> Ok record.target
    | Ok _ -> assert false

  let accepts machine events =
    let rec loop current = function
      | [] -> Ok (String_set.mem current machine.accepting_set)
      | event :: rest ->
          if not (String_set.mem event machine.alphabet_set) then
            Error (Unknown_event event)
          else (
            match Event_map.find_opt (current, event) machine.transition_map with
            | None -> Ok false
            | Some target -> loop target rest)
    in
    loop machine.initial_state events

  let reset machine =
    machine.current <- machine.initial_state;
    machine.trace_rev <- []

  let reachable_states machine =
    let graph = Graph.create ~allow_self_loops:true () in
    String_set.iter (Graph.add_node graph) machine.state_set;
    List.iter
      (fun (row : dfa_transition) ->
        match Graph.add_edge graph row.source row.target with
        | Ok () -> ()
        | Error _ -> assert false)
      machine.transition_rows;
    match Graph.transitive_closure graph machine.initial_state with
    | Error _ -> [ machine.initial_state ]
    | Ok reached ->
        String_set.elements
          (List.fold_left
             (fun set value -> String_set.add value set)
             (String_set.singleton machine.initial_state)
             reached)

  let is_complete machine =
    String_set.for_all
      (fun state ->
        String_set.for_all
          (fun event -> Event_map.mem (state, event) machine.transition_map)
          machine.alphabet_set)
      machine.state_set

  let validate machine =
    let reached = sorted_unique (reachable_states machine) in
    let unreachable = String_set.diff machine.state_set reached in
    let unreachable_accepting = String_set.diff machine.accepting_set reached in
    let warnings = ref [] in
    String_set.iter
      (fun state -> warnings := ("unreachable state: " ^ state) :: !warnings)
      unreachable;
    String_set.iter
      (fun state ->
        warnings := ("unreachable accepting state: " ^ state) :: !warnings)
      unreachable_accepting;
    String_set.iter
      (fun state ->
        String_set.iter
          (fun event ->
            if not (Event_map.mem (state, event) machine.transition_map) then
              warnings :=
                ("missing transition: " ^ state ^ " / " ^ event) :: !warnings)
          machine.alphabet_set)
      machine.state_set;
    List.rev !warnings

  let to_table machine =
    [ "State" :: alphabet machine ]
    @ List.map
        (fun state ->
          state
          :: List.map
               (fun event ->
                 Option.value ~default:"-"
                   (Event_map.find_opt (state, event) machine.transition_map))
               (alphabet machine))
        (states machine)

  let to_ascii machine =
    to_table machine |> List.map (String.concat " | ") |> String.concat "\n"

  let quote value = "\"" ^ String.escaped value ^ "\""

  let to_dot machine =
    let lines =
      ref
        [
          "digraph DFA {";
          "  rankdir=LR;";
          "  __start [shape=point];";
          "  __start -> " ^ quote machine.initial_state ^ ";";
        ]
    in
    List.iter
      (fun state ->
        let shape =
          if String_set.mem state machine.accepting_set then "doublecircle"
          else "circle"
        in
        lines :=
          !lines @ [ "  " ^ quote state ^ " [shape=" ^ shape ^ "];" ])
      (states machine);
    List.iter
      (fun (row : dfa_transition) ->
        lines :=
          !lines
          @ [
              "  " ^ quote row.source ^ " -> " ^ quote row.target
              ^ " [label=" ^ quote row.event ^ "];";
            ])
      machine.transition_rows;
    String.concat "\n" (!lines @ [ "}" ])

  let clone machine =
    {
      machine with
      current = machine.current;
      trace_rev = List.map Fun.id machine.trace_rev;
    }
end

module Nfa_key = struct
  type t = string * string option

  let compare = Stdlib.compare
end

module Nfa_map = Map.Make (Nfa_key)

module State_set_key = struct
  type t = String_set.t

  let compare = String_set.compare
end

module State_set_map = Map.Make (State_set_key)

module Nfa = struct
  type t = {
    state_set : String_set.t;
    alphabet_set : String_set.t;
    transition_map : String_set.t Nfa_map.t;
    transition_rows : nfa_transition list;
    initial_state : state;
    accepting_set : String_set.t;
    max_generated_states : int;
    max_trace_entries : int;
    max_trace_state_cells : int;
    mutable current : String_set.t;
    mutable trace_rev : nfa_trace_record list;
    mutable trace_cells : int;
  }

  (* The queue terminates on epsilon cycles because a state is enqueued once. *)
  let epsilon_closure_set map seeds =
    let closure = ref seeds in
    let queue = Queue.create () in
    String_set.iter (fun state -> Queue.add state queue) seeds;
    while not (Queue.is_empty queue) do
      let state = Queue.take queue in
      let targets =
        Option.value ~default:String_set.empty
          (Nfa_map.find_opt (state, None) map)
      in
      String_set.iter
        (fun target ->
          if not (String_set.mem target !closure) then (
            closure := String_set.add target !closure;
            Queue.add target queue))
        targets
    done;
    !closure

  let states machine = String_set.elements machine.state_set
  let alphabet machine = String_set.elements machine.alphabet_set
  let transitions machine = List.map Fun.id machine.transition_rows
  let initial machine = machine.initial_state
  let accepting machine = String_set.elements machine.accepting_set
  let current_states machine = String_set.elements machine.current
  let trace machine = List.rev machine.trace_rev

  let create ?(max_generated_states = 4_096) ?(max_trace_entries = 100_000)
      ?(max_trace_state_cells = 1_000_000) ~states ~alphabet ~transitions
      ~initial ~accepting () =
    let state_set = sorted_unique states in
    let alphabet_set = sorted_unique alphabet in
    let invalid_limit =
      List.find_opt
        (fun (_, value) -> value < 0)
        [
          ("max_generated_states", max_generated_states);
          ("max_trace_entries", max_trace_entries);
          ("max_trace_state_cells", max_trace_state_cells);
        ]
    in
    if String_set.is_empty state_set then Error Empty_states
    else
      match invalid_limit with
      | Some (name, value) -> Error (Invalid_limit (name, value))
      | None when not (String_set.mem initial state_set) ->
          Error (Unknown_initial initial)
      | None -> (
          match first_missing state_set accepting with
          | Some value -> Error (Unknown_accepting value)
          | None ->
              let add (map, rows) (row : nfa_transition) =
                if not (String_set.mem row.source state_set) then
                  Error (Unknown_transition_source row.source)
                else
                  match row.event with
                  | Some event when not (String_set.mem event alphabet_set) ->
                      Error (Unknown_transition_event event)
                  | _ -> (
                      match first_missing state_set row.targets with
                      | Some value -> Error (Unknown_transition_target value)
                      | None ->
                          let key = row.source, row.event in
                          if Nfa_map.mem key map then
                            Error (Duplicate_transition (row.source, row.event))
                          else
                            let targets = sorted_unique row.targets in
                            Ok
                              ( Nfa_map.add key targets map,
                                { row with targets = String_set.elements targets }
                                :: rows ))
              in
              (match
                 Dfa.fold_result add (Nfa_map.empty, []) transitions
               with
              | Error error -> Error error
              | Ok (transition_map, rows_rev) ->
                  let current =
                    epsilon_closure_set transition_map
                      (String_set.singleton initial)
                  in
                  Ok
                    {
                      state_set;
                      alphabet_set;
                      transition_map;
                      transition_rows =
                        List.sort Stdlib.compare (List.rev rows_rev);
                      initial_state = initial;
                      accepting_set = sorted_unique accepting;
                      max_generated_states;
                      max_trace_entries;
                      max_trace_state_cells;
                      current;
                      trace_rev = [];
                      trace_cells = 0;
                    }))

  let epsilon_closure machine values =
    match first_missing machine.state_set values with
    | Some value -> Error (Unknown_state value)
    | None ->
        Ok
          (epsilon_closure_set machine.transition_map (sorted_unique values)
          |> String_set.elements)

  let next_set machine current event =
    String_set.fold
      (fun state result ->
        String_set.union result
          (Option.value ~default:String_set.empty
             (Nfa_map.find_opt (state, Some event) machine.transition_map)))
      current String_set.empty
    |> epsilon_closure_set machine.transition_map

  let process machine event =
    if not (String_set.mem event machine.alphabet_set) then
      Error (Unknown_event event)
    else
      let next = next_set machine machine.current event in
      let cost = String_set.cardinal machine.current + String_set.cardinal next in
      if List.length machine.trace_rev >= machine.max_trace_entries then
        Error (Trace_limit_exceeded machine.max_trace_entries)
      else if machine.trace_cells + cost > machine.max_trace_state_cells then
        Error (Trace_state_limit_exceeded machine.max_trace_state_cells)
      else
        let record =
          {
            states_before = String_set.elements machine.current;
            event = Some event;
            states_after = String_set.elements next;
          }
        in
        machine.current <- next;
        machine.trace_rev <- record :: machine.trace_rev;
        machine.trace_cells <- machine.trace_cells + cost;
        Ok record.states_after

  let process_sequence machine events =
    let before = List.length machine.trace_rev in
    let rec loop = function
      | [] ->
          let all = trace machine in
          Ok (List.filteri (fun index _ -> index >= before) all)
      | event :: rest -> (
          match process machine event with
          | Error error -> Error error
          | Ok _ -> loop rest)
    in
    loop events

  let accepts machine events =
    let rec loop current = function
      | [] ->
          Ok
            (not
               (String_set.is_empty
                  (String_set.inter current machine.accepting_set)))
      | event :: rest ->
          if not (String_set.mem event machine.alphabet_set) then
            Error (Unknown_event event)
          else loop (next_set machine current event) rest
    in
    let start =
      epsilon_closure_set machine.transition_map
        (String_set.singleton machine.initial_state)
    in
    loop start events

  let reset machine =
    machine.current <-
      epsilon_closure_set machine.transition_map
        (String_set.singleton machine.initial_state);
    machine.trace_rev <- [];
    machine.trace_cells <- 0

  let to_dfa machine =
    let initial_set =
      epsilon_closure_set machine.transition_map
        (String_set.singleton machine.initial_state)
    in
    if machine.max_generated_states = 0 then
      Error (Subset_limit_exceeded machine.max_generated_states)
    else
      (* Sets, not rendered state names, are the identity.  Opaque breadth-first
         names avoid collisions between {"a,b"} and {"a"; "b"}.  The empty
         set is a real dead state, so the result is a complete DFA. *)
      let seen = ref (State_set_map.singleton initial_set "S0") in
      let next_id = ref 1 in
      let queue = Queue.create () in
      Queue.add initial_set queue;
      let rows = ref [] in
      let failure = ref None in
      while Option.is_none !failure && not (Queue.is_empty queue) do
        let subset = Queue.take queue in
        String_set.iter
          (fun event ->
            if Option.is_none !failure then
              let target = next_set machine subset event in
              let target_name =
                match State_set_map.find_opt target !seen with
                | Some name -> name
                | None ->
                    if
                      State_set_map.cardinal !seen
                      >= machine.max_generated_states
                    then (
                      failure :=
                        Some
                          (Subset_limit_exceeded machine.max_generated_states);
                      "")
                    else
                      let name = "S" ^ string_of_int !next_id in
                      incr next_id;
                      seen := State_set_map.add target name !seen;
                      Queue.add target queue;
                      name
              in
              if Option.is_none !failure then
                rows :=
                  {
                    source = State_set_map.find subset !seen;
                    event;
                    target = target_name;
                  }
                  :: !rows)
          machine.alphabet_set
      done;
      match !failure with
      | Some error -> Error error
      | None ->
          let accepting =
            State_set_map.fold
              (fun subset name result ->
                if
                  String_set.is_empty
                    (String_set.inter subset machine.accepting_set)
                then result
                else name :: result)
              !seen []
          in
          Dfa.create ~states:(List.map snd (State_set_map.bindings !seen))
            ~alphabet:(alphabet machine) ~transitions:(List.rev !rows)
            ~initial:"S0" ~accepting ()

  let quote value = "\"" ^ String.escaped value ^ "\""

  let event_name = function None -> "ε" | Some value -> value

  let to_dot machine =
    let lines =
      ref
        [
          "digraph NFA {";
          "  rankdir=LR;";
          "  __start [shape=point];";
          "  __start -> " ^ quote machine.initial_state ^ ";";
        ]
    in
    List.iter
      (fun state ->
        let shape =
          if String_set.mem state machine.accepting_set then "doublecircle"
          else "circle"
        in
        lines :=
          !lines @ [ "  " ^ quote state ^ " [shape=" ^ shape ^ "];" ])
      (states machine);
    List.iter
      (fun row ->
        List.iter
          (fun target ->
            lines :=
              !lines
              @ [
                  "  " ^ quote row.source ^ " -> " ^ quote target
                  ^ " [label=" ^ quote (event_name row.event) ^ "];";
                ])
          (List.sort String.compare row.targets))
      machine.transition_rows;
    String.concat "\n" (!lines @ [ "}" ])
end

let minimize machine =
  (* Missing transitions share the synthetic -1 outcome during refinement. *)
  let reachable = sorted_unique (Dfa.reachable_states machine) in
  let accepting = String_set.inter reachable machine.Dfa.accepting_set in
  let rejecting = String_set.diff reachable accepting in
  let partitions =
    [ rejecting; accepting ] |> List.filter (fun set -> not (String_set.is_empty set))
    |> ref
  in
  let block_index state =
    let rec find index = function
      | [] -> -1
      | block :: rest ->
          if String_set.mem state block then index else find (index + 1) rest
    in
    find 0 !partitions
  in
  let signature state =
    Dfa.alphabet machine
    |> List.map (fun event ->
           match
             Event_map.find_opt (state, event) machine.Dfa.transition_map
           with
           | None -> -1
           | Some target -> block_index target)
  in
  let changed = ref true in
  while !changed do
    changed := false;
    let next =
      List.concat_map
        (fun block ->
          let groups =
            String_set.fold
              (fun state groups ->
                let key = signature state in
                let previous =
                  Option.value ~default:String_set.empty
                    (List.assoc_opt key groups)
                in
                (key, String_set.add state previous)
                :: List.remove_assoc key groups)
              block []
            |> List.map snd
          in
          if List.length groups > 1 then changed := true;
          groups)
        !partitions
    in
    partitions := List.sort Stdlib.compare next
  done;
  let named_blocks =
    List.mapi (fun index block -> block, "M" ^ string_of_int index) !partitions
  in
  let name_for state =
    named_blocks
    |> List.find (fun (block, _) -> String_set.mem state block)
    |> snd
  in
  let states = List.map snd named_blocks in
  let transitions =
    List.concat_map
      (fun (block, name) ->
        let representative = String_set.min_elt block in
        List.filter_map
          (fun event ->
            Event_map.find_opt
              (representative, event)
              machine.Dfa.transition_map
            |> Option.map (fun target ->
                   {
                     source = name;
                     event;
                     target = name_for target;
                   }))
          (Dfa.alphabet machine))
      named_blocks
  in
  let accepting =
    named_blocks
    |> List.filter_map (fun (block, name) ->
           if String_set.is_empty (String_set.inter block accepting) then None
           else Some name)
  in
  match
    Dfa.create ~states ~alphabet:(Dfa.alphabet machine) ~transitions
      ~initial:(name_for (Dfa.initial machine)) ~accepting ()
  with
  | Ok reduced -> reduced
  | Error _ -> assert false

module Pda_key = struct
  type t = string * string option * string

  let compare = Stdlib.compare
end

module Pda_map = Map.Make (Pda_key)

module Pda = struct
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

  type t = {
    state_set : String_set.t;
    input_set : String_set.t;
    stack_set : String_set.t;
    transition_map : transition Pda_map.t;
    transition_rows : transition list;
    initial_state : state;
    initial_stack_symbol : string;
    accepting_set : String_set.t;
    max_stack_depth : int;
    max_trace_entries : int;
    max_epsilon_steps : int;
    mutable current : state;
    mutable current_stack : string list;
    mutable trace_rev : trace_entry list;
  }

  let states machine = String_set.elements machine.state_set
  let input_alphabet machine = String_set.elements machine.input_set
  let stack_alphabet machine = String_set.elements machine.stack_set
  let transitions machine = List.map Fun.id machine.transition_rows
  let accepting machine = String_set.elements machine.accepting_set
  let current_state machine = machine.current
  let stack machine = List.map Fun.id machine.current_stack
  let stack_top machine =
    match List.rev machine.current_stack with [] -> None | value :: _ -> Some value
  let trace machine = List.rev machine.trace_rev

  let create ?(max_stack_depth = 4_096) ?(max_trace_entries = 2_048)
      ?(max_epsilon_steps = 10_000) ~states ~input_alphabet ~stack_alphabet
      ~transitions ~initial ~initial_stack_symbol ~accepting () =
    let state_set = sorted_unique states in
    let input_set = sorted_unique input_alphabet in
    let stack_set = sorted_unique stack_alphabet in
    let invalid_limit =
      List.find_opt
        (fun (name, value) ->
          if name = "max_stack_depth" || name = "max_epsilon_steps" then
            value < 1
          else value < 0)
        [
          ("max_stack_depth", max_stack_depth);
          ("max_trace_entries", max_trace_entries);
          ("max_epsilon_steps", max_epsilon_steps);
        ]
    in
    if String_set.is_empty state_set then Error Empty_states
    else
      match invalid_limit with
      | Some (name, value) -> Error (Invalid_limit (name, value))
      | None when not (String_set.mem initial state_set) ->
          Error (Unknown_initial initial)
      | None when not (String_set.mem initial_stack_symbol stack_set) ->
          Error (Unknown_initial_stack_symbol initial_stack_symbol)
      | None -> (
          match first_missing state_set accepting with
          | Some value -> Error (Unknown_accepting value)
          | None ->
              let add (map, rows) (row : transition) =
                if not (String_set.mem row.source state_set) then
                  Error (Unknown_transition_source row.source)
                else if not (String_set.mem row.target state_set) then
                  Error (Unknown_transition_target row.target)
                else
                  match row.event with
                  | Some event when not (String_set.mem event input_set) ->
                      Error (Unknown_transition_event event)
                  | _ when not (String_set.mem row.stack_read stack_set) ->
                      Error (Unknown_stack_read row.stack_read)
                  | _ -> (
                      match first_missing stack_set row.stack_push with
                      | Some value -> Error (Unknown_stack_push value)
                      | None ->
                          let key = row.source, row.event, row.stack_read in
                          if Pda_map.mem key map then
                            Error
                              (Duplicate_pda_transition
                                 (row.source, row.event, row.stack_read))
                          else Ok (Pda_map.add key row map, row :: rows))
              in
              (match Dfa.fold_result add (Pda_map.empty, []) transitions with
              | Error error -> Error error
              | Ok (transition_map, rows_rev) ->
                  Ok
                    {
                      state_set;
                      input_set;
                      stack_set;
                      transition_map;
                      transition_rows =
                        List.sort Stdlib.compare (List.rev rows_rev);
                      initial_state = initial;
                      initial_stack_symbol;
                      accepting_set = sorted_unique accepting;
                      max_stack_depth;
                      max_trace_entries;
                      max_epsilon_steps;
                      current = initial;
                      current_stack = [ initial_stack_symbol ];
                      trace_rev = [];
                    }))

  let top stack =
    match List.rev stack with [] -> None | value :: _ -> Some value

  let apply (row : transition) stack =
    match List.rev stack with
    | [] -> None
    | _ :: rest_rev ->
        let rest = List.rev rest_rev in
        Some (rest @ row.stack_push)

  let process_transition machine event =
    if not (String_set.mem event machine.input_set) then
      Error (Unknown_event event)
    else
      let stack_symbol = top machine.current_stack in
      match stack_symbol with
      | None ->
          Error (Missing_pda_transition (machine.current, event, None))
      | Some symbol -> (
          match
            Pda_map.find_opt
              (machine.current, Some event, symbol)
              machine.transition_map
          with
          | None ->
              Error
                (Missing_pda_transition
                   (machine.current, event, Some symbol))
          | Some row ->
              let next_stack = Option.get (apply row machine.current_stack) in
              if List.length machine.trace_rev >= machine.max_trace_entries then
                Error (Trace_limit_exceeded machine.max_trace_entries)
              else if List.length next_stack > machine.max_stack_depth then
                Error (Stack_limit_exceeded machine.max_stack_depth)
              else
                let entry =
                  {
                    source = row.source;
                    event = row.event;
                    stack_read = row.stack_read;
                    target = row.target;
                    stack_push = List.map Fun.id row.stack_push;
                    stack_after = List.map Fun.id next_stack;
                  }
                in
                machine.current <- row.target;
                machine.current_stack <- next_stack;
                machine.trace_rev <- entry :: machine.trace_rev;
                Ok row.target)

  let process = process_transition

  (* Named input is complete before the bounded end-of-input epsilon phase. *)
  let finish_epsilon machine =
    let steps = ref 0 in
    let failure = ref None in
    let continue = ref true in
    while !continue && Option.is_none !failure do
      match top machine.current_stack with
      | None -> continue := false
      | Some symbol -> (
          match
            Pda_map.find_opt
              (machine.current, None, symbol)
              machine.transition_map
          with
          | None -> continue := false
          | Some row ->
              if !steps >= machine.max_epsilon_steps then
                failure := Some (Epsilon_limit_exceeded machine.max_epsilon_steps)
              else
                let next_stack = Option.get (apply row machine.current_stack) in
                if List.length machine.trace_rev >= machine.max_trace_entries then
                  failure := Some (Trace_limit_exceeded machine.max_trace_entries)
                else if List.length next_stack > machine.max_stack_depth then
                  failure := Some (Stack_limit_exceeded machine.max_stack_depth)
                else (
                  let entry =
                    {
                      source = row.source;
                      event = None;
                      stack_read = row.stack_read;
                      target = row.target;
                      stack_push = List.map Fun.id row.stack_push;
                      stack_after = List.map Fun.id next_stack;
                    }
                  in
                  machine.current <- row.target;
                  machine.current_stack <- next_stack;
                  machine.trace_rev <- entry :: machine.trace_rev;
                  incr steps))
    done;
    match !failure with Some error -> Error error | None -> Ok ()

  let process_sequence machine events =
    let before = List.length machine.trace_rev in
    let rec named = function
      | [] -> finish_epsilon machine
      | event :: rest -> (
          match process machine event with
          | Error error -> Error error
          | Ok _ -> named rest)
    in
    match named events with
    | Error error -> Error error
    | Ok () ->
        trace machine
        |> List.filteri (fun index _ -> index >= before)
        |> fun records -> Ok records

  let accepts machine events =
    let state = ref machine.initial_state in
    let stack = ref [ machine.initial_stack_symbol ] in
    let trace_count = ref 0 in
    let failure = ref None in
    let rejected = ref false in
    let apply_pure row =
      match apply row !stack with
      | None -> rejected := true
      | Some next_stack ->
          if !trace_count >= machine.max_trace_entries then
            failure := Some (Trace_limit_exceeded machine.max_trace_entries)
          else if List.length next_stack > machine.max_stack_depth then
            failure := Some (Stack_limit_exceeded machine.max_stack_depth)
          else (
            state := row.target;
            stack := next_stack;
            incr trace_count)
    in
    List.iter
      (fun event ->
        if Option.is_none !failure && not !rejected then
          if not (String_set.mem event machine.input_set) then
            failure := Some (Unknown_event event)
          else
            match top !stack with
            | None -> rejected := true
            | Some symbol -> (
                match
                  Pda_map.find_opt
                    (!state, Some event, symbol)
                    machine.transition_map
                with
                | None -> rejected := true
                | Some row -> apply_pure row))
      events;
    let epsilon_steps = ref 0 in
    let continue = ref true in
    while
      !continue && Option.is_none !failure && not !rejected
    do
      match top !stack with
      | None -> continue := false
      | Some symbol -> (
          match Pda_map.find_opt (!state, None, symbol) machine.transition_map with
          | None -> continue := false
          | Some row ->
              if !epsilon_steps >= machine.max_epsilon_steps then
                failure := Some (Epsilon_limit_exceeded machine.max_epsilon_steps)
              else (
                incr epsilon_steps;
                apply_pure row))
    done;
    match !failure with
    | Some error -> Error error
    | None ->
        Ok ((not !rejected) && String_set.mem !state machine.accepting_set)

  let reset machine =
    machine.current <- machine.initial_state;
    machine.current_stack <- [ machine.initial_stack_symbol ];
    machine.trace_rev <- []
end

module Mode_graph = Coding_adventures_directed_graph.Make (String)

module Modal = struct
  type trace_entry = {
    from_mode : string;
    trigger : event;
    to_mode : string;
  }

  type t = {
    modes : Dfa.t String_map.t;
    transitions : string Event_map.t;
    initial_mode : string;
    graph : Mode_graph.Labeled.labeled;
    max_trace_entries : int;
    mutable current : string;
    mutable trace_rev : trace_entry list;
  }

  let mode_names machine = List.map fst (String_map.bindings machine.modes)
  let current_mode machine = machine.current
  let mode_trace machine = List.rev machine.trace_rev

  let active_machine machine =
    Dfa.clone (String_map.find machine.current machine.modes)

  let create ?(max_trace_entries = 100_000) ~modes ~mode_transitions
      ~initial_mode () =
    if modes = [] then Error Empty_modes
    else if max_trace_entries < 0 then
      Error (Invalid_limit ("max_trace_entries", max_trace_entries))
    else
      let rec add_modes map = function
        | [] -> Ok map
        | (name, machine) :: rest ->
            if String_map.mem name map then Error (Duplicate_mode name)
            else add_modes (String_map.add name (Dfa.clone machine) map) rest
      in
      match add_modes String_map.empty modes with
      | Error error -> Error error
      | Ok mode_map when not (String_map.mem initial_mode mode_map) ->
          Error (Unknown_initial_mode initial_mode)
      | Ok mode_map ->
          let graph = Mode_graph.Labeled.create ~allow_self_loops:true () in
          String_map.iter
            (fun name _ -> Mode_graph.Labeled.add_node graph name)
            mode_map;
          let add_transition map ((source, trigger), target) =
            if not (String_map.mem source mode_map) then
              Error (Unknown_mode_source source)
            else if not (String_map.mem target mode_map) then
              Error (Unknown_mode_target target)
            else if Event_map.mem (source, trigger) map then
              Error (Duplicate_mode_transition (source, trigger))
            else
              match
                Mode_graph.Labeled.add_edge graph source target trigger
              with
              | Error _ -> Error (Unknown_mode_target target)
              | Ok () -> Ok (Event_map.add (source, trigger) target map)
          in
          (match
             Dfa.fold_result add_transition Event_map.empty mode_transitions
           with
          | Error error -> Error error
          | Ok transitions ->
              String_map.iter (fun _ machine -> Dfa.reset machine) mode_map;
              Ok
                {
                  modes = mode_map;
                  transitions;
                  initial_mode;
                  graph;
                  max_trace_entries;
                  current = initial_mode;
                  trace_rev = [];
                })

  let process machine event =
    Dfa.process (String_map.find machine.current machine.modes) event

  (* Switching is deliberately separate from DFA event processing. *)
  let switch_mode machine trigger =
    match Event_map.find_opt (machine.current, trigger) machine.transitions with
    | None -> Error (Missing_mode_transition (machine.current, trigger))
    | Some target ->
        assert
          (Mode_graph.Labeled.has_edge_with_label machine.graph machine.current
             target trigger);
        if List.length machine.trace_rev >= machine.max_trace_entries then
          Error (Trace_limit_exceeded machine.max_trace_entries)
        else
          let target_machine = String_map.find target machine.modes in
          Dfa.reset target_machine;
          let entry = { from_mode = machine.current; trigger; to_mode = target } in
          machine.current <- target;
          machine.trace_rev <- entry :: machine.trace_rev;
          Ok target

  let reset machine =
    String_map.iter (fun _ contained -> Dfa.reset contained) machine.modes;
    machine.current <- machine.initial_mode;
    machine.trace_rev <- []
end
