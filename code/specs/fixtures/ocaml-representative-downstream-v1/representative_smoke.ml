let unwrap message = function Ok value -> value | Error _ -> failwith message
let require condition message = if not condition then failwith message

module String_node = struct
  type t = string

  let compare = String.compare
end

module Undirected = Coding_adventures_graph.Make (String_node)
module Directed = Coding_adventures_directed_graph.Make (String_node)

let () =
  let gate =
    unwrap "logic-gates xor failed"
      (Coding_adventures_logic_gates.Basic.xor_gate 1 0)
  in
  require (gate = 1) "unexpected xor result";
  let graph =
    Undirected.create ~representation:Undirected.Adjacency_matrix ()
  in
  unwrap "graph add_edge failed"
    (Undirected.add_edge ~weight:0.0 graph "a" "b");
  let bfs = unwrap "graph bfs failed" (Undirected.bfs graph "a") in
  require (bfs = [ "a"; "b" ]) "unexpected graph traversal";
  let directed = Directed.create () in
  unwrap "directed-graph add_edge failed"
    (Directed.add_edge directed "parse" "emit");
  let order =
    unwrap "directed-graph topological_sort failed"
      (Directed.topological_sort directed)
  in
  require (order = [ "parse"; "emit" ]) "unexpected topological order";
  let transition : Coding_adventures_state_machine.dfa_transition =
    { source = "locked"; event = "coin"; target = "open" }
  in
  let machine =
    unwrap "state-machine creation failed"
      (Coding_adventures_state_machine.Dfa.create
         ~states:[ "locked"; "open" ] ~alphabet:[ "coin" ]
         ~transitions:[ transition ] ~initial:"locked" ~accepting:[ "open" ] ())
  in
  let current =
    unwrap "state-machine process failed"
      (Coding_adventures_state_machine.Dfa.process machine "coin")
  in
  require (current = "open") "unexpected DFA state";
  print_endline
    {|{"schema_version":1,"fixture":"ocaml-representative-downstream-v1","logic_gate":1,"graph_bfs":["a","b"],"directed_order":["parse","emit"],"dfa_state":"open","passes":true}|}
