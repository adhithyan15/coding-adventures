(** Paired benchmark collection and lossless strict report replay. *)

type config = {
  pairs : int;
  confidence_percent : int;
  minimum_factor : float;
  seed : int64;
  target_ns : int64;
  max_batch : int64;
}

type order = Baseline_first | Candidate_first
type pair = { baseline_ns : int64; candidate_ns : int64; order : order }
type interval = { low : float; high : float }
type analysis = { ratio : float; interval : interval option; verdict : string }

type metadata = {
  workload : string;
  invocation : string list;
  revision : string;
  runtime : string;
  os : string;
  arch : string;
  profile : string option;
}

type report = {
  config : config;
  batch_size : int64;
  pairs : pair list;
  analysis : analysis;
  metadata : metadata option;
}

type batch = int64 -> (int64, string) result
type clock = unit -> int64

val default_config : config
val analyze : pair list -> int -> float -> (analysis, string) result

val collect :
  config -> int64 -> batch -> batch -> clock -> (report, string) result

val encode_report : report -> string
val replay_report : string -> (report, string) result

val default_metadata :
  string list -> string -> string -> string -> string -> metadata
