(** Static enforcement for OCaml capability manifests. *)

module Capability : sig
  type t = private { category : string; action : string; target : string }

  val make : category:string -> action:string -> target:string -> t
  val to_string : t -> string
  val compare : t -> t -> int
end

type source_kind = Implementation | Interface

type detection = {
  file : string;
  line : int;
  column : int;
  capability : Capability.t;
  evidence : string;
}

type banned_construct = {
  file : string;
  line : int;
  column : int;
  construct : string;
  evidence : string;
  required_capability : Capability.t option;
  exemptible : bool;
}

type exception_declaration = {
  construct : string;
  language : string;
  justification : string;
}

type manifest = {
  package : string;
  capabilities : Capability.t list;
  exceptions : exception_declaration list;
}

type violation = {
  code : string;
  file : string;
  line : int;
  column : int;
  message : string;
}

type analysis_result = {
  dir : string;
  detected : detection list;
  banned : banned_construct list;
  declared : Capability.t list;
  violations : violation list;
}

val parse_manifest : string -> (manifest, string) result

val analyze_source :
  filename:string ->
  source_kind ->
  string ->
  (detection list * banned_construct list, string) result

val evaluate :
  dir:string ->
  manifest:manifest ->
  detections:detection list ->
  banned:banned_construct list ->
  analysis_result

val analyze_directory : string -> (analysis_result, string) result
val passed : analysis_result -> bool
val format_violation : violation -> string

val run :
  dir:string ->
  verbose:bool ->
  stdout:(string -> unit) ->
  stderr:(string -> unit) ->
  int
