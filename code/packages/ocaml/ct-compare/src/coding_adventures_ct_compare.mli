(** Constant-time byte comparison and selection: the OCaml lane of CT01
    ([code/specs/CT01-constant-time-compare.md]).

    Inputs are immutable [string]s used as byte strings. Lengths are public;
    contents and the selection bit are secret. For valid equal-length inputs no
    control flow, memory index or loop bound depends on those secrets.

    {b What this does not promise.} Source-level shape only. OCaml native code
    is produced by an optimising compiler that is free to rewrite the loops, the
    runtime is garbage collected (the minor heap and compactor copy live data,
    so secret bytes may be duplicated and left behind), and the interpreter
    ([ocaml] bytecode) dispatches every instruction through a jump table. No
    wall-clock constant time, cache or speculation guarantee is made.

    {b Selection bit.} OCaml's [Bool.to_int] and [if] on the secret are
    conditionals, so the bit is an [int] that must be exactly [0] (pick [right])
    or [1] (pick [left]), and each equality has a bit-returning variant:
    [ct_select_bytes a b (ct_eq_bit x y)] converts no boolean. *)

type error =
  | Select_length_mismatch
      (** CT01's [CT_SELECT_LENGTH_MISMATCH]: the inputs differ in length. *)
  | Invalid_choice  (** The selection bit was neither [0] nor [1]. *)

val ct_eq_bit : string -> string -> int
(** [1] when the strings are equal, else [0]. A length mismatch is [0] and reads
    neither input. *)

val ct_eq_fixed_bit : string -> string -> int
(** Fixed-width companion of {!ct_eq_bit} (the same operation in OCaml). *)

val ct_eq : string -> string -> bool
(** Boolean form of {!ct_eq_bit}; the boolean is the public result. *)

val ct_eq_fixed : string -> string -> bool
(** Boolean form of {!ct_eq_fixed_bit}. *)

val ct_select_bytes : string -> string -> int -> (string, error) result
(** [ct_select_bytes left right bit] is a fresh copy of [left] when [bit = 1]
    and of [right] when [bit = 0]. [Error Select_length_mismatch] for unequal
    lengths and [Error Invalid_choice] for any other [bit]; both are decided
    before either input's contents are read. *)

val ct_eq_u64_bit : int64 -> int64 -> int
(** [1] when the two 64-bit patterns are equal, else [0]. [Int64.t] carries all
    64 bits, so [-1L] is [0xFFFFFFFFFFFFFFFF]. *)

val ct_eq_u64 : int64 -> int64 -> bool
(** Boolean form of {!ct_eq_u64_bit}. *)
