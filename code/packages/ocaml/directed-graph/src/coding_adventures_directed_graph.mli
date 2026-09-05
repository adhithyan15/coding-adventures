(* Deterministic generic directed graphs with explicit reverse adjacency. *)
module Make (Node : Map.OrderedType) : sig
  type node = Node.t

  type property_value =
    | String of string
    | Number of float
    | Bool of bool
    | Null

  type property_bag

  type error =
    | Node_not_found of node
    | Edge_not_found of node * node
    | Self_loop of node
    | Cycle
    | Invalid_weight of float
    | Label_not_found of node * node * string

  type weighted_edge = node * node * float
  type t

  val properties : (string * property_value) list -> property_bag
  val property_bindings : property_bag -> (string * property_value) list
  val property_find_opt : string -> property_bag -> property_value option
  val create : ?allow_self_loops:bool -> unit -> t
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
  val successors : t -> node -> (node list, error) result
  val predecessors : t -> node -> (node list, error) result
  val neighbors : t -> node -> (node list, error) result
  val neighbors_weighted : t -> node -> ((node * float) list, error) result
  val out_degree : t -> node -> (int, error) result
  val in_degree : t -> node -> (int, error) result
  val bfs : t -> node -> (node list, error) result
  val dfs : t -> node -> (node list, error) result
  val topological_sort : t -> (node list, error) result
  val has_cycle : t -> bool
  val transitive_closure : t -> node -> (node list, error) result
  val transitive_dependents : t -> node -> (node list, error) result
  val independent_groups : t -> (node list list, error) result
  val affected_nodes : t -> node list -> node list
  val strongly_connected_components : t -> node list list

  module Labeled : sig
    type labeled

    val create : ?allow_self_loops:bool -> unit -> labeled
    val add_node : ?properties:property_bag -> labeled -> node -> unit
    val remove_node : labeled -> node -> (unit, error) result
    val has_node : labeled -> node -> bool
    val nodes : labeled -> node list
    val size : labeled -> int

    val add_edge :
      ?weight:float ->
      ?properties:property_bag ->
      labeled ->
      node ->
      node ->
      string ->
      (unit, error) result

    val remove_edge : labeled -> node -> node -> (unit, error) result

    val remove_edge_label :
      labeled -> node -> node -> string -> (unit, error) result

    val has_edge : labeled -> node -> node -> bool
    val edges : labeled -> weighted_edge list
    val edge_weight : labeled -> node -> node -> (float, error) result
    val has_edge_with_label : labeled -> node -> node -> string -> bool
    val edge_labels : labeled -> node -> node -> string list
    val edges_labeled : labeled -> (node * node * string * float) list
    val graph_properties : labeled -> property_bag
    val set_graph_property : labeled -> string -> property_value -> unit
    val remove_graph_property : labeled -> string -> unit
    val node_properties : labeled -> node -> (property_bag, error) result

    val set_node_property :
      labeled -> node -> string -> property_value -> (unit, error) result

    val remove_node_property :
      labeled -> node -> string -> (unit, error) result

    val edge_properties :
      labeled -> node -> node -> (property_bag, error) result

    val set_edge_property :
      labeled -> node -> node -> string -> property_value -> (unit, error) result

    val remove_edge_property :
      labeled -> node -> node -> string -> (unit, error) result

    val successors : labeled -> node -> (node list, error) result
    val predecessors : labeled -> node -> (node list, error) result
    val neighbors : labeled -> node -> (node list, error) result
    val neighbors_weighted : labeled -> node -> ((node * float) list, error) result
    val out_degree : labeled -> node -> (int, error) result
    val in_degree : labeled -> node -> (int, error) result
    val bfs : labeled -> node -> (node list, error) result
    val dfs : labeled -> node -> (node list, error) result
    val topological_sort : labeled -> (node list, error) result
    val has_cycle : labeled -> bool
    val transitive_closure : labeled -> node -> (node list, error) result
    val transitive_dependents : labeled -> node -> (node list, error) result
    val independent_groups : labeled -> (node list list, error) result
    val affected_nodes : labeled -> node list -> node list
    val strongly_connected_components : labeled -> node list list
  end
end
