module Graph = Coding_adventures_directed_graph.Make (String)
open Graph

let get = function
  | Ok value -> value
  | Error _ -> Alcotest.fail "unexpected error"

let expect_error = function
  | Error _ -> ()
  | Ok _ -> Alcotest.fail "expected error"

let strings = Alcotest.list Alcotest.string

let test_structure_and_properties () =
  let graph = create () in
  List.iter (add_node graph) [ "C"; "A"; "B" ];
  Alcotest.check strings "ordered nodes" [ "A"; "B"; "C" ] (nodes graph);
  Alcotest.check Alcotest.int "size" 3 (size graph);
  add_edge ~weight:2.
    ~properties:(properties [ ("kind", String "road") ])
    graph "A" "B"
  |> get;
  add_edge ~weight:3. graph "B" "A" |> get;
  Alcotest.check Alcotest.bool "directions independent" true
    (has_edge graph "A" "B" && has_edge graph "B" "A");
  Alcotest.check (Alcotest.float 0.) "forward weight" 2.
    (get (edge_weight graph "A" "B"));
  Alcotest.check strings "successors" [ "B" ] (get (successors graph "A"));
  Alcotest.check strings "predecessors" [ "B" ] (get (predecessors graph "A"));
  Alcotest.check Alcotest.int "out degree" 1 (get (out_degree graph "A"));
  Alcotest.check Alcotest.int "in degree" 1 (get (in_degree graph "A"));
  set_graph_property graph "name" (String "demo");
  set_node_property graph "A" "active" (Bool true) |> get;
  set_edge_property graph "A" "B" "weight" (Number 4.) |> get;
  Alcotest.check Alcotest.bool "property synchronization" true
    (property_find_opt "name" (graph_properties graph) = Some (String "demo")
    && property_find_opt "active" (get (node_properties graph "A"))
       = Some (Bool true)
    && get (edge_weight graph "A" "B") = 4.);
  remove_edge_property graph "A" "B" "weight" |> get;
  remove_node_property graph "A" "active" |> get;
  remove_graph_property graph "name";
  Alcotest.check (Alcotest.float 0.) "weight reset" 1.
    (get (edge_weight graph "A" "B"));
  expect_error (add_edge ~weight:(-1.) graph "A" "B");
  expect_error (add_edge ~weight:nan graph "A" "B");
  expect_error (node_properties graph "missing");
  expect_error (edge_properties graph "A" "missing");
  expect_error (edge_weight graph "A" "missing");
  expect_error (set_node_property graph "missing" "x" Null);
  expect_error (remove_node_property graph "missing" "x");
  expect_error (set_edge_property graph "A" "missing" "x" Null);
  expect_error (remove_edge_property graph "A" "missing" "x");
  expect_error (set_edge_property graph "A" "B" "weight" (Number (-1.)));
  expect_error (set_edge_property graph "A" "B" "weight" (String "heavy"));
  set_edge_property graph "A" "B" "label" (String "forward") |> get;
  remove_edge_property graph "A" "B" "label" |> get;
  let implicit = create () in
  add_edge implicit "left" "right" |> get;
  Alcotest.check strings "edge creates endpoints" [ "left"; "right" ]
    (nodes implicit);
  remove_edge graph "A" "B" |> get;
  expect_error (remove_edge graph "A" "B");
  remove_node graph "B" |> get;
  Alcotest.check Alcotest.bool "incident edges removed" false
    (has_edge graph "B" "A");
  expect_error (remove_node graph "B")

let diamond () =
  let graph = create () in
  List.iter (add_node graph) [ "D"; "B"; "A"; "C" ];
  List.iter
    (fun (left, right) -> add_edge graph left right |> get)
    [ ("A", "B"); ("A", "C"); ("B", "D"); ("C", "D") ];
  graph

let test_dag_algorithms () =
  let graph = diamond () in
  let before = edges graph in
  Alcotest.check strings "bfs" [ "A"; "B"; "C"; "D" ] (get (bfs graph "A"));
  Alcotest.check strings "dfs" [ "A"; "B"; "D"; "C" ] (get (dfs graph "A"));
  Alcotest.check strings "topological order" [ "A"; "B"; "C"; "D" ]
    (get (topological_sort graph));
  Alcotest.check strings "closure" [ "B"; "C"; "D" ]
    (get (transitive_closure graph "A"));
  Alcotest.check strings "dependents" [ "A"; "B"; "C" ]
    (get (transitive_dependents graph "D"));
  Alcotest.check (Alcotest.list strings) "independent groups"
    [ [ "A" ]; [ "B"; "C" ]; [ "D" ] ]
    (get (independent_groups graph));
  Alcotest.check strings "affected" [ "A"; "B"; "C"; "D" ]
    (affected_nodes graph [ "D"; "missing" ]);
  Alcotest.check Alcotest.bool "acyclic" false (has_cycle graph);
  Alcotest.check Alcotest.bool "algorithms do not mutate" true
    (before = edges graph);
  expect_error (bfs graph "missing");
  expect_error (dfs graph "missing");
  expect_error (transitive_closure graph "missing");
  expect_error (transitive_dependents graph "missing");
  expect_error (successors graph "missing");
  expect_error (predecessors graph "missing");
  expect_error (neighbors_weighted graph "missing");
  expect_error (out_degree graph "missing");
  expect_error (in_degree graph "missing")

let test_depth_first_branch_order () =
  let graph = create () in
  List.iter
    (fun (left, right) -> add_edge graph left right |> get)
    [ ("A", "B"); ("A", "C"); ("A", "D"); ("B", "C"); ("B", "E") ];
  Alcotest.check strings "branch discovery order" [ "A"; "B"; "C"; "E"; "D" ]
    (get (dfs graph "A"));
  Alcotest.check Alcotest.bool "cross edge to black is not a cycle" false
    (has_cycle graph)

let test_cycles_and_components () =
  let graph = create () in
  List.iter (add_node graph) [ "A"; "B"; "C"; "D"; "E" ];
  List.iter
    (fun (left, right) -> add_edge graph left right |> get)
    [ ("A", "B"); ("B", "A"); ("B", "C"); ("C", "D"); ("D", "C") ];
  Alcotest.check Alcotest.bool "cycle" true (has_cycle graph);
  expect_error (topological_sort graph);
  expect_error (independent_groups graph);
  Alcotest.check (Alcotest.list strings) "ordered components"
    [ [ "A"; "B" ]; [ "C"; "D" ]; [ "E" ] ]
    (strongly_connected_components graph);
  let strict = create () in
  add_node strict "A";
  expect_error (add_edge strict "A" "A");
  let permissive = create ~allow_self_loops:true () in
  add_node permissive "A";
  add_edge permissive "A" "A" |> get;
  Alcotest.check Alcotest.bool "allowed self-loop cycles" true
    (has_cycle permissive)

let test_labeled_edges () =
  let graph = Labeled.create () in
  List.iter (Labeled.add_node graph) [ "A"; "B"; "C" ];
  Alcotest.check Alcotest.bool "labeled node" true (Labeled.has_node graph "A");
  Alcotest.check strings "labeled nodes" [ "A"; "B"; "C" ] (Labeled.nodes graph);
  Labeled.add_edge graph "A" "B" "red" |> get;
  Labeled.add_edge ~weight:2. graph "A" "B" "blue" |> get;
  Labeled.add_edge ~weight:9. graph "A" "B" "blue" |> get;
  Labeled.add_edge graph "B" "A" "reverse" |> get;
  Alcotest.check strings "labels ordered" [ "blue"; "red" ]
    (Labeled.edge_labels graph "A" "B");
  Alcotest.check Alcotest.bool "specific label" true
    (Labeled.has_edge_with_label graph "A" "B" "red");
  Alcotest.check Alcotest.bool "missing label" false
    (Labeled.has_edge_with_label graph "A" "C" "red");
  Alcotest.check Alcotest.int "three labeled rows" 3
    (List.length (Labeled.edges_labeled graph));
  Alcotest.check (Alcotest.float 0.) "duplicate label is idempotent" 2.
    (match Labeled.edges_labeled graph with
    | ("A", "B", "blue", weight) :: _ -> weight
    | _ -> Alcotest.fail "missing labeled edge");
  Labeled.remove_edge_label graph "A" "B" "red" |> get;
  Alcotest.check strings "partial removal" [ "blue" ]
    (Labeled.edge_labels graph "A" "B");
  expect_error (Labeled.remove_edge_label graph "A" "B" "missing");
  Labeled.remove_edge_label graph "A" "B" "blue" |> get;
  Alcotest.check Alcotest.bool "last label removes structure" false
    (Labeled.has_edge graph "A" "B");
  Labeled.add_edge graph "A" "C" "one" |> get;
  Labeled.remove_edge graph "A" "C" |> get;
  Alcotest.check strings "missing labels are empty" []
    (Labeled.edge_labels graph "A" "C");
  Labeled.remove_node graph "A" |> get;
  Alcotest.check Alcotest.bool "reverse labels cleaned" false
    (Labeled.has_edge graph "B" "A");
  expect_error (Labeled.remove_node graph "A");
  expect_error (Labeled.remove_edge graph "A" "B");
  expect_error (Labeled.remove_edge_label graph "A" "B" "missing");
  let dag = Labeled.create () in
  Labeled.add_edge dag "A" "B" "edge" |> get;
  Alcotest.check strings "labeled successors" [ "B" ]
    (get (Labeled.successors dag "A"));
  Alcotest.check strings "labeled predecessors" [ "A" ]
    (get (Labeled.predecessors dag "B"));
  Alcotest.check strings "labeled topological" [ "A"; "B" ]
    (get (Labeled.topological_sort dag));
  Alcotest.check (Alcotest.list strings) "labeled groups" [ [ "A" ]; [ "B" ] ]
    (get (Labeled.independent_groups dag));
  let strict = Labeled.create () in
  expect_error (Labeled.add_edge strict "A" "A" "loop")

let test_labeled_delegation () =
  let graph = Labeled.create () in
  List.iter
    (fun (left, right, label, weight) ->
      Labeled.add_edge ~weight graph left right label |> get)
    [
      ("A", "B", "ab", 2.);
      ("A", "C", "ac", 3.);
      ("B", "D", "bd", 1.);
      ("C", "D", "cd", 1.);
    ];
  Alcotest.(check int) "delegated size" 4 (Labeled.size graph);
  Alcotest.(check int) "structural edges" 4 (List.length (Labeled.edges graph));
  Alcotest.(check (float 0.)) "delegated weight" 2.
    (get (Labeled.edge_weight graph "A" "B"));
  Labeled.set_graph_property graph "kind" (String "dag");
  Labeled.set_node_property graph "A" "root" (Bool true) |> get;
  Labeled.set_edge_property graph "A" "B" "weight" (Number 4.) |> get;
  Alcotest.check Alcotest.bool "delegated properties" true
    (property_find_opt "kind" (Labeled.graph_properties graph)
       = Some (String "dag")
    && property_find_opt "root" (get (Labeled.node_properties graph "A"))
       = Some (Bool true)
    && property_find_opt "weight"
         (get (Labeled.edge_properties graph "A" "B"))
       = Some (Number 4.));
  Alcotest.check Alcotest.bool "labeled weights synchronized" true
    (Labeled.edges_labeled graph
    |> List.filter (fun (left, right, _, _) -> left = "A" && right = "B")
    |> List.for_all (fun (_, _, _, weight) -> weight = 4.));
  Labeled.remove_edge_property graph "A" "B" "weight" |> get;
  Labeled.remove_node_property graph "A" "root" |> get;
  Labeled.remove_graph_property graph "kind";
  Alcotest.(check (float 0.)) "weight reset through delegate" 1.
    (get (Labeled.edge_weight graph "A" "B"));
  Alcotest.check strings "delegated neighbors" [ "B"; "C" ]
    (get (Labeled.neighbors graph "A"));
  Alcotest.check Alcotest.bool "weighted neighbors" true
    (get (Labeled.neighbors_weighted graph "A") = [ ("B", 1.); ("C", 3.) ]);
  Alcotest.(check int) "out degree" 2 (get (Labeled.out_degree graph "A"));
  Alcotest.(check int) "in degree" 2 (get (Labeled.in_degree graph "D"));
  Alcotest.check strings "delegated bfs" [ "A"; "B"; "C"; "D" ]
    (get (Labeled.bfs graph "A"));
  Alcotest.check strings "delegated dfs" [ "A"; "B"; "D"; "C" ]
    (get (Labeled.dfs graph "A"));
  Alcotest.check strings "delegated topo" [ "A"; "B"; "C"; "D" ]
    (get (Labeled.topological_sort graph));
  Alcotest.(check bool) "delegated cycle" false (Labeled.has_cycle graph);
  Alcotest.check strings "delegated closure" [ "B"; "C"; "D" ]
    (get (Labeled.transitive_closure graph "A"));
  Alcotest.check strings "delegated dependents" [ "A"; "B"; "C" ]
    (get (Labeled.transitive_dependents graph "D"));
  Alcotest.check strings "delegated affected" [ "A"; "B"; "C"; "D" ]
    (Labeled.affected_nodes graph [ "D" ]);
  Alcotest.check (Alcotest.list strings) "delegated components"
    [ [ "A" ]; [ "B" ]; [ "C" ]; [ "D" ] ]
    (Labeled.strongly_connected_components graph);
  expect_error (Labeled.node_properties graph "missing");
  expect_error (Labeled.edge_properties graph "A" "missing");
  expect_error (Labeled.neighbors graph "missing");
  expect_error (Labeled.bfs graph "missing")

let test_large_iterative_traversal () =
  let module Int_graph = Coding_adventures_directed_graph.Make (Int) in
  let graph = Int_graph.create () in
  for node = 0 to 999 do
    Int_graph.add_node graph node
  done;
  for node = 0 to 998 do
    Int_graph.add_edge graph node (node + 1) |> get
  done;
  Alcotest.check Alcotest.int "all reached" 1000
    (List.length (get (Int_graph.bfs graph 0)))

let () =
  Alcotest.run "coding-adventures-directed-graph"
    [
      ( "directed graph",
        [
          Alcotest.test_case "structure and properties" `Quick
            test_structure_and_properties;
          Alcotest.test_case "dag algorithms" `Quick test_dag_algorithms;
          Alcotest.test_case "depth-first branch order" `Quick
            test_depth_first_branch_order;
          Alcotest.test_case "cycles and components" `Quick
            test_cycles_and_components;
          Alcotest.test_case "labeled edges" `Quick test_labeled_edges;
          Alcotest.test_case "labeled delegation" `Quick
            test_labeled_delegation;
          Alcotest.test_case "large iterative traversal" `Quick
            test_large_iterative_traversal;
        ] );
    ]
