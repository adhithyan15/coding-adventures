(** Deterministic mutable directed and labeled graphs.

    Forward and reverse indexes remain synchronized. [Node.compare] controls all
    observable ordering, and weights must be finite and non-negative. *)

module Make (Node : Map.OrderedType) : sig
  (** A directed graph over [Node.t]. *)

  type node = Node.t
  (** Node identity. *)

  (** Generic string-keyed property values. Reserved edge key ["weight"] must be
      a valid {!Number}. *)
  type property_value =
    | String of string
    | Number of float
    | Bool of bool
    | Null

  type property_bag
  (** An immutable ordered property snapshot. *)

  (** Typed query, mutation, cycle, self-loop, weight, and label failures. *)
  type error =
    | Node_not_found of node
    | Edge_not_found of node * node
    | Self_loop of node
    | Cycle
    | Invalid_weight of float
    | Label_not_found of node * node * string

  type weighted_edge = node * node * float
  (** [(source, target, weight)]. *)

  type t
  (** A mutable directed graph. *)

  val properties : (string * property_value) list -> property_bag
  (** Builds a bag with later duplicate keys winning. *)

  val property_bindings : property_bag -> (string * property_value) list
  (** Returns bindings sorted by [String.compare]. *)

  val property_find_opt : string -> property_bag -> property_value option
  (** Finds a value without mutating the bag. *)

  val create : ?allow_self_loops:bool -> unit -> t
  (** Creates an empty graph; self-loops are disabled by default. *)

  val add_node : ?properties:property_bag -> t -> node -> unit
  (** Adds a node or merges new properties into an existing node. *)

  val remove_node : t -> node -> (unit, error) result
  (** Removes a node plus every incoming and outgoing edge. *)

  val has_node : t -> node -> bool
  (** Tests node membership. *)

  val nodes : t -> node list
  (** Returns nodes in sorted order. *)

  val size : t -> int
  (** Returns the node count. *)

  val add_edge :
    ?weight:float ->
    ?properties:property_bag ->
    t ->
    node ->
    node ->
    (unit, error) result
  (** Validates self-loop policy and weight before mutation, creates missing
      endpoints, then adds or updates [source -> target]. Properties merge and
      reserved ["weight"] follows the structural weight. *)

  val remove_edge : t -> node -> node -> (unit, error) result
  (** Removes one oriented edge and its properties. *)

  val has_edge : t -> node -> node -> bool
  (** Tests an oriented edge; missing endpoints yield [false]. *)

  val edges : t -> weighted_edge list
  (** Returns edges in deterministic source/target order. *)

  val edge_weight : t -> node -> node -> (float, error) result
  (** Returns one structural edge weight. *)

  val graph_properties : t -> property_bag
  (** Returns an immutable graph-property snapshot. *)

  val set_graph_property : t -> string -> property_value -> unit
  (** Replaces one graph property. *)

  val remove_graph_property : t -> string -> unit
  (** Removes a graph property; an absent key is a no-op. *)

  val node_properties : t -> node -> (property_bag, error) result
  (** Returns an immutable node-property snapshot. *)

  val set_node_property :
    t -> node -> string -> property_value -> (unit, error) result
  (** Replaces one property on an existing node. *)

  val remove_node_property : t -> node -> string -> (unit, error) result
  (** Removes a node property; an absent key is a successful no-op. *)

  val edge_properties : t -> node -> node -> (property_bag, error) result
  (** Returns an immutable edge-property snapshot. *)

  val set_edge_property :
    t -> node -> node -> string -> property_value -> (unit, error) result
  (** Replaces one edge property. Reserved ["weight"] atomically updates the
      structural weight and accepts only a valid {!Number}. *)

  val remove_edge_property : t -> node -> node -> string -> (unit, error) result
  (** Removes a property. Removing ["weight"] resets it to [1.0]. *)

  val successors : t -> node -> (node list, error) result
  (** Returns sorted outgoing neighbors. *)

  val predecessors : t -> node -> (node list, error) result
  (** Returns sorted incoming neighbors. *)

  val neighbors : t -> node -> (node list, error) result
  (** Alias of {!successors}; predecessors are not included. *)

  val neighbors_weighted : t -> node -> ((node * float) list, error) result
  (** Returns sorted outgoing neighbors paired with weights. *)

  val out_degree : t -> node -> (int, error) result
  (** Returns outgoing-edge count. *)

  val in_degree : t -> node -> (int, error) result
  (** Returns incoming-edge count. *)

  val bfs : t -> node -> (node list, error) result
  (** Breadth-first traversal over outgoing edges with sorted tie breaking. *)

  val dfs : t -> node -> (node list, error) result
  (** Iterative depth-first traversal over outgoing edges in sorted branch
      order. *)

  val topological_sort : t -> (node list, error) result
  (** Kahn ordering with the smallest ready node first, or {!Cycle}. *)

  val has_cycle : t -> bool
  (** Tests directed cycles, including allowed self-loops. *)

  val transitive_closure : t -> node -> (node list, error) result
  (** Returns sorted reachable successors, excluding the origin. *)

  val transitive_dependents : t -> node -> (node list, error) result
  (** Returns sorted reverse-reachable nodes, excluding the origin. *)

  val independent_groups : t -> (node list list, error) result
  (** Returns deterministic parallel topological layers, or {!Cycle}. *)

  val affected_nodes : t -> node list -> node list
  (** Returns known changed nodes and all reverse dependents, sorted; unknown
      inputs are ignored. *)

  val strongly_connected_components : t -> node list list
  (** Returns sorted members and lexicographically sorted components. *)

  module Labeled : sig
    (** Multiple string labels decorating one shared structural edge.

        Labels have no independent weight or property bag. Empty strings are
        ordinary labels. *)

    type labeled
    (** A mutable labeled directed graph. *)

    val create : ?allow_self_loops:bool -> unit -> labeled
    (** Creates an empty labeled graph with base self-loop semantics. *)

    val add_node : ?properties:property_bag -> labeled -> node -> unit
    (** Adds or merges a node using base semantics. *)

    val remove_node : labeled -> node -> (unit, error) result
    (** Removes a node, incident edges, and their labels. *)

    val has_node : labeled -> node -> bool
    (** Tests node membership. *)

    val nodes : labeled -> node list
    (** Returns sorted nodes. *)

    val size : labeled -> int
    (** Returns the node count. *)

    val add_edge :
      ?weight:float ->
      ?properties:property_bag ->
      labeled ->
      node ->
      node ->
      string ->
      (unit, error) result
    (** Adds a label and its shared structural edge. Re-adding an existing label
        is a complete no-op, including supplied weight/properties. A new label
        on an existing pair updates the shared edge for every label. *)

    val remove_edge : labeled -> node -> node -> (unit, error) result
    (** Removes the structural edge and all labels. *)

    val remove_edge_label :
      labeled -> node -> node -> string -> (unit, error) result
    (** Removes one label; removing the last label removes the edge. *)

    val has_edge : labeled -> node -> node -> bool
    (** Tests structural edge membership. *)

    val edges : labeled -> weighted_edge list
    (** Returns shared structural edges in deterministic order. *)

    val edge_weight : labeled -> node -> node -> (float, error) result
    (** Returns the shared structural edge weight. *)

    val has_edge_with_label : labeled -> node -> node -> string -> bool
    (** Tests a label; missing edges or labels yield [false]. *)

    val edge_labels : labeled -> node -> node -> string list
    (** Returns sorted labels; a missing edge yields [[]]. *)

    val edges_labeled : labeled -> (node * node * string * float) list
    (** Returns [(source, target, label, shared_weight)] ordered by edge then
        label. *)

    val graph_properties : labeled -> property_bag
    (** Returns an immutable underlying graph-property snapshot. *)

    val set_graph_property : labeled -> string -> property_value -> unit
    (** Replaces an underlying graph property. *)

    val remove_graph_property : labeled -> string -> unit
    (** Removes an underlying graph property. *)

    val node_properties : labeled -> node -> (property_bag, error) result
    (** Returns an immutable node-property snapshot. *)

    val set_node_property :
      labeled -> node -> string -> property_value -> (unit, error) result
    (** Replaces one node property. *)

    val remove_node_property : labeled -> node -> string -> (unit, error) result
    (** Removes one node property. *)

    val edge_properties :
      labeled -> node -> node -> (property_bag, error) result
    (** Returns the shared immutable edge-property snapshot. *)

    val set_edge_property :
      labeled ->
      node ->
      node ->
      string ->
      property_value ->
      (unit, error) result
    (** Replaces a shared structural edge property. *)

    val remove_edge_property :
      labeled -> node -> node -> string -> (unit, error) result
    (** Removes or resets one shared structural edge property. *)

    val successors : labeled -> node -> (node list, error) result
    (** Returns sorted outgoing neighbors. *)

    val predecessors : labeled -> node -> (node list, error) result
    (** Returns sorted incoming neighbors. *)

    val neighbors : labeled -> node -> (node list, error) result
    (** Alias of labeled {!successors}. *)

    val neighbors_weighted :
      labeled -> node -> ((node * float) list, error) result
    (** Returns sorted outgoing neighbors with shared weights. *)

    val out_degree : labeled -> node -> (int, error) result
    (** Returns structural outgoing-edge count. *)

    val in_degree : labeled -> node -> (int, error) result
    (** Returns structural incoming-edge count. *)

    val bfs : labeled -> node -> (node list, error) result
    (** Runs base breadth-first traversal. *)

    val dfs : labeled -> node -> (node list, error) result
    (** Runs base iterative depth-first traversal. *)

    val topological_sort : labeled -> (node list, error) result
    (** Runs base deterministic topological sorting. *)

    val has_cycle : labeled -> bool
    (** Tests base structural directed cycles. *)

    val transitive_closure : labeled -> node -> (node list, error) result
    (** Returns sorted structural forward closure. *)

    val transitive_dependents : labeled -> node -> (node list, error) result
    (** Returns sorted structural reverse closure. *)

    val independent_groups : labeled -> (node list list, error) result
    (** Returns structural topological layers. *)

    val affected_nodes : labeled -> node list -> node list
    (** Returns changed nodes and structural reverse dependents. *)

    val strongly_connected_components : labeled -> node list list
    (** Returns structural strongly connected components. *)
  end
end
