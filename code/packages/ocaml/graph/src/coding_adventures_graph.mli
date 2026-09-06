(** Deterministic mutable undirected graphs.

    [Node.compare] controls every observable order. Sparse adjacency-list and
    dense adjacency-matrix storage have identical semantics. Self-loops are
    permitted; weights must be finite and non-negative. *)

module Make (Node : Map.OrderedType) : sig
  (** An undirected graph over the ordered node type [Node.t]. *)

  type node = Node.t
  (** Node identity. *)

  (** An internal storage choice with no observable semantic difference. *)
  type representation =
    | Adjacency_list  (** Sparse adjacency maps. *)
    | Adjacency_matrix  (** Dense matrix storage. *)

  (** A generic property value. Edge key ["weight"] is reserved and must hold a
      finite, non-negative {!Number}. *)
  type property_value =
    | String of string
    | Number of float
    | Bool of bool
    | Null

  type property_bag
  (** An immutable ordered snapshot of string-keyed properties. *)

  (** Typed query, mutation, connectivity, and weight failures. *)
  type error =
    | Node_not_found of node
    | Edge_not_found of node * node
    | Not_connected
    | Invalid_weight of float

  type weighted_edge = node * node * float
  (** [(canonical_left, canonical_right, weight)]. *)

  type t
  (** A mutable graph. *)

  val properties : (string * property_value) list -> property_bag
  (** Builds a bag; later duplicate keys win. *)

  val property_bindings : property_bag -> (string * property_value) list
  (** Returns bindings sorted by [String.compare]. *)

  val property_find_opt : string -> property_bag -> property_value option
  (** Finds a value without mutating the bag. *)

  val create : ?representation:representation -> unit -> t
  (** Creates an empty graph, using {!Adjacency_list} by default. *)

  val representation : t -> representation
  (** Returns the storage strategy selected at construction. *)

  val add_node : ?properties:property_bag -> t -> node -> unit
  (** Adds a node, or merges properties into an existing node with new values
      winning. *)

  val remove_node : t -> node -> (unit, error) result
  (** Removes a node, all incident edges, and their properties. *)

  val has_node : t -> node -> bool
  (** Tests membership. *)

  val nodes : t -> node list
  (** Returns all nodes in [Node.compare] order. *)

  val size : t -> int
  (** Returns the node count. *)

  val add_edge :
    ?weight:float ->
    ?properties:property_bag ->
    t ->
    node ->
    node ->
    (unit, error) result
  (** Validates before mutation, creates missing endpoints, and adds or updates
      an undirected edge. Properties merge and reserved ["weight"] is forced to
      the structural weight. *)

  val remove_edge : t -> node -> node -> (unit, error) result
  (** Removes an edge and its properties. *)

  val has_edge : t -> node -> node -> bool
  (** Tests edge membership; missing endpoints yield [false]. *)

  val edges : t -> weighted_edge list
  (** Returns canonicalized edges in deterministic endpoint order. *)

  val edge_weight : t -> node -> node -> (float, error) result
  (** Returns the structural weight. *)

  val graph_properties : t -> property_bag
  (** Returns an immutable graph-property snapshot. *)

  val set_graph_property : t -> string -> property_value -> unit
  (** Replaces one graph property. *)

  val remove_graph_property : t -> string -> unit
  (** Removes one graph property; an absent key is a no-op. *)

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
  (** Replaces an edge property. Reserved ["weight"] atomically updates the
      structural weight and accepts only a valid {!Number}. *)

  val remove_edge_property : t -> node -> node -> string -> (unit, error) result
  (** Removes a property. Removing reserved ["weight"] resets it to [1.0] and
      retains the canonical key. *)

  val neighbors : t -> node -> (node list, error) result
  (** Returns adjacent nodes in sorted order. *)

  val neighbors_weighted : t -> node -> ((node * float) list, error) result
  (** Returns sorted adjacent nodes paired with weights. *)

  val degree : t -> node -> (int, error) result
  (** Returns incident-edge count; a self-loop counts once. *)

  val bfs : t -> node -> (node list, error) result
  (** Breadth-first traversal with smallest-node tie breaking. *)

  val dfs : t -> node -> (node list, error) result
  (** Iterative depth-first traversal with smallest-node branch order. *)

  val is_connected : t -> bool
  (** Tests connectivity; the empty graph is connected. *)

  val connected_components : t -> node list list
  (** Returns sorted members and components ordered by their first member. *)

  val has_cycle : t -> bool
  (** Tests undirected cycles; a self-loop is a cycle. *)

  val shortest_path : t -> node -> node -> (node list, error) result
  (** Runs deterministic Dijkstra search. A path includes both endpoints;
      unreachable destinations yield [Ok []]. *)

  val minimum_spanning_tree : t -> (weighted_edge list, error) result
  (** Runs deterministic Kruskal selection. Empty and singleton graphs yield
      [Ok []]; a disconnected multi-node graph yields {!Not_connected}. *)
end
