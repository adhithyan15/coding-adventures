(** The mutator algebra (VER06 section 3, VER06r section 2).

    Offsets, counts, deltas and values are [Bigint.t]: a fixture can feed a
    1,000-digit count and a flip costs O(len) regardless. No big integer is ever
    built from input bytes. *)

type endian = Little | Big

val parse_endian : string -> endian option
(** ["little"] and ["big"] only. *)

val interest8 : int64 list
val interest16 : int64 list
val interest32 : int64 list

val interest64 : int64 list
(** Unsigned words: the last two have the top bit set. *)

val interest_by_width : (int * int64 list) list
val deltas : int list

val width_of : Bigint.t -> int
(** 1, 2, 4 or 8; otherwise 0, which makes the mutators the identity. *)

val bit_flip : string -> Bigint.t -> Bigint.t -> string
(** [bit_flip data bit_offset count] inverts [count] consecutive bits from
    [bit_offset mod (8 * len)], wrapping; by parity, so O(len). *)

val byte_flip : string -> Bigint.t -> Bigint.t -> string

val byte_arithmetic :
  string -> Bigint.t -> Bigint.t -> Bigint.t -> endian -> string
(** [byte_arithmetic data offset delta width endian]. *)

val boundary_insert :
  string -> Bigint.t -> Bigint.t -> Bigint.t -> endian -> string
(** [boundary_insert data offset width value endian]. *)

val block_delete : string -> Bigint.t -> Bigint.t -> string
val block_insert : string -> Bigint.t -> string -> string
val block_duplicate : string -> Bigint.t -> Bigint.t -> Bigint.t -> string
val splice : string -> string -> Bigint.t -> Bigint.t -> string

type seq_result = Work_ok of string * int | Work_exhausted of int

exception Work_cap_exceeded of int * int
(** work done, cap. *)

val mutate_sequence_counted :
  string -> Bigint.t -> Bigint.t -> int -> int -> seq_result
(** [mutate_sequence_counted data seed steps max_len cap]. The work is the bytes
    copied; the call stops at the step that would cross [cap]. Raises
    [Error.Argument_error] for [steps] above [Fuzz_limits.max_iterations] or an
    input above 1 MiB, before copying anything. *)

val mutate_sequence : string -> Bigint.t -> Bigint.t -> int -> string
(** With the per-call cap 2^28; raises [Work_cap_exceeded]. *)

val mutate_sequence_with_cap :
  int -> string -> Bigint.t -> Bigint.t -> int -> string
(** A test seam: an explicit per-call cap. *)
