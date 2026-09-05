let unwrap message = function Ok value -> value | Error _ -> failwith message
let require condition message = if not condition then failwith message

let () =
  let transition : Coding_adventures_state_machine.dfa_transition =
    { source = "cold"; event = "boot"; target = "warm" }
  in
  let machine =
    unwrap "leaf DFA creation failed"
      (Coding_adventures_state_machine.Dfa.create
         ~states:[ "cold"; "warm" ] ~alphabet:[ "boot" ]
         ~transitions:[ transition ] ~initial:"cold" ~accepting:[ "warm" ] ())
  in
  let reachable =
    Coding_adventures_state_machine.Dfa.reachable_states machine
  in
  require (reachable = [ "cold"; "warm" ]) "unexpected reachable states";
  let current =
    unwrap "leaf DFA process failed"
      (Coding_adventures_state_machine.Dfa.process machine "boot")
  in
  require (current = "warm") "unexpected DFA state";
  print_endline
    {|{"schema_version":1,"fixture":"ocaml-state-machine-transitive-link-v1","reachable":["cold","warm"],"dfa_state":"warm","passes":true}|}
