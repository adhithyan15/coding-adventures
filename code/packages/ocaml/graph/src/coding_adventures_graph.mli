(* Deterministic generic undirected graphs with sparse-list and dense-matrix
    storage. Both representations expose the same observable ordering. *)
module Make (Node : Map.OrderedType) : sig
  type node = Node.t
  type representation = Adjacency_list | Adjacency_matrix

  type property_value =
    | String of string
    | Number of float
    | Bool of bool
    | Null

  type property_bag

  type error =
    | Node_not_found of node
    | Edge_not_found of node * node
    | Not_connected
    | Invalid_weight of float

  type weighted_edge = node * node * float
  type t

  val properties : (string * property_value) list -> property_bag
  val property_bindings : property_bag -> (string * property_value) list
  val property_find_opt : string -> property_bag -> property_value option
  val create : ?representation:representation -> unit -> t

  (* Construct an empty graph using the requested internal representation. *)
  val representation : t -> representation
  val add_node : ?properties:property_bag -> t -> node -> unit
  val remove_node : t -> node -> (unit, error) result
  val has_node : t -> node -> bool
  val nodes : t -> node list
  val size : t -> int

  val add_edge :
    ?weight:float ->
    ?properties:property_bag ->
    t ->
    node ->
    node ->
    (unit, error) result

  val remove_edge : t -> node -> node -> (unit, error) result
  val has_edge : t -> node -> node -> bool
  val edges : t -> weighted_edge list
  val edge_weight : t -> node -> node -> (float, error) result
  val graph_properties : t -> property_bag
  val set_graph_property : t -> string -> property_value -> unit
  val remove_graph_property : t -> string -> unit
  val node_properties : t -> node -> (property_bag, error) result

  val set_node_property :
    t -> node -> string -> property_value -> (unit, error) result

  val remove_node_property : t -> node -> string -> (unit, error) result
  val edge_properties : t -> node -> node -> (property_bag, error) result

  val set_edge_property :
    t -> node -> node -> string -> property_value -> (unit, error) result

  val remove_edge_property : t -> node -> node -> string -> (unit, error) result
  val neighbors : t -> node -> (node list, error) result
  val neighbors_weighted : t -> node -> ((node * float) list, error) result
  val degree : t -> node -> (int, error) result
  val bfs : t -> node -> (node list, error) result
  val dfs : t -> node -> (node list, error) result
  val is_connected : t -> bool
  val connected_components : t -> node list list
  val has_cycle : t -> bool
  val shortest_path : t -> node -> node -> (node list, error) result
  val minimum_spanning_tree : t -> (weighted_edge list, error) result
end
