module Make (Node : Map.OrderedType) = struct
  (* One semantic graph sits above two genuinely different stores. Algorithms
      only consume sorted neighbor bindings, which makes their result order
      independent of the chosen sparse or dense representation. *)
  module Node_map = Map.Make (Node)
  module Node_set = Set.Make (Node)
  module String_map = Map.Make (String)

  module Edge = struct
    type t = Node.t * Node.t

    let compare (left_a, left_b) (right_a, right_b) =
      let first = Node.compare left_a right_a in
      if first <> 0 then first else Node.compare left_b right_b
  end

  module Edge_map = Map.Make (Edge)

  type node = Node.t
  type representation = Adjacency_list | Adjacency_matrix

  type property_value =
    | String of string
    | Number of float
    | Bool of bool
    | Null

  type property_bag = property_value String_map.t

  type error =
    | Node_not_found of node
    | Edge_not_found of node * node
    | Not_connected
    | Invalid_weight of float

  type weighted_edge = node * node * float

  type matrix_store = {
    mutable order : node array;
    mutable index : int Node_map.t;
    mutable cells : float option array array;
  }

  type storage =
    | List_store of float Node_map.t Node_map.t
    | Matrix_store of matrix_store

  type t = {
    mutable node_set : Node_set.t;
    mutable storage : storage;
    mutable node_data : property_bag Node_map.t;
    mutable edge_data : property_bag Edge_map.t;
    mutable graph_data : property_bag;
  }

  let properties entries =
    List.fold_left
      (fun bag (key, value) -> String_map.add key value bag)
      String_map.empty entries

  let property_bindings = String_map.bindings
  let property_find_opt = String_map.find_opt

  let create ?(representation = Adjacency_list) () =
    {
      node_set = Node_set.empty;
      storage =
        (match representation with
        | Adjacency_list -> List_store Node_map.empty
        | Adjacency_matrix ->
            Matrix_store { order = [||]; index = Node_map.empty; cells = [||] });
      node_data = Node_map.empty;
      edge_data = Edge_map.empty;
      graph_data = String_map.empty;
    }

  let representation graph =
    match graph.storage with
    | List_store _ -> Adjacency_list
    | Matrix_store _ -> Adjacency_matrix

  let canonical_edge left right =
    if Node.compare left right <= 0 then (left, right) else (right, left)

  let has_node graph node = Node_set.mem node graph.node_set
  let nodes graph = Node_set.elements graph.node_set
  let size graph = Node_set.cardinal graph.node_set

  let merge_bags previous updates =
    String_map.fold String_map.add updates previous

  let index_nodes order =
    Array.fold_left
      (fun (next, index) node -> (next + 1, Node_map.add node next index))
      (0, Node_map.empty) order
    |> snd

  let resize_matrix matrix new_order =
    (* Matrix indices follow [Node.compare]. Rebuilding creates the complete
        replacement before publishing it, then copies only surviving cells.
        [float option] distinguishes a missing edge from a legal zero weight. *)
    let new_index = index_nodes new_order in
    let count = Array.length new_order in
    let new_cells = Array.make_matrix count count None in
    Array.iteri
      (fun old_left left ->
        match Node_map.find_opt left new_index with
        | None -> ()
        | Some new_left ->
            Array.iteri
              (fun old_right right ->
                match Node_map.find_opt right new_index with
                | None -> ()
                | Some new_right ->
                    new_cells.(new_left).(new_right) <-
                      matrix.cells.(old_left).(old_right))
              matrix.order)
      matrix.order;
    matrix.order <- new_order;
    matrix.index <- new_index;
    matrix.cells <- new_cells

  let add_storage_node graph node =
    match graph.storage with
    | List_store adjacency ->
        graph.storage <- List_store (Node_map.add node Node_map.empty adjacency)
    | Matrix_store matrix ->
        resize_matrix matrix (Array.of_list (Node_set.elements graph.node_set))

  let add_node ?(properties = String_map.empty) graph node =
    if not (has_node graph node) then (
      graph.node_set <- Node_set.add node graph.node_set;
      add_storage_node graph node);
    let previous =
      Option.value ~default:String_map.empty
        (Node_map.find_opt node graph.node_data)
    in
    graph.node_data <-
      Node_map.add node (merge_bags previous properties) graph.node_data

  let require_node graph node =
    if has_node graph node then Ok () else Error (Node_not_found node)

  let valid_weight weight = Float.is_finite weight && weight >= 0.

  let add_neighbor graph left right weight =
    match graph.storage with
    | List_store adjacency ->
        let current = Node_map.find left adjacency in
        graph.storage <-
          List_store
            (Node_map.add left (Node_map.add right weight current) adjacency)
    | Matrix_store matrix ->
        let left_index = Node_map.find left matrix.index in
        let right_index = Node_map.find right matrix.index in
        matrix.cells.(left_index).(right_index) <- Some weight

  let add_edge ?(weight = 1.) ?(properties = String_map.empty) graph left right
      =
    (* Validation precedes both structural writes. The property entry is
        canonicalized for an undirected edge, while adjacency is written in
        both directions (twice to the same cell for a self-edge). *)
    if not (valid_weight weight) then Error (Invalid_weight weight)
    else (
      add_node graph left;
      add_node graph right;
      add_neighbor graph left right weight;
      add_neighbor graph right left weight;
      let edge = canonical_edge left right in
      let previous =
        Option.value ~default:String_map.empty
          (Edge_map.find_opt edge graph.edge_data)
      in
      let merged =
        merge_bags previous properties
        |> String_map.add "weight" (Number weight)
      in
      graph.edge_data <- Edge_map.add edge merged graph.edge_data;
      Ok ())

  let has_edge graph left right =
    match graph.storage with
    | List_store adjacency -> (
        match Node_map.find_opt left adjacency with
        | None -> false
        | Some neighbors -> Node_map.mem right neighbors)
    | Matrix_store matrix -> (
        match
          ( Node_map.find_opt left matrix.index,
            Node_map.find_opt right matrix.index )
        with
        | Some left_index, Some right_index ->
            Option.is_some matrix.cells.(left_index).(right_index)
        | _ -> false)

  let require_edge graph left right =
    if has_edge graph left right then Ok ()
    else Error (Edge_not_found (left, right))

  let remove_neighbor graph left right =
    match graph.storage with
    | List_store adjacency ->
        let current = Node_map.find left adjacency in
        graph.storage <-
          List_store
            (Node_map.add left (Node_map.remove right current) adjacency)
    | Matrix_store matrix ->
        let left_index = Node_map.find left matrix.index in
        let right_index = Node_map.find right matrix.index in
        matrix.cells.(left_index).(right_index) <- None

  let neighbor_bindings graph node =
    match graph.storage with
    | List_store adjacency -> Node_map.bindings (Node_map.find node adjacency)
    | Matrix_store matrix ->
        let row = matrix.cells.(Node_map.find node matrix.index) in
        let result = ref [] in
        for position = Array.length matrix.order - 1 downto 0 do
          match row.(position) with
          | None -> ()
          | Some value -> result := (matrix.order.(position), value) :: !result
        done;
        !result

  let stored_weight graph left right =
    match graph.storage with
    | List_store adjacency -> Node_map.find right (Node_map.find left adjacency)
    | Matrix_store matrix ->
        let left_index = Node_map.find left matrix.index in
        let right_index = Node_map.find right matrix.index in
        Option.get matrix.cells.(left_index).(right_index)

  let remove_edge graph left right =
    match require_edge graph left right with
    | Error error -> Error error
    | Ok () ->
        remove_neighbor graph left right;
        remove_neighbor graph right left;
        graph.edge_data <-
          Edge_map.remove (canonical_edge left right) graph.edge_data;
        Ok ()

  let remove_node graph node =
    match require_node graph node with
    | Error error -> Error error
    | Ok () ->
        let neighbors = neighbor_bindings graph node in
        List.iter
          (fun (neighbor, _) -> remove_neighbor graph neighbor node)
          neighbors;
        graph.node_set <- Node_set.remove node graph.node_set;
        (match graph.storage with
        | List_store adjacency ->
            graph.storage <- List_store (Node_map.remove node adjacency)
        | Matrix_store matrix ->
            resize_matrix matrix
              (Array.of_list (Node_set.elements graph.node_set)));
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
           (left, right, stored_weight graph left right))

  let edge_weight graph left right =
    match require_edge graph left right with
    | Error error -> Error error
    | Ok () -> Ok (stored_weight graph left right)

  let graph_properties graph = graph.graph_data

  let set_graph_property graph key value =
    graph.graph_data <- String_map.add key value graph.graph_data

  let remove_graph_property graph key =
    graph.graph_data <- String_map.remove key graph.graph_data

  let node_properties graph node =
    match require_node graph node with
    | Error error -> Error error
    | Ok () -> Ok (Node_map.find node graph.node_data)

  let set_node_property graph node key value =
    match node_properties graph node with
    | Error error -> Error error
    | Ok bag ->
        graph.node_data <-
          Node_map.add node (String_map.add key value bag) graph.node_data;
        Ok ()

  let remove_node_property graph node key =
    match node_properties graph node with
    | Error error -> Error error
    | Ok bag ->
        graph.node_data <-
          Node_map.add node (String_map.remove key bag) graph.node_data;
        Ok ()

  let edge_properties graph left right =
    match require_edge graph left right with
    | Error error -> Error error
    | Ok () -> Ok (Edge_map.find (canonical_edge left right) graph.edge_data)

  let set_edge_property graph left right key value =
    match edge_properties graph left right with
    | Error error -> Error error
    | Ok bag ->
        if key = "weight" then
          match value with
          | Number weight when valid_weight weight ->
              add_neighbor graph left right weight;
              add_neighbor graph right left weight;
              graph.edge_data <-
                Edge_map.add
                  (canonical_edge left right)
                  (String_map.add key value bag)
                  graph.edge_data;
              Ok ()
          | Number weight -> Error (Invalid_weight weight)
          | _ -> Error (Invalid_weight Float.nan)
        else (
          graph.edge_data <-
            Edge_map.add
              (canonical_edge left right)
              (String_map.add key value bag)
              graph.edge_data;
          Ok ())

  let remove_edge_property graph left right key =
    match edge_properties graph left right with
    | Error error -> Error error
    | Ok bag ->
        let updated =
          if key = "weight" then (
            add_neighbor graph left right 1.;
            add_neighbor graph right left 1.;
            String_map.add "weight" (Number 1.) bag)
          else String_map.remove key bag
        in
        graph.edge_data <-
          Edge_map.add (canonical_edge left right) updated graph.edge_data;
        Ok ()

  let neighbors_weighted graph node =
    match require_node graph node with
    | Error error -> Error error
    | Ok () -> Ok (neighbor_bindings graph node)

  let neighbors graph node =
    match neighbors_weighted graph node with
    | Error error -> Error error
    | Ok entries -> Ok (List.map fst entries)

  let degree graph node =
    match neighbors graph node with
    | Error error -> Error error
    | Ok entries -> Ok (List.length entries)

  let bfs graph start =
    (* Origins are marked when queued, preventing duplicate work. Sorted
        neighbor enumeration provides a stable breadth-first tie break. *)
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
            (neighbor_bindings graph node)
        done;
        Ok (List.rev !result)

  let dfs graph start =
    (* Reversing sorted neighbors before pushing them makes the smallest node
        the next stack pop, matching adjacency-list and matrix traversal. *)
    match require_node graph start with
    | Error error -> Error error
    | Ok () ->
        let stack = Stack.create () in
        Stack.push start stack;
        let visited = ref Node_set.empty in
        let result = ref [] in
        while not (Stack.is_empty stack) do
          let node = Stack.pop stack in
          if not (Node_set.mem node !visited) then (
            visited := Node_set.add node !visited;
            result := node :: !result;
            neighbor_bindings graph node
            |> List.rev
            |> List.iter (fun (neighbor, _) ->
                   if not (Node_set.mem neighbor !visited) then
                     Stack.push neighbor stack))
        done;
        Ok (List.rev !result)

  let is_connected graph =
    match nodes graph with
    | [] -> true
    | start :: _ -> (
        match bfs graph start with
        | Ok reached -> List.length reached = size graph
        | Error _ -> false)

  let connected_components graph =
    let unseen = ref graph.node_set in
    let components = ref [] in
    while not (Node_set.is_empty !unseen) do
      let start = Node_set.min_elt !unseen in
      match bfs graph start with
      | Error _ -> assert false
      | Ok component ->
          List.iter
            (fun node -> unseen := Node_set.remove node !unseen)
            component;
          components := component :: !components
    done;
    List.rev !components

  let has_cycle graph =
    let visited = ref Node_set.empty in
    let found = ref false in
    List.iter
      (fun start ->
        if (not !found) && not (Node_set.mem start !visited) then (
          let stack = Stack.create () in
          Stack.push (start, None) stack;
          while (not !found) && not (Stack.is_empty stack) do
            let node, parent = Stack.pop stack in
            if not (Node_set.mem node !visited) then (
              visited := Node_set.add node !visited;
              List.iter
                (fun (neighbor, _) ->
                  if Node.compare neighbor node = 0 then found := true
                  else if not (Node_set.mem neighbor !visited) then
                    Stack.push (neighbor, Some node) stack
                  else
                    match parent with
                    | Some previous when Node.compare neighbor previous = 0 ->
                        ()
                    | _ -> found := true)
                (neighbor_bindings graph node))
          done))
      (nodes graph);
    !found

  let shortest_path graph start target =
    (* This teaching implementation uses the O(V^2) form of Dijkstra: select
        the smallest unvisited distance, break ties with [Node.compare], then
        relax non-negative outgoing weights. *)
    match require_node graph start with
    | Error error -> Error error
    | Ok () -> (
        match require_node graph target with
        | Error error -> Error error
        | Ok () ->
            let infinity = Float.infinity in
            let distances =
              ref
                (List.fold_left
                   (fun map node -> Node_map.add node infinity map)
                   Node_map.empty (nodes graph)
                |> Node_map.add start 0.)
            in
            let parents = ref Node_map.empty in
            let unvisited = ref graph.node_set in
            while not (Node_set.is_empty !unvisited) do
              let selected =
                Node_set.fold
                  (fun node best ->
                    match best with
                    | None -> Some node
                    | Some current ->
                        let node_distance = Node_map.find node !distances in
                        let current_distance =
                          Node_map.find current !distances
                        in
                        if
                          node_distance < current_distance
                          || node_distance = current_distance
                             && Node.compare node current < 0
                        then Some node
                        else best)
                  !unvisited None
              in
              match selected with
              | None -> ()
              | Some node ->
                  unvisited := Node_set.remove node !unvisited;
                  let distance = Node_map.find node !distances in
                  if Float.is_finite distance then
                    List.iter
                      (fun (neighbor, weight) ->
                        if Node_set.mem neighbor !unvisited then
                          let candidate = distance +. weight in
                          let previous = Node_map.find neighbor !distances in
                          if candidate < previous then (
                            distances :=
                              Node_map.add neighbor candidate !distances;
                            parents := Node_map.add neighbor node !parents))
                      (neighbor_bindings graph node)
            done;
            if not (Float.is_finite (Node_map.find target !distances)) then
              Ok []
            else
              let rec build path node =
                if Node.compare node start = 0 then start :: path
                else build (node :: path) (Node_map.find node !parents)
              in
              Ok (build [] target))

  let minimum_spanning_tree graph =
    (* Kruskal orders edges by weight and canonical endpoints. The local
        union-find uses path compression, so equal-cost graphs still produce
        one deterministic minimum spanning tree. *)
    if size graph <= 1 then Ok []
    else if not (is_connected graph) then Error Not_connected
    else
      let parent =
        ref
          (List.fold_left
             (fun map node -> Node_map.add node node map)
             Node_map.empty (nodes graph))
      in
      let rec find node =
        let next = Node_map.find node !parent in
        if Node.compare next node = 0 then node
        else
          let root = find next in
          parent := Node_map.add node root !parent;
          root
      in
      let ordered =
        List.sort
          (fun (left_a, left_b, left_weight) (right_a, right_b, right_weight) ->
            let weight_order = Float.compare left_weight right_weight in
            if weight_order <> 0 then weight_order
            else
              let first = Node.compare left_a right_a in
              if first <> 0 then first else Node.compare left_b right_b)
          (edges graph)
      in
      let result = ref [] in
      List.iter
        (fun ((left, right, _) as edge) ->
          let left_root = find left in
          let right_root = find right in
          if Node.compare left_root right_root <> 0 then (
            parent := Node_map.add right_root left_root !parent;
            result := edge :: !result))
        ordered;
      Ok (List.rev !result)
end
