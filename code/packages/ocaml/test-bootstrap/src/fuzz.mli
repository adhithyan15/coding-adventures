(** The OCaml coverage-guided fuzz engine (VER06r): the public facade.

    Every execution runs in its own domain with a fresh, abstract
    {!Fuzz_handle.t}. Coverage is what the target reports through
    {!Fuzz_handle.edge}. A non-cooperating tight loop cannot be stopped
    in-process (VER06r 4). *)

include module type of struct
  include Fuzz_types
end

type target = Fuzz_executor.target
type exec = Fuzz_engine.exec

val run : string list -> options -> target -> report
(** A bounded campaign. The tier is [caller-reported] when the target called
    [edge], else [black-box-uninstrumented]. *)

val run_outcomes : string list -> options -> exec -> report
val executor : target -> options -> exec
(** One domain per call, a private leak counter (no cap across calls). *)

val executor_tracked : Fuzz_executor.leaks -> target -> options -> exec

val verdict : report -> options -> exec -> string -> verdict
(** Re-runs every crash witness; only the report's own leak count is enforced. *)

val verdict_tracked :
  Fuzz_executor.leaks -> report -> options -> exec -> string -> verdict

val verdict_for : target -> report -> options -> string -> verdict
(** {!verdict} with the executor and the re-verification sharing ONE leak
    counter: the call to use. *)

val corpus_admit : int64 list -> int64 list -> bool * int
val calculate_energy : int -> int -> int -> int -> int
val tap_failures : report -> string -> int -> string list
val format_tap_failure : int -> string -> string -> string -> string
val crash_signature : string -> string list -> string
