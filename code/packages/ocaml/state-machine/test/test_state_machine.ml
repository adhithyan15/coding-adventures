open Coding_adventures_state_machine

let get = function
  | Ok value -> value
  | Error _ -> Alcotest.fail "unexpected error"

let expect_error predicate = function
  | Error error when predicate error -> ()
  | Error _ -> Alcotest.fail "unexpected error variant"
  | Ok _ -> Alcotest.fail "expected error"

let strings = Alcotest.list Alcotest.string

let dfa ?actions ?max_trace_entries ~states ~alphabet ~transitions ~initial
    ~accepting () =
  Dfa.create ?actions ?max_trace_entries ~states ~alphabet ~transitions ~initial
    ~accepting ()
  |> get

let test_dfa_runtime_and_actions () =
  let calls = ref [] in
  let unlock =
    {
      name = "unlock";
      run = (fun source event target -> calls := (source, event, target) :: !calls);
    }
  in
  let machine =
    dfa ~states:[ "unlocked"; "locked" ] ~alphabet:[ "push"; "coin" ]
      ~transitions:
        [
          { source = "locked"; event = "coin"; target = "unlocked" };
          { source = "locked"; event = "push"; target = "locked" };
          { source = "unlocked"; event = "coin"; target = "unlocked" };
          { source = "unlocked"; event = "push"; target = "locked" };
        ]
      ~actions:[ { source = "locked"; event = "coin"; action = unlock } ]
      ~initial:"locked" ~accepting:[ "unlocked" ] ()
  in
  Alcotest.(check string) "process target" "unlocked"
    (get (Dfa.process machine "coin"));
  Alcotest.(check (list (triple string string string))) "action args"
    [ ("locked", "coin", "unlocked") ] (List.rev !calls);
  Alcotest.(check int) "trace length" 1 (List.length (Dfa.trace machine));
  let before = Dfa.current_state machine, Dfa.trace machine, !calls in
  Alcotest.(check bool) "fresh acceptance" true
    (get (Dfa.accepts machine [ "coin" ]));
  Alcotest.check Alcotest.bool "acceptance is non-mutating" true
    (before = (Dfa.current_state machine, Dfa.trace machine, !calls));
  Dfa.reset machine;
  Alcotest.(check string) "reset state" "locked" (Dfa.current_state machine);
  Alcotest.(check int) "reset trace" 0 (List.length (Dfa.trace machine));
  expect_error (function Unknown_event "kick" -> true | _ -> false)
    (Dfa.process machine "kick")

let test_dfa_limits_and_introspection () =
  let transitions =
    [
      { source = "q0"; event = "0"; target = "q0" };
      { source = "q0"; event = "1"; target = "q1" };
      { source = "q1"; event = "0"; target = "q2" };
      { source = "q1"; event = "1"; target = "q0" };
      { source = "q2"; event = "0"; target = "q1" };
      { source = "q2"; event = "1"; target = "q2" };
    ]
  in
  let machine =
    dfa ~states:[ "q2"; "q0"; "q1" ] ~alphabet:[ "1"; "0" ] ~transitions
      ~initial:"q0" ~accepting:[ "q0" ] ()
  in
  List.iter
    (fun word -> Alcotest.(check bool) "accepted" true (get (Dfa.accepts machine word)))
    [ []; [ "0" ]; [ "1"; "1" ]; [ "1"; "1"; "0" ] ];
  List.iter
    (fun word -> Alcotest.(check bool) "rejected" false (get (Dfa.accepts machine word)))
    [ [ "1"; "0" ]; [ "1"; "0"; "1" ] ];
  Alcotest.check strings "ordered states" [ "q0"; "q1"; "q2" ]
    (Dfa.states machine);
  Alcotest.check strings "reachable" [ "q0"; "q1"; "q2" ]
    (Dfa.reachable_states machine);
  Alcotest.(check bool) "complete" true (Dfa.is_complete machine);
  Alcotest.check strings "valid" [] (Dfa.validate machine);
  Alcotest.(check bool) "table populated" true (List.length (Dfa.to_table machine) = 4);
  Alcotest.(check bool) "ascii deterministic" true
    (String.length (Dfa.to_ascii machine) > 0);
  Alcotest.(check bool) "dot deterministic" true
    (String.starts_with ~prefix:"digraph DFA" (Dfa.to_dot machine));
  let limited =
    dfa ~max_trace_entries:1 ~states:[ "a" ] ~alphabet:[ "x" ]
      ~transitions:[ { source = "a"; event = "x"; target = "a" } ]
      ~initial:"a" ~accepting:[] ()
  in
  ignore (get (Dfa.process limited "x"));
  expect_error (function Trace_limit_exceeded 1 -> true | _ -> false)
    (Dfa.process limited "x");
  Alcotest.(check int) "failed step does not trace" 1
    (List.length (Dfa.trace limited));
  let incomplete =
    dfa ~states:[ "a"; "b" ] ~alphabet:[ "x" ] ~transitions:[] ~initial:"a"
      ~accepting:[ "b" ] ()
  in
  expect_error (function Missing_transition ("a", "x") -> true | _ -> false)
    (Dfa.process incomplete "x");
  Alcotest.(check bool) "missing acceptance rejects" false
    (get (Dfa.accepts incomplete [ "x" ]));
  Alcotest.(check bool) "incomplete" false (Dfa.is_complete incomplete);
  Alcotest.(check bool) "warnings" true (Dfa.validate incomplete <> [])

let test_nfa_epsilon_and_subset () =
  let machine =
    Nfa.create ~states:[ "q0"; "q1"; "q2"; "q3"; "q4" ]
      ~alphabet:[ "a"; "b" ]
      ~transitions:
        [
          { source = "q0"; event = None; targets = [ "q1" ] };
          { source = "q1"; event = None; targets = [ "q2" ] };
          { source = "q2"; event = None; targets = [ "q1" ] };
          { source = "q2"; event = Some "a"; targets = [ "q3"; "q2" ] };
          { source = "q3"; event = Some "b"; targets = [ "q4" ] };
        ]
      ~initial:"q0" ~accepting:[ "q4" ] ()
    |> get
  in
  Alcotest.check strings "initial epsilon closure" [ "q0"; "q1"; "q2" ]
    (Nfa.current_states machine);
  Alcotest.check strings "branched closure" [ "q1"; "q2"; "q3" ]
    (get (Nfa.process machine "a"));
  Alcotest.(check bool) "accepts ab" true (get (Nfa.accepts machine [ "a"; "b" ]));
  let before = Nfa.current_states machine, Nfa.trace machine in
  let converted = get (Nfa.to_dfa machine) in
  Alcotest.check Alcotest.bool "conversion is non-mutating" true
    (before = (Nfa.current_states machine, Nfa.trace machine));
  Alcotest.(check bool) "converted agrees" true
    (get (Dfa.accepts converted [ "a"; "b" ]));
  Nfa.reset machine;
  Alcotest.(check int) "nfa reset trace" 0 (List.length (Nfa.trace machine));
  let constrained =
    Nfa.create ~max_trace_state_cells:5
      ~states:[ "q0"; "q1"; "q2"; "q3" ] ~alphabet:[ "a" ]
      ~transitions:
        [
          { source = "q0"; event = None; targets = [ "q1"; "q2" ] };
          { source = "q1"; event = None; targets = [ "q2" ] };
          { source = "q2"; event = Some "a"; targets = [ "q1"; "q3" ] };
        ]
      ~initial:"q0" ~accepting:[] ()
    |> get
  in
  let states_before = Nfa.current_states constrained in
  expect_error (function Trace_state_limit_exceeded 5 -> true | _ -> false)
    (Nfa.process constrained "a");
  Alcotest.check strings "cell failure atomic" states_before
    (Nfa.current_states constrained)

let test_nfa_validation_and_limits () =
  let base ?(max_generated_states = 4_096) ?(max_trace_entries = 100_000)
      ?(max_trace_state_cells = 1_000_000) states alphabet transitions initial
      accepting =
    Nfa.create ~max_generated_states ~max_trace_entries ~max_trace_state_cells
      ~states ~alphabet ~transitions ~initial ~accepting ()
  in
  expect_error (function Empty_states -> true | _ -> false)
    (base [] [] [] "q" []);
  expect_error
    (function Invalid_limit ("max_generated_states", -1) -> true | _ -> false)
    (base ~max_generated_states:(-1) [ "q" ] [] [] "q" []);
  expect_error (function Unknown_initial "z" -> true | _ -> false)
    (base [ "q" ] [] [] "z" []);
  expect_error (function Unknown_accepting "z" -> true | _ -> false)
    (base [ "q" ] [] [] "q" [ "z" ]);
  expect_error (function Unknown_transition_source "z" -> true | _ -> false)
    (base [ "q" ] [ "x" ]
       [ { source = "z"; event = Some "x"; targets = [ "q" ] } ]
       "q" []);
  expect_error (function Unknown_transition_event "z" -> true | _ -> false)
    (base [ "q" ] [ "x" ]
       [ { source = "q"; event = Some "z"; targets = [ "q" ] } ]
       "q" []);
  expect_error (function Unknown_transition_target "z" -> true | _ -> false)
    (base [ "q" ] [ "x" ]
       [ { source = "q"; event = Some "x"; targets = [ "z" ] } ]
       "q" []);
  expect_error
    (function Duplicate_transition ("q", Some "x") -> true | _ -> false)
    (base [ "q" ] [ "x" ]
       [
         { source = "q"; event = Some "x"; targets = [ "q" ] };
         { source = "q"; event = Some "x"; targets = [ "q" ] };
       ]
       "q" []);
  let machine =
    base [ "q"; "a" ] [ ""; "x" ]
      [
        { source = "q"; event = None; targets = [ "a" ] };
        { source = "a"; event = Some ""; targets = [ "a" ] };
        { source = "a"; event = Some "x"; targets = [ "q" ] };
      ]
      "q" [ "a" ]
    |> get
  in
  Alcotest.check strings "nfa states accessor" [ "a"; "q" ] (Nfa.states machine);
  Alcotest.check strings "nfa alphabet accessor" [ ""; "x" ]
    (Nfa.alphabet machine);
  Alcotest.(check int) "nfa transitions accessor" 3
    (List.length (Nfa.transitions machine));
  Alcotest.(check string) "nfa initial accessor" "q" (Nfa.initial machine);
  Alcotest.check strings "nfa accepting accessor" [ "a" ]
    (Nfa.accepting machine);
  Alcotest.check strings "closure accessor" [ "a"; "q" ]
    (get (Nfa.epsilon_closure machine [ "q" ]));
  expect_error (function Unknown_state "z" -> true | _ -> false)
    (Nfa.epsilon_closure machine [ "z" ]);
  Alcotest.(check bool) "empty string is named event" true
    (get (Nfa.accepts machine [ "" ]));
  Alcotest.(check int) "nfa sequence records" 2
    (List.length (get (Nfa.process_sequence machine [ ""; "x" ])));
  expect_error (function Unknown_event "z" -> true | _ -> false)
    (Nfa.process machine "z");
  Alcotest.(check bool) "nfa dot rendered" true
    (String.starts_with ~prefix:"digraph NFA" (Nfa.to_dot machine));
  let no_trace =
    base ~max_trace_entries:0 [ "q" ] [ "x" ]
      [ { source = "q"; event = Some "x"; targets = [ "q" ] } ]
      "q" []
    |> get
  in
  expect_error (function Trace_limit_exceeded 0 -> true | _ -> false)
    (Nfa.process no_trace "x");
  let no_subsets =
    base ~max_generated_states:0 [ "q" ] [] [] "q" [] |> get
  in
  expect_error (function Subset_limit_exceeded 0 -> true | _ -> false)
    (Nfa.to_dfa no_subsets);
  let one_subset =
    base ~max_generated_states:1 [ "q"; "a" ] [ "x" ]
      [ { source = "q"; event = Some "x"; targets = [ "a" ] } ]
      "q" []
    |> get
  in
  expect_error (function Subset_limit_exceeded 1 -> true | _ -> false)
    (Nfa.to_dfa one_subset)

let test_minimization () =
  let source =
    dfa ~states:[ "q0"; "q1"; "q2"; "dead" ] ~alphabet:[ "x" ]
      ~transitions:
        [
          { source = "q0"; event = "x"; target = "q1" };
          { source = "q1"; event = "x"; target = "q1" };
          { source = "q2"; event = "x"; target = "q2" };
          { source = "dead"; event = "x"; target = "dead" };
        ]
      ~initial:"q0" ~accepting:[ "q1"; "q2" ] ()
  in
  let reduced = minimize source in
  Alcotest.check strings "unreachable removed" [ "q0"; "q1" ]
    (Dfa.states reduced);
  Alcotest.(check bool) "language preserved" true
    (get (Dfa.accepts reduced [ "x"; "x" ]));
  Alcotest.check strings "source unchanged" [ "dead"; "q0"; "q1"; "q2" ]
    (Dfa.states source)

let test_minimization_merges_reachable_states () =
  let source =
    dfa ~states:[ "q0"; "q1"; "q2" ] ~alphabet:[ "a"; "b" ]
      ~transitions:
        [
          { source = "q0"; event = "a"; target = "q1" };
          { source = "q0"; event = "b"; target = "q2" };
          { source = "q1"; event = "a"; target = "q1" };
          { source = "q1"; event = "b"; target = "q1" };
          { source = "q2"; event = "a"; target = "q2" };
          { source = "q2"; event = "b"; target = "q2" };
        ]
      ~initial:"q0" ~accepting:[ "q1"; "q2" ] ()
  in
  let reduced = minimize source in
  Alcotest.check strings "equivalent states merged" [ "q0"; "{q1,q2}" ]
    (Dfa.states reduced);
  Alcotest.(check bool) "a accepted" true (get (Dfa.accepts reduced [ "a" ]));
  Alcotest.(check bool) "empty rejected" false (get (Dfa.accepts reduced []))

let test_pda_balanced_and_limits () =
  let transitions =
    [
      {
        Pda.source = "q";
        event = Some "(";
        stack_read = "$";
        target = "q";
        stack_push = [ "$"; "(" ];
      };
      {
        Pda.source = "q";
        event = Some "(";
        stack_read = "(";
        target = "q";
        stack_push = [ "("; "(" ];
      };
      {
        Pda.source = "q";
        event = Some ")";
        stack_read = "(";
        target = "q";
        stack_push = [];
      };
      {
        Pda.source = "q";
        event = None;
        stack_read = "$";
        target = "accept";
        stack_push = [ "$" ];
      };
    ]
  in
  let machine =
    Pda.create ~states:[ "q"; "accept" ] ~input_alphabet:[ "("; ")" ]
      ~stack_alphabet:[ "$"; "(" ] ~transitions ~initial:"q"
      ~initial_stack_symbol:"$" ~accepting:[ "accept" ] ()
    |> get
  in
  List.iter
    (fun word -> Alcotest.(check bool) "balanced" true (get (Pda.accepts machine word)))
    [ []; [ "("; ")" ]; [ "("; "("; ")"; ")" ] ];
  List.iter
    (fun word -> Alcotest.(check bool) "unbalanced" false (get (Pda.accepts machine word)))
    [ [ ")" ]; [ "(" ]; [ ")"; "(" ] ];
  ignore (get (Pda.process machine "("));
  Alcotest.check strings "bottom-to-top stack" [ "$"; "(" ]
    (Pda.stack machine);
  Alcotest.(check (option string)) "stack top" (Some "(")
    (Pda.stack_top machine);
  let before = Pda.current_state machine, Pda.stack machine, Pda.trace machine in
  expect_error (function Unknown_event "x" -> true | _ -> false)
    (Pda.process machine "x");
  Alcotest.check Alcotest.bool "pda failure atomic" true
    (before = (Pda.current_state machine, Pda.stack machine, Pda.trace machine));
  Pda.reset machine;
  Alcotest.check strings "pda reset stack" [ "$" ] (Pda.stack machine);
  let shallow =
    Pda.create ~max_stack_depth:1 ~states:[ "q" ] ~input_alphabet:[ "(" ]
      ~stack_alphabet:[ "$"; "(" ]
      ~transitions:[ List.hd transitions ] ~initial:"q"
      ~initial_stack_symbol:"$" ~accepting:[] ()
    |> get
  in
  expect_error (function Stack_limit_exceeded 1 -> true | _ -> false)
    (Pda.process shallow "(")

let pda_row source event stack_read target stack_push : Pda.transition =
  { source; event; stack_read; target; stack_push }

let test_pda_validation_and_epsilon () =
  let base ?(max_stack_depth = 4_096) ?(max_trace_entries = 2_048)
      ?(max_epsilon_steps = 10_000) states input_alphabet stack_alphabet
      transitions initial initial_stack_symbol accepting =
    Pda.create ~max_stack_depth ~max_trace_entries ~max_epsilon_steps ~states
      ~input_alphabet ~stack_alphabet ~transitions ~initial
      ~initial_stack_symbol ~accepting ()
  in
  expect_error (function Empty_states -> true | _ -> false)
    (base [] [] [ "$" ] [] "q" "$" []);
  expect_error
    (function Invalid_limit ("max_stack_depth", 0) -> true | _ -> false)
    (base ~max_stack_depth:0 [ "q" ] [] [ "$" ] [] "q" "$" []);
  expect_error (function Unknown_initial "z" -> true | _ -> false)
    (base [ "q" ] [] [ "$" ] [] "z" "$" []);
  expect_error
    (function Unknown_initial_stack_symbol "z" -> true | _ -> false)
    (base [ "q" ] [] [ "$" ] [] "q" "z" []);
  expect_error (function Unknown_accepting "z" -> true | _ -> false)
    (base [ "q" ] [] [ "$" ] [] "q" "$" [ "z" ]);
  expect_error (function Unknown_transition_source "z" -> true | _ -> false)
    (base [ "q" ] [ "x" ] [ "$" ]
       [ pda_row "z" (Some "x") "$" "q" [ "$" ] ]
       "q" "$" []);
  expect_error (function Unknown_transition_target "z" -> true | _ -> false)
    (base [ "q" ] [ "x" ] [ "$" ]
       [ pda_row "q" (Some "x") "$" "z" [ "$" ] ]
       "q" "$" []);
  expect_error (function Unknown_transition_event "z" -> true | _ -> false)
    (base [ "q" ] [ "x" ] [ "$" ]
       [ pda_row "q" (Some "z") "$" "q" [ "$" ] ]
       "q" "$" []);
  expect_error (function Unknown_stack_read "z" -> true | _ -> false)
    (base [ "q" ] [ "x" ] [ "$" ]
       [ pda_row "q" (Some "x") "z" "q" [ "$" ] ]
       "q" "$" []);
  expect_error (function Unknown_stack_push "z" -> true | _ -> false)
    (base [ "q" ] [ "x" ] [ "$" ]
       [ pda_row "q" (Some "x") "$" "q" [ "z" ] ]
       "q" "$" []);
  let duplicate = pda_row "q" (Some "x") "$" "q" [ "$" ] in
  expect_error
    (function
      | Duplicate_pda_transition ("q", Some "x", "$") -> true
      | _ -> false)
    (base [ "q" ] [ "x" ] [ "$" ] [ duplicate; duplicate ] "q" "$" []);
  let machine =
    base [ "q"; "done" ] [ "pop" ] [ "$" ]
      [
        pda_row "q" (Some "pop") "$" "q" [];
        pda_row "q" None "$" "done" [ "$" ];
      ]
      "q" "$" [ "done" ]
    |> get
  in
  Alcotest.check strings "pda states accessor" [ "done"; "q" ]
    (Pda.states machine);
  Alcotest.check strings "pda input accessor" [ "pop" ]
    (Pda.input_alphabet machine);
  Alcotest.check strings "pda stack alphabet accessor" [ "$" ]
    (Pda.stack_alphabet machine);
  Alcotest.check strings "pda accepting accessor" [ "done" ]
    (Pda.accepting machine);
  Alcotest.(check int) "pda transition accessor" 2
    (List.length (Pda.transitions machine));
  ignore (get (Pda.process machine "pop"));
  Alcotest.(check (option string)) "empty stack top" None
    (Pda.stack_top machine);
  expect_error
    (function Missing_pda_transition ("q", "pop", None) -> true | _ -> false)
    (Pda.process machine "pop");
  Pda.reset machine;
  Alcotest.(check int) "sequence includes epsilon" 1
    (List.length (get (Pda.process_sequence machine [])));
  let no_trace =
    base ~max_trace_entries:0 [ "q" ] [ "x" ] [ "$" ]
      [ pda_row "q" (Some "x") "$" "q" [ "$" ] ]
      "q" "$" []
    |> get
  in
  expect_error (function Trace_limit_exceeded 0 -> true | _ -> false)
    (Pda.process no_trace "x");
  let cycle =
    base ~max_epsilon_steps:1 [ "q" ] [] [ "$" ]
      [ pda_row "q" None "$" "q" [ "$" ] ]
      "q" "$" []
    |> get
  in
  expect_error (function Epsilon_limit_exceeded 1 -> true | _ -> false)
    (Pda.process_sequence cycle []);
  expect_error (function Epsilon_limit_exceeded 1 -> true | _ -> false)
    (Pda.accepts cycle [])

let modal_dfa name event =
  dfa ~states:[ name ^ "0"; name ^ "1" ] ~alphabet:[ event ]
    ~transitions:
      [
        { source = name ^ "0"; event; target = name ^ "1" };
        { source = name ^ "1"; event; target = name ^ "1" };
      ]
    ~initial:(name ^ "0") ~accepting:[ name ^ "1" ] ()

let test_modal_explicit_switching () =
  let data = modal_dfa "d" "text" in
  let tag = modal_dfa "t" "name" in
  let machine =
    Modal.create ~modes:[ ("DATA", data); ("TAG", tag) ]
      ~mode_transitions:
        [ (("DATA", "open"), "TAG"); (("TAG", "close"), "DATA") ]
      ~initial_mode:"DATA" ()
    |> get
  in
  Alcotest.(check string) "active process" "d1"
    (get (Modal.process machine "text"));
  expect_error (function Unknown_event "open" -> true | _ -> false)
    (Modal.process machine "open");
  Alcotest.(check string) "process never switches" "DATA"
    (Modal.current_mode machine);
  Alcotest.(check string) "explicit switch" "TAG"
    (get (Modal.switch_mode machine "open"));
  Alcotest.(check string) "target reset" "t0"
    (Dfa.current_state (Modal.active_machine machine));
  ignore (get (Modal.process machine "name"));
  ignore (get (Modal.switch_mode machine "close"));
  ignore (get (Modal.switch_mode machine "open"));
  Alcotest.(check string) "re-entry reset" "t0"
    (Dfa.current_state (Modal.active_machine machine));
  expect_error (function Missing_mode_transition ("TAG", "nope") -> true | _ -> false)
    (Modal.switch_mode machine "nope");
  Modal.reset machine;
  Alcotest.(check string) "modal reset mode" "DATA" (Modal.current_mode machine);
  Alcotest.(check int) "modal reset trace" 0 (List.length (Modal.mode_trace machine))

let test_modal_validation_and_limits () =
  let one = modal_dfa "q" "x" in
  expect_error
    (function Invalid_limit ("max_trace_entries", -1) -> true | _ -> false)
    (Modal.create ~max_trace_entries:(-1) ~modes:[ ("A", one) ]
       ~mode_transitions:[] ~initial_mode:"A" ());
  expect_error (function Duplicate_mode "A" -> true | _ -> false)
    (Modal.create ~modes:[ ("A", one); ("A", one) ] ~mode_transitions:[]
       ~initial_mode:"A" ());
  expect_error (function Unknown_initial_mode "Z" -> true | _ -> false)
    (Modal.create ~modes:[ ("A", one) ] ~mode_transitions:[]
       ~initial_mode:"Z" ());
  expect_error (function Unknown_mode_source "Z" -> true | _ -> false)
    (Modal.create ~modes:[ ("A", one) ]
       ~mode_transitions:[ (("Z", "go"), "A") ] ~initial_mode:"A" ());
  expect_error (function Unknown_mode_target "Z" -> true | _ -> false)
    (Modal.create ~modes:[ ("A", one) ]
       ~mode_transitions:[ (("A", "go"), "Z") ] ~initial_mode:"A" ());
  expect_error
    (function Duplicate_mode_transition ("A", "go") -> true | _ -> false)
    (Modal.create ~modes:[ ("A", one) ]
       ~mode_transitions:[ (("A", "go"), "A"); (("A", "go"), "A") ]
       ~initial_mode:"A" ());
  let limited =
    Modal.create ~max_trace_entries:0 ~modes:[ ("A", one) ]
      ~mode_transitions:[ (("A", "stay"), "A") ] ~initial_mode:"A" ()
    |> get
  in
  Alcotest.check strings "mode names" [ "A" ] (Modal.mode_names limited);
  expect_error (function Trace_limit_exceeded 0 -> true | _ -> false)
    (Modal.switch_mode limited "stay");
  Alcotest.(check string) "limit leaves mode" "A" (Modal.current_mode limited)

let test_constructor_errors () =
  expect_error (function Empty_states -> true | _ -> false)
    (Dfa.create ~states:[] ~alphabet:[] ~transitions:[] ~initial:"q"
       ~accepting:[] ());
  expect_error (function Unknown_initial "q" -> true | _ -> false)
    (Dfa.create ~states:[ "x" ] ~alphabet:[] ~transitions:[] ~initial:"q"
       ~accepting:[] ());
  expect_error (function Invalid_limit ("max_trace_entries", -1) -> true | _ -> false)
    (Dfa.create ~max_trace_entries:(-1) ~states:[ "q" ] ~alphabet:[]
       ~transitions:[] ~initial:"q" ~accepting:[] ());
  expect_error (function Duplicate_transition ("q", Some "x") -> true | _ -> false)
    (Dfa.create ~states:[ "q" ] ~alphabet:[ "x" ]
       ~transitions:
         [
           { source = "q"; event = "x"; target = "q" };
           { source = "q"; event = "x"; target = "q" };
         ]
       ~initial:"q" ~accepting:[] ());
  expect_error (function Unknown_accepting "z" -> true | _ -> false)
    (Dfa.create ~states:[ "q" ] ~alphabet:[] ~transitions:[] ~initial:"q"
       ~accepting:[ "z" ] ());
  expect_error (function Unknown_transition_source "z" -> true | _ -> false)
    (Dfa.create ~states:[ "q" ] ~alphabet:[ "x" ]
       ~transitions:[ { source = "z"; event = "x"; target = "q" } ]
       ~initial:"q" ~accepting:[] ());
  expect_error (function Unknown_transition_event "z" -> true | _ -> false)
    (Dfa.create ~states:[ "q" ] ~alphabet:[ "x" ]
       ~transitions:[ { source = "q"; event = "z"; target = "q" } ]
       ~initial:"q" ~accepting:[] ());
  expect_error (function Unknown_transition_target "z" -> true | _ -> false)
    (Dfa.create ~states:[ "q" ] ~alphabet:[ "x" ]
       ~transitions:[ { source = "q"; event = "x"; target = "z" } ]
       ~initial:"q" ~accepting:[] ());
  let action = { name = "bad"; run = (fun _ _ _ -> ()) } in
  expect_error (function Action_without_transition ("q", "x") -> true | _ -> false)
    (Dfa.create ~actions:[ { source = "q"; event = "x"; action } ]
       ~states:[ "q" ] ~alphabet:[ "x" ] ~transitions:[] ~initial:"q"
       ~accepting:[] ());
  expect_error (function Empty_modes -> true | _ -> false)
    (Modal.create ~modes:[] ~mode_transitions:[] ~initial_mode:"none" ())

let test_dfa_sequence_and_snapshots () =
  let machine =
    dfa ~states:[ "b"; "a" ] ~alphabet:[ "y"; "x" ]
      ~transitions:
        [
          { source = "a"; event = "x"; target = "b" };
          { source = "b"; event = "y"; target = "a" };
        ]
      ~initial:"a" ~accepting:[ "b" ] ()
  in
  Alcotest.check strings "alphabet snapshot" [ "x"; "y" ] (Dfa.alphabet machine);
  Alcotest.(check string) "initial accessor" "a" (Dfa.initial machine);
  Alcotest.check strings "accepting accessor" [ "b" ] (Dfa.accepting machine);
  Alcotest.(check int) "transition snapshot" 2 (List.length (Dfa.transitions machine));
  Alcotest.(check int) "sequence records" 2
    (List.length (get (Dfa.process_sequence machine [ "x"; "y" ])));
  let before = Dfa.current_state machine, Dfa.trace machine in
  expect_error (function Unknown_event "z" -> true | _ -> false)
    (Dfa.process_sequence machine [ "x"; "z" ]);
  Alcotest.check Alcotest.bool "sequence preflight atomic" true
    (before = (Dfa.current_state machine, Dfa.trace machine))

let () =
  Alcotest.run "coding-adventures-state-machine"
    [
      ( "state machines",
        [
          Alcotest.test_case "dfa runtime and actions" `Quick
            test_dfa_runtime_and_actions;
          Alcotest.test_case "dfa limits and introspection" `Quick
            test_dfa_limits_and_introspection;
          Alcotest.test_case "dfa sequence and snapshots" `Quick
            test_dfa_sequence_and_snapshots;
          Alcotest.test_case "nfa epsilon and subset" `Quick
            test_nfa_epsilon_and_subset;
          Alcotest.test_case "nfa validation and limits" `Quick
            test_nfa_validation_and_limits;
          Alcotest.test_case "minimization" `Quick test_minimization;
          Alcotest.test_case "minimization merges reachable states" `Quick
            test_minimization_merges_reachable_states;
          Alcotest.test_case "pda balanced and limits" `Quick
            test_pda_balanced_and_limits;
          Alcotest.test_case "pda validation and epsilon" `Quick
            test_pda_validation_and_epsilon;
          Alcotest.test_case "modal explicit switching" `Quick
            test_modal_explicit_switching;
          Alcotest.test_case "modal validation and limits" `Quick
            test_modal_validation_and_limits;
          Alcotest.test_case "constructor errors" `Quick
            test_constructor_errors;
        ] );
    ]
