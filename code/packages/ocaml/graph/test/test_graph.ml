module String_graph = Coding_adventures_graph.Make (String)
open String_graph

let get = function
  | Ok value -> value
  | Error _ -> Alcotest.fail "unexpected error"

let expect_error = function
  | Error _ -> ()
  | Ok _ -> Alcotest.fail "expected error"

let strings = Alcotest.list Alcotest.string

let build representation =
  let graph = create ~representation () in
  List.iter (add_node graph) [ "A"; "B"; "C"; "D" ];
  ignore (get (add_edge graph "A" "B"));
  ignore (get (add_edge graph "B" "C"));
  ignore (get (add_edge graph "C" "D"));
  graph

let test_representations () =
  List.iter
    (fun representation ->
      let graph = build representation in
      Alcotest.check Alcotest.bool "representation retained" true
        (String_graph.representation graph = representation);
      Alcotest.check strings "nodes sorted" [ "A"; "B"; "C"; "D" ] (nodes graph);
      Alcotest.check Alcotest.int "size" 4 (size graph);
      Alcotest.check Alcotest.bool "symmetric" true (has_edge graph "B" "A");
      Alcotest.check Alcotest.int "degree" 2 (get (degree graph "B"));
      Alcotest.check strings "neighbors" [ "A"; "C" ]
        (get (neighbors graph "B"));
      Alcotest.check strings "bfs" [ "A"; "B"; "C"; "D" ] (get (bfs graph "A"));
      Alcotest.check strings "dfs" [ "A"; "B"; "C"; "D" ] (get (dfs graph "A"));
      Alcotest.check Alcotest.bool "connected" true (is_connected graph);
      Alcotest.check Alcotest.bool "acyclic" false (has_cycle graph))
    [ Adjacency_list; Adjacency_matrix ]

let test_mutation_cleanup () =
  let graph = build Adjacency_list in
  remove_edge graph "B" "C" |> get;
  Alcotest.check Alcotest.bool "removed both ways" false
    (has_edge graph "C" "B");
  expect_error (remove_edge graph "B" "C");
  remove_node graph "D" |> get;
  Alcotest.check Alcotest.bool "node removed" false (has_node graph "D");
  expect_error (remove_node graph "D");
  expect_error (neighbors graph "missing");
  expect_error (degree graph "missing");
  expect_error (bfs graph "missing");
  expect_error (dfs graph "missing");
  Alcotest.check Alcotest.bool "missing edge lookup" false
    (has_edge graph "missing" "A");
  expect_error (edge_weight graph "A" "missing")

let test_properties () =
  let graph = create () in
  set_graph_property graph "kind" (String "demo");
  add_node ~properties:(properties [ ("color", String "red") ]) graph "A";
  add_node ~properties:(properties [ ("active", Bool true) ]) graph "A";
  add_node graph "B";
  ignore
    (get
       (add_edge ~weight:3.
          ~properties:(properties [ ("role", String "road") ])
          graph "A" "B"));
  Alcotest.check Alcotest.bool "graph property" true
    (property_find_opt "kind" (graph_properties graph) = Some (String "demo"));
  Alcotest.check Alcotest.bool "node properties merged" true
    (property_find_opt "color" (get (node_properties graph "A"))
     = Some (String "red")
    && property_find_opt "active" (get (node_properties graph "A"))
       = Some (Bool true));
  Alcotest.check (Alcotest.float 0.) "weight" 3.
    (get (edge_weight graph "B" "A"));
  set_edge_property graph "A" "B" "weight" (Number 2.) |> get;
  Alcotest.check (Alcotest.float 0.) "property updates weight" 2.
    (get (edge_weight graph "A" "B"));
  remove_edge_property graph "B" "A" "weight" |> get;
  Alcotest.check (Alcotest.float 0.) "weight reset" 1.
    (get (edge_weight graph "A" "B"));
  Alcotest.check Alcotest.bool "weight property reset" true
    (property_find_opt "weight" (get (edge_properties graph "A" "B"))
    = Some (Number 1.));
  remove_node_property graph "A" "color" |> get;
  remove_graph_property graph "kind";
  Alcotest.check Alcotest.bool "removed" true
    (property_find_opt "color" (get (node_properties graph "A")) = None
    && property_find_opt "kind" (graph_properties graph) = None);
  expect_error (node_properties graph "missing");
  expect_error (set_node_property graph "missing" "x" Null);
  expect_error (remove_node_property graph "missing" "x");
  expect_error (edge_properties graph "A" "missing");
  expect_error (set_edge_property graph "A" "missing" "x" Null);
  expect_error (remove_edge_property graph "A" "missing" "x");
  expect_error (set_edge_property graph "A" "B" "weight" (Number (-1.)));
  expect_error (set_edge_property graph "A" "B" "weight" (String "heavy"));
  set_edge_property graph "A" "B" "label" (String "road") |> get;
  Alcotest.check Alcotest.bool "ordinary edge property" true
    (property_find_opt "label" (get (edge_properties graph "A" "B"))
    = Some (String "road"))

let test_components_and_cycles () =
  Alcotest.check Alcotest.bool "empty connected" true (is_connected (create ()));
  let graph = create () in
  List.iter (add_node graph) [ "A"; "B"; "C"; "D"; "E"; "F" ];
  ignore (get (add_edge graph "A" "B"));
  ignore (get (add_edge graph "B" "C"));
  ignore (get (add_edge graph "D" "E"));
  Alcotest.check (Alcotest.list strings) "components"
    [ [ "A"; "B"; "C" ]; [ "D"; "E" ]; [ "F" ] ]
    (connected_components graph);
  Alcotest.check Alcotest.bool "disconnected" false (is_connected graph);
  ignore (get (add_edge graph "C" "A"));
  Alcotest.check Alcotest.bool "triangle" true (has_cycle graph);
  let self_loop = create () in
  add_node self_loop "A";
  add_edge self_loop "A" "A" |> get;
  Alcotest.check Alcotest.bool "self loop" true (has_cycle self_loop)

let test_weighted_algorithms () =
  let graph = create () in
  List.iter (add_node graph) [ "A"; "B"; "C"; "D" ];
  List.iter
    (fun (left, right, weight) ->
      ignore (get (add_edge ~weight graph left right)))
    [ ("A", "B", 1.); ("A", "C", 4.); ("B", "C", 2.); ("C", "D", 1.) ];
  Alcotest.check strings "dijkstra" [ "A"; "B"; "C"; "D" ]
    (get (shortest_path graph "A" "D"));
  Alcotest.check Alcotest.bool "no path" true
    (let other = create () in
     List.iter (add_node other) [ "A"; "B" ];
     get (shortest_path other "A" "B") = []);
  Alcotest.check Alcotest.bool "mst" true
    (get (minimum_spanning_tree graph)
    = [ ("A", "B", 1.); ("C", "D", 1.); ("B", "C", 2.) ]);
  expect_error (add_edge ~weight:(-1.) graph "A" "B");
  expect_error (add_edge ~weight:nan graph "A" "B");
  expect_error (shortest_path graph "missing" "A");
  expect_error (shortest_path graph "A" "missing");
  Alcotest.check Alcotest.bool "single node mst" true
    (let singleton = create () in
     add_node singleton "A";
     get (minimum_spanning_tree singleton) = []);
  let disconnected = create () in
  List.iter (add_node disconnected) [ "A"; "B" ];
  expect_error (minimum_spanning_tree disconnected)

let test_large_iterative_traversal () =
  let module Int_graph = Coding_adventures_graph.Make (Int) in
  let graph = Int_graph.create () in
  for node = 0 to 999 do
    Int_graph.add_node graph node
  done;
  for node = 0 to 998 do
    ignore (get (Int_graph.add_edge graph node (node + 1)))
  done;
  Alcotest.check Alcotest.int "all reached" 1000
    (List.length (get (Int_graph.bfs graph 0)))

let test_matrix_rebuilds () =
  let graph = create ~representation:Adjacency_matrix () in
  List.iter (add_node graph) [ "D"; "B"; "A"; "C" ];
  set_graph_property graph "kind" (String "dense");
  set_node_property graph "C" "color" (String "green") |> get;
  add_edge ~weight:0. graph "A" "B" |> get;
  add_edge ~weight:2. graph "B" "C" |> get;
  add_edge ~weight:3.
    ~properties:(properties [ ("name", String "survivor") ])
    graph "C" "D"
  |> get;
  Alcotest.check Alcotest.bool "zero weight is an edge" true
    (has_edge graph "A" "B" && get (edge_weight graph "A" "B") = 0.);
  remove_node graph "B" |> get;
  Alcotest.check strings "sorted after middle removal" [ "A"; "C"; "D" ]
    (nodes graph);
  Alcotest.check Alcotest.bool "surviving edge and properties" true
    (get (edge_weight graph "C" "D") = 3.
    && property_find_opt "name" (get (edge_properties graph "C" "D"))
       = Some (String "survivor")
    && property_find_opt "color" (get (node_properties graph "C"))
       = Some (String "green")
    && property_find_opt "kind" (graph_properties graph) = Some (String "dense")
    );
  add_node graph "B";
  add_edge graph "A" "B" |> get;
  Alcotest.check strings "add after rebuild" [ "A"; "B" ] (get (bfs graph "A"));
  List.iter (fun node -> remove_node graph node |> get) [ "A"; "B"; "C"; "D" ];
  add_node graph "Z";
  add_edge graph "Z" "Z" |> get;
  Alcotest.check Alcotest.bool "self edge after empty rebuild" true
    (has_edge graph "Z" "Z");
  remove_edge graph "Z" "Z" |> get;
  Alcotest.check Alcotest.bool "self edge removed" false
    (has_edge graph "Z" "Z")

let () =
  Alcotest.run "coding-adventures-graph"
    [
      ( "graph",
        [
          Alcotest.test_case "representations" `Quick test_representations;
          Alcotest.test_case "mutation cleanup" `Quick test_mutation_cleanup;
          Alcotest.test_case "properties" `Quick test_properties;
          Alcotest.test_case "components and cycles" `Quick
            test_components_and_cycles;
          Alcotest.test_case "weighted algorithms" `Quick
            test_weighted_algorithms;
          Alcotest.test_case "large iterative traversal" `Quick
            test_large_iterative_traversal;
          Alcotest.test_case "matrix rebuilds" `Quick test_matrix_rebuilds;
        ] );
    ]
