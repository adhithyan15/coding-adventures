(** One target execution in its own domain (VER06r 4). *)

type target = string -> Fuzz_handle.t -> unit
(** A fuzz target: it gets the input and a fresh per-execution handle. Any
    synchronous exception escaping it is a crash. *)

type exec_result = {
  outcome : Fuzz_types.outcome;
  edges : int list;  (** the edges of THIS execution, read after retirement *)
  edge_calls : int;
  leaked : bool;  (** the worker never answered (leaked, never joined) *)
}

type leaks = int Atomic.t
(** A leak counter shared by every execution of a campaign. *)

val new_leaks : unit -> leaks

val global_leaks : int Atomic.t
(** Leaked worker domains alive in the whole process, across campaigns: counted
    when a worker is given up on, subtracted when it finally returns. At
    {!Fuzz_limits.max_global_leaked_workers} {!run_tracked} refuses to start
    another execution (a test seam: tests set it). *)

val stale_kind : string

val run_tracked :
  ?cross_check:bool ->
  ?on_handle:(Fuzz_handle_internal.t -> unit) ->
  ?kill_wait_ms:int ->
  ?leaked_kill_wait_ms:int ->
  leaks ->
  target ->
  string ->
  Fuzz_types.options ->
  exec_result
(** Runs [target] on [input] in a fresh domain, then retires the handle and
    judges the run. [Sys.Break] and [Out_of_memory] raised by the target are
    re-raised here. After the first leak the shorter [leaked_kill_wait_ms] is
    used (CPU milliseconds; defaults 2000 and 500). [cross_check] (default true)
    and [on_handle] are test seams. *)

val run :
  ?cross_check:bool ->
  ?on_handle:(Fuzz_handle_internal.t -> unit) ->
  ?kill_wait_ms:int ->
  ?leaked_kill_wait_ms:int ->
  target ->
  string ->
  Fuzz_types.options ->
  exec_result
(** {!run_tracked} with a private leak counter. *)
