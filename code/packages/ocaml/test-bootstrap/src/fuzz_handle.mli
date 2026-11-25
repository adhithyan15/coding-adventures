(** The per-execution coverage and budget handle (VER06r 5).

    A target receives a FRESH handle for every execution and reports coverage
    with {!edge}, burns its step budget with {!tick} and its memory budget with
    {!reserve}. The type is ABSTRACT: nothing here lets a holder reach the
    state. The representation and the test seams are in [Fuzz_handle_internal]
    (unsafe, tests only).

    After its execution a handle is RETIRED: {!edge} is a no-op and {!tick} and
    {!reserve} raise [Sentinel (token, Retired)]. A forged handle (another
    token) is inert in the same way. After a budget breach every {!tick} and
    {!reserve} raises again, whichever budget breached. *)

type t
type token
(** Opaque identity: only [Fuzz_handle_internal] can mint one. *)

type which = Steps | Mem | Retired

exception Sentinel of token * which
(** Raised by a breaching or retired call. An ordinary exception (a target CAN
    catch it): the guarantee is stickiness plus the engine's post-run verdict. *)

val wrap : Fuzz_handle_internal.t -> t
(** Engine and test use: hides the representation; there is no way back. *)

val token : t -> token

val edge_id : string -> int
(** A stable id derived from a name (FNV-1a). *)

val edge : t -> int -> unit
(** Records that edge [id] ran (reduced modulo 65,536). *)

val tick : t -> int -> unit
(** Burns [n] steps; raises when the budget is passed. A negative [n] charges
    nothing; a huge one is clamped. *)

val reserve : t -> int -> unit
(** Charges [n] bytes against the memory budget, together with the bytes the
    calling domain allocated since the execution started. *)
