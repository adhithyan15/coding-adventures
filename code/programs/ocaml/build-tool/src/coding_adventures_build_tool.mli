(** Process-free graph and diff-selection decisions for the OCaml build tool.

    Every value is supplied by the caller. This module never reads a checkout,
    invokes Git, launches work, or inspects host state. *)

type edge = { prerequisite : string; dependent : string }
type graph_input = { packages : string list; edges : edge list }
type graph_result = { edges : edge list; levels : string list list }
type source_mode = Package_prefix | Strict_globs
type unknown_path_policy = All | Error

type package_spec = {
  name : string;
  rel_path : string;
  source_mode : source_mode;
  source_globs : string list;
}

type applies_to = {
  exact_roots : string list;
  descendant_roots : string list;
  excluded_roots : string list;
}

type boundary_input = {
  path : string;
  role : string;
  generated_component : string option;
}

type boundary_rule = {
  id : string;
  input_origin : string;
  applies_to : applies_to;
  inputs : boundary_input list;
  reason : string;
  owner : string;
}

type repository_boundary = {
  schema_version : int;
  language_source_input_registry_sha256 : string;
  boundaries : boundary_rule list;
}
(** Inert boundary data that the trusted adapter has already validated against
    the language-neutral repository-source-input-boundary contract. The core
    independently enforces schema-shape and work budgets before hashing or
    projecting this value. *)

type diff_selection_input = {
  packages : package_spec list;
  edges : edge list;
  forced_packages : string list;
  unknown_path_policy : unknown_path_policy;
  changed_paths : string list;
  boundary_sha256 : string option;
  boundary : repository_boundary option;
}

type diff_selection_result = {
  changed_packages : string list;
  affected_packages : string list;
  prerequisite_packages : string list;
}

type error =
  | Graph_package_limit_exceeded
  | Graph_edge_limit_exceeded
  | Graph_package_invalid
  | Graph_package_duplicate
  | Graph_edge_unknown
  | Graph_edge_self
  | Graph_edge_duplicate
  | Graph_cycle
  | Diff_edge_cycle
  | Diff_package_invalid
  | Diff_package_duplicate
  | Diff_path_invalid
  | Diff_glob_invalid
  | Diff_forced_package_invalid
  | Diff_forced_package_unknown
  | Diff_boundary_digest_mismatch
  | Diff_unknown_path
  | Diff_match_limit_exceeded  (** Stable, closed, payload-free failures. *)

val error_code : error -> string
(** Maps a typed failure to its language-neutral diagnostic code. *)

val repository_boundary_digest : repository_boundary -> (string, error) result
(** Computes the language-neutral canonical digest after enforcing independent
    shape and work-budget guards. *)

val evaluate_graph : graph_input -> (graph_result, error) result
(** Validates and evaluates deterministic prerequisite-first graph levels. *)

val evaluate_diff_selection :
  diff_selection_input -> (diff_selection_result, error) result
(** Validates and evaluates changed, affected, and prerequisite-only sets. *)
