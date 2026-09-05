module Make (Node : Map.OrderedType) = struct
  module Node_map = Map.Make (Node)
  module Node_set = Set.Make (Node)
  module String_set = Set.Make (String)
  module Property_base = Coding_adventures_graph.Make (Node)

  module Edge = struct
    type t = Node.t * Node.t

    let compare (left_a, left_b) (right_a, right_b) =
      let first = Node.compare left_a right_a in
      if first <> 0 then first else Node.compare left_b right_b
  end

  module Edge_map = Map.Make (Edge)

  type node = Node.t

  type property_value = Property_base.property_value =
    | String of string
    | Number of float
    | Bool of bool
    | Null

  type property_bag = Property_base.property_bag

  type error =
    | Node_not_found of node
    | Edge_not_found of node * node
    | Self_loop of node
    | Cycle
    | Invalid_weight of float
    | Label_not_found of node * node * string

  type weighted_edge = node * node * float

  type t = {
    allow_self_loops : bool;
    mutable node_set : Node_set.t;
    mutable forward : float Node_map.t Node_map.t;
    mutable reverse : float Node_map.t Node_map.t;
    mutable node_data : property_bag Node_map.t;
    mutable edge_data : property_bag Edge_map.t;
    mutable graph_data : property_bag;
  }

  let properties = Property_base.properties
  let property_bindings = Property_base.property_bindings
  let property_find_opt = Property_base.property_find_opt

  let create ?(allow_self_loops = false) () =
    {
      allow_self_loops;
      node_set = Node_set.empty;
      forward = Node_map.empty;
      reverse = Node_map.empty;
      node_data = Node_map.empty;
      edge_data = Edge_map.empty;
      graph_data = properties [];
    }

  let has_node graph node = Node_set.mem node graph.node_set
  let nodes graph = Node_set.elements graph.node_set
  let size graph = Node_set.cardinal graph.node_set

  let merge_bags previous updates =
    properties (property_bindings previous @ property_bindings updates)

  let add_node ?(properties = properties []) graph node =
    if not (has_node graph node) then (
      graph.node_set <- Node_set.add node graph.node_set;
      graph.forward <- Node_map.add node Node_map.empty graph.forward;
      graph.reverse <- Node_map.add node Node_map.empty graph.reverse);
    let previous =
      Option.value
        ~default:(Property_base.properties [])
        (Node_map.find_opt node graph.node_data)
    in
    graph.node_data <-
      Node_map.add node (merge_bags previous properties) graph.node_data

  let require_node graph node =
    if has_node graph node then Ok () else Error (Node_not_found node)

  let valid_weight weight = Float.is_finite weight && weight >= 0.

  let set_neighbor adjacency left right weight =
    let neighbors = Node_map.find left adjacency in
    Node_map.add left (Node_map.add right weight neighbors) adjacency

  let remove_neighbor adjacency left right =
    let neighbors = Node_map.find left adjacency in
    Node_map.add left (Node_map.remove right neighbors) adjacency

  let add_edge ?(weight = 1.) ?(properties = properties []) graph left right =
    (* Self-loop and weight checks happen before implicit endpoint creation, so
       invalid requests leave both structure and properties unchanged. *)
    if Node.compare left right = 0 && not graph.allow_self_loops then
      Error (Self_loop left)
    else if not (valid_weight weight) then Error (Invalid_weight weight)
    else (
      add_node graph left;
      add_node graph right;
      graph.forward <- set_neighbor graph.forward left right weight;
      graph.reverse <- set_neighbor graph.reverse right left weight;
      let previous =
        Option.value
          ~default:(Property_base.properties [])
          (Edge_map.find_opt (left, right) graph.edge_data)
      in
      let merged =
        merge_bags previous properties |> fun bag ->
        merge_bags bag (Property_base.properties [ ("weight", Number weight) ])
      in
      graph.edge_data <- Edge_map.add (left, right) merged graph.edge_data;
      Ok ())

  let has_edge graph left right =
    match Node_map.find_opt left graph.forward with
    | None -> false
    | Some successors -> Node_map.mem right successors

  let require_edge graph left right =
    if has_edge graph left right then Ok ()
    else Error (Edge_not_found (left, right))

  let remove_edge graph left right =
    match require_edge graph left right with
    | Error error -> Error error
    | Ok () ->
        graph.forward <- remove_neighbor graph.forward left right;
        graph.reverse <- remove_neighbor graph.reverse right left;
        graph.edge_data <- Edge_map.remove (left, right) graph.edge_data;
        Ok ()

  let remove_node graph node =
    match require_node graph node with
    | Error error -> Error error
    | Ok () ->
        Node_map.iter
          (fun successor _ ->
            graph.reverse <- remove_neighbor graph.reverse successor node)
          (Node_map.find node graph.forward);
        Node_map.iter
          (fun predecessor _ ->
            graph.forward <- remove_neighbor graph.forward predecessor node)
          (Node_map.find node graph.reverse);
        graph.forward <- Node_map.remove node graph.forward;
        graph.reverse <- Node_map.remove node graph.reverse;
        graph.node_set <- Node_set.remove node graph.node_set;
        graph.node_data <- Node_map.remove node graph.node_data;
        graph.edge_data <-
          Edge_map.filter
            (fun (left, right) _ ->
              Node.compare left node <> 0 && Node.compare right node <> 0)
            graph.edge_data;
        Ok ()

  let edges graph =
    Edge_map.bindings graph.edge_data
    |> List.map (fun ((left, right), _) ->
           (left, right, Node_map.find right (Node_map.find left graph.forward)))

  let edge_weight graph left right =
    match require_edge graph left right with
    | Error error -> Error error
    | Ok () -> Ok (Node_map.find right (Node_map.find left graph.forward))

  let graph_properties graph = graph.graph_data

  let set_graph_property graph key value =
    graph.graph_data <-
      merge_bags graph.graph_data (properties [ (key, value) ])

  let remove_property bag key =
    property_bindings bag
    |> List.filter (fun (candidate, _) -> candidate <> key)
    |> properties

  let remove_graph_property graph key =
    graph.graph_data <- remove_property graph.graph_data key

  let node_properties graph node =
    match require_node graph node with
    | Error error -> Error error
    | Ok () -> Ok (Node_map.find node graph.node_data)

  let set_node_property graph node key value =
    match node_properties graph node with
    | Error error -> Error error
    | Ok bag ->
        graph.node_data <-
          Node_map.add node
            (merge_bags bag (properties [ (key, value) ]))
            graph.node_data;
        Ok ()

  let remove_node_property graph node key =
    match node_properties graph node with
    | Error error -> Error error
    | Ok bag ->
        graph.node_data <-
          Node_map.add node (remove_property bag key) graph.node_data;
        Ok ()

  let edge_properties graph left right =
    match require_edge graph left right with
    | Error error -> Error error
    | Ok () -> Ok (Edge_map.find (left, right) graph.edge_data)

  let set_edge_property graph left right key value =
    match edge_properties graph left right with
    | Error error -> Error error
    | Ok bag ->
        if key = "weight" then
          match value with
          | Number weight when valid_weight weight ->
              graph.forward <- set_neighbor graph.forward left right weight;
              graph.reverse <- set_neighbor graph.reverse right left weight;
              graph.edge_data <-
                Edge_map.add (left, right)
                  (merge_bags bag (properties [ (key, value) ]))
                  graph.edge_data;
              Ok ()
          | Number weight -> Error (Invalid_weight weight)
          | _ -> Error (Invalid_weight Float.nan)
        else (
          graph.edge_data <-
            Edge_map.add (left, right)
              (merge_bags bag (properties [ (key, value) ]))
              graph.edge_data;
          Ok ())

  let remove_edge_property graph left right key =
    match edge_properties graph left right with
    | Error error -> Error error
    | Ok bag ->
        let updated =
          if key = "weight" then (
            graph.forward <- set_neighbor graph.forward left right 1.;
            graph.reverse <- set_neighbor graph.reverse right left 1.;
            merge_bags bag (properties [ ("weight", Number 1.) ]))
          else remove_property bag key
        in
        graph.edge_data <- Edge_map.add (left, right) updated graph.edge_data;
        Ok ()

  let adjacency_bindings adjacency node =
    Node_map.bindings (Node_map.find node adjacency)

  let successors graph node =
    match require_node graph node with
    | Error error -> Error error
    | Ok () -> Ok (List.map fst (adjacency_bindings graph.forward node))

  let predecessors graph node =
    match require_node graph node with
    | Error error -> Error error
    | Ok () -> Ok (List.map fst (adjacency_bindings graph.reverse node))

  let neighbors = successors

  let neighbors_weighted graph node =
    match require_node graph node with
    | Error error -> Error error
    | Ok () -> Ok (adjacency_bindings graph.forward node)

  let out_degree graph node =
    match successors graph node with
    | Error error -> Error error
    | Ok entries -> Ok (List.length entries)

  let in_degree graph node =
    match predecessors graph node with
    | Error error -> Error error
    | Ok entries -> Ok (List.length entries)

  let traverse adjacency graph start depth_first =
    match require_node graph start with
    | Error error -> Error error
    | Ok () ->
        let frontier = Stack.create () in
        Stack.push start frontier;
        let visited = ref Node_set.empty in
        let result = ref [] in
        while not (Stack.is_empty frontier) do
          let node = Stack.pop frontier in
          if not (Node_set.mem node !visited) then (
            visited := Node_set.add node !visited;
            result := node :: !result;
            let next = adjacency_bindings adjacency node |> List.map fst in
            let ordered = if depth_first then List.rev next else next in
            List.iter
              (fun neighbor ->
                if not (Node_set.mem neighbor !visited) then
                  Stack.push neighbor frontier)
              ordered)
        done;
        Ok (List.rev !result)

  let bfs graph start =
    match require_node graph start with
    | Error error -> Error error
    | Ok () ->
        let queue = Queue.create () in
        Queue.add start queue;
        let visited = ref (Node_set.singleton start) in
        let result = ref [] in
        while not (Queue.is_empty queue) do
          let node = Queue.take queue in
          result := node :: !result;
          List.iter
            (fun (neighbor, _) ->
              if not (Node_set.mem neighbor !visited) then (
                visited := Node_set.add neighbor !visited;
                Queue.add neighbor queue))
            (adjacency_bindings graph.forward node)
        done;
        Ok (List.rev !result)

  let dfs graph start = traverse graph.forward graph start true

  let indegrees graph =
    List.fold_left
      (fun map node ->
        Node_map.add node
          (List.length (adjacency_bindings graph.reverse node))
          map)
      Node_map.empty (nodes graph)

  let topological_sort graph =
    (* Kahn's ready set is ordered, including nodes that become ready later. *)
    let counts = ref (indegrees graph) in
    let ready =
      ref
        (Node_map.fold
           (fun node count set ->
             if count = 0 then Node_set.add node set else set)
           !counts Node_set.empty)
    in
    let result = ref [] in
    while not (Node_set.is_empty !ready) do
      let node = Node_set.min_elt !ready in
      ready := Node_set.remove node !ready;
      result := node :: !result;
      List.iter
        (fun (successor, _) ->
          let remaining = Node_map.find successor !counts - 1 in
          counts := Node_map.add successor remaining !counts;
          if remaining = 0 then ready := Node_set.add successor !ready)
        (adjacency_bindings graph.forward node)
    done;
    if List.length !result = size graph then Ok (List.rev !result)
    else Error Cycle

  let has_cycle graph =
    (* An explicit enter/leave stack implements the DT01 three-colour DFS
       without relying on the native call stack.  A GRAY edge is a back edge;
       an edge to BLACK is a completed cross edge and is not a cycle. *)
    let colors = ref Node_map.empty in
    let color node = Option.value ~default:0 (Node_map.find_opt node !colors) in
    let cycle = ref false in
    List.iter
      (fun start ->
        if (not !cycle) && color start = 0 then (
          let stack = Stack.create () in
          Stack.push (start, true) stack;
          while (not !cycle) && not (Stack.is_empty stack) do
            let node, entering = Stack.pop stack in
            if entering then
              match color node with
              | 1 -> cycle := true
              | 2 -> ()
              | _ ->
                  colors := Node_map.add node 1 !colors;
                  Stack.push (node, false) stack;
                  adjacency_bindings graph.forward node
                  |> List.map fst |> List.rev
                  |> List.iter (fun successor ->
                         Stack.push (successor, true) stack)
            else colors := Node_map.add node 2 !colors
          done))
      (nodes graph);
    !cycle

  let reachable adjacency graph origin =
    match require_node graph origin with
    | Error error -> Error error
    | Ok () ->
        let queue = Queue.create () in
        let visited = ref (Node_set.singleton origin) in
        List.iter
          (fun (node, _) ->
            if not (Node_set.mem node !visited) then (
              visited := Node_set.add node !visited;
              Queue.add node queue))
          (adjacency_bindings adjacency origin);
        while not (Queue.is_empty queue) do
          let node = Queue.take queue in
          List.iter
            (fun (next, _) ->
              if not (Node_set.mem next !visited) then (
                visited := Node_set.add next !visited;
                Queue.add next queue))
            (adjacency_bindings adjacency node)
        done;
        Ok (Node_set.elements (Node_set.remove origin !visited))

  let transitive_closure graph node = reachable graph.forward graph node
  let transitive_dependents graph node = reachable graph.reverse graph node

  let independent_groups graph =
    let counts = ref (indegrees graph) in
    let remaining = ref graph.node_set in
    let groups = ref [] in
    while not (Node_set.is_empty !remaining) do
      let group =
        Node_set.filter (fun node -> Node_map.find node !counts = 0) !remaining
      in
      if Node_set.is_empty group then remaining := Node_set.empty
      else (
        groups := Node_set.elements group :: !groups;
        Node_set.iter
          (fun node ->
            remaining := Node_set.remove node !remaining;
            List.iter
              (fun (successor, _) ->
                counts :=
                  Node_map.add successor
                    (Node_map.find successor !counts - 1)
                    !counts)
              (adjacency_bindings graph.forward node))
          group)
    done;
    let flattened = List.fold_left ( + ) 0 (List.map List.length !groups) in
    if flattened = size graph then Ok (List.rev !groups) else Error Cycle

  let affected_nodes graph changed =
    List.fold_left
      (fun affected node ->
        if not (has_node graph node) then affected
        else
          let dependents =
            match transitive_dependents graph node with
            | Ok entries -> entries
            | Error _ -> []
          in
          List.fold_left
            (fun set item -> Node_set.add item set)
            (Node_set.add node affected)
            dependents)
      Node_set.empty changed
    |> Node_set.elements

  let compare_node_lists left right =
    let rec loop left right =
      match (left, right) with
      | [], [] -> 0
      | [], _ -> -1
      | _, [] -> 1
      | left_head :: left_tail, right_head :: right_tail ->
          let order = Node.compare left_head right_head in
          if order <> 0 then order else loop left_tail right_tail
    in
    loop left right

  let finish_order graph =
    let visited = ref Node_set.empty in
    let finished = ref [] in
    List.iter
      (fun start ->
        if not (Node_set.mem start !visited) then (
          let stack = Stack.create () in
          Stack.push (start, false) stack;
          while not (Stack.is_empty stack) do
            let node, exiting = Stack.pop stack in
            if exiting then finished := node :: !finished
            else if not (Node_set.mem node !visited) then (
              visited := Node_set.add node !visited;
              Stack.push (node, true) stack;
              adjacency_bindings graph.forward node
              |> List.rev
              |> List.iter (fun (next, _) ->
                     if not (Node_set.mem next !visited) then
                       Stack.push (next, false) stack))
          done))
      (nodes graph);
    !finished

  let strongly_connected_components graph =
    (* Kosaraju's second pass walks reverse edges in decreasing finish order;
       both component members and the outer result are normalized afterward. *)
    let visited = ref Node_set.empty in
    let components = ref [] in
    List.iter
      (fun start ->
        if not (Node_set.mem start !visited) then (
          let stack = Stack.create () in
          Stack.push start stack;
          visited := Node_set.add start !visited;
          let component = ref Node_set.empty in
          while not (Stack.is_empty stack) do
            let node = Stack.pop stack in
            component := Node_set.add node !component;
            List.iter
              (fun (next, _) ->
                if not (Node_set.mem next !visited) then (
                  visited := Node_set.add next !visited;
                  Stack.push next stack))
              (adjacency_bindings graph.reverse node)
          done;
          components := Node_set.elements !component :: !components))
      (finish_order graph);
    List.sort compare_node_lists !components

  module Labeled = struct
    type labeled = { graph : t; mutable labels : String_set.t Edge_map.t }

    let create ?(allow_self_loops = false) () =
      { graph = create ~allow_self_loops (); labels = Edge_map.empty }

    let add_node ?(properties = properties []) labeled node =
      add_node ~properties labeled.graph node

    let has_node labeled = has_node labeled.graph
    let nodes labeled = nodes labeled.graph
    let has_edge labeled = has_edge labeled.graph

    let add_edge ?(weight = 1.) ?(properties = properties []) labeled left right
        label =
      let previous =
        Option.value ~default:String_set.empty
          (Edge_map.find_opt (left, right) labeled.labels)
      in
      if String_set.mem label previous then Ok ()
      else
        match add_edge ~weight ~properties labeled.graph left right with
        | Error error -> Error error
        | Ok () ->
            labeled.labels <-
              Edge_map.add (left, right)
                (String_set.add label previous)
                labeled.labels;
            Ok ()

    let edge_labels labeled left right =
      Option.value ~default:String_set.empty
        (Edge_map.find_opt (left, right) labeled.labels)
      |> String_set.elements

    let has_edge_with_label labeled left right label =
      match Edge_map.find_opt (left, right) labeled.labels with
      | None -> false
      | Some labels -> String_set.mem label labels

    let remove_edge labeled left right =
      match remove_edge labeled.graph left right with
      | Error error -> Error error
      | Ok () ->
          labeled.labels <- Edge_map.remove (left, right) labeled.labels;
          Ok ()

    let remove_edge_label labeled left right label =
      match Edge_map.find_opt (left, right) labeled.labels with
      | None -> Error (Label_not_found (left, right, label))
      | Some labels when not (String_set.mem label labels) ->
          Error (Label_not_found (left, right, label))
      | Some labels ->
          let remaining = String_set.remove label labels in
          if String_set.is_empty remaining then remove_edge labeled left right
          else (
            labeled.labels <-
              Edge_map.add (left, right) remaining labeled.labels;
            Ok ())

    let remove_node labeled node =
      match remove_node labeled.graph node with
      | Error error -> Error error
      | Ok () ->
          labeled.labels <-
            Edge_map.filter
              (fun (left, right) _ ->
                Node.compare left node <> 0 && Node.compare right node <> 0)
              labeled.labels;
          Ok ()

    let edges_labeled labeled =
      Edge_map.bindings labeled.labels
      |> List.concat_map (fun ((left, right), labels) ->
             let weight =
               match edge_weight labeled.graph left right with
               | Ok value -> value
               | Error _ -> assert false
             in
             String_set.elements labels
             |> List.map (fun label -> (left, right, label, weight)))

    let successors labeled = successors labeled.graph
    let predecessors labeled = predecessors labeled.graph
    let topological_sort labeled = topological_sort labeled.graph
    let independent_groups labeled = independent_groups labeled.graph
  end
end
