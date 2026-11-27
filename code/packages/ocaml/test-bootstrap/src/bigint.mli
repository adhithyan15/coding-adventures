(** Small arbitrary-precision signed integers (VER05q section 3).

    The OCaml standard library has no big integers and Zarith is a third-party
    dependency, so the engine carries just what it needs. Values are immutable.
*)

type t

val zero : t
val one : t
val of_int : int -> t

val of_int64 : int64 -> t
(** Signed interpretation of the word. *)

val of_uint64 : int64 -> t
(** Unsigned interpretation of the word: [-1L] is [2^64 - 1]. *)

val to_int_opt : t -> int option
(** [None] when the value does not fit a native [int]. *)

val to_int64_wrap : t -> int64
(** The low 64 bits of the two's-complement form, exactly the reference's mask.
*)

val of_string : string -> t
(** Canonical or not, decimal digits with an optional leading [-]. Raises
    [Invalid_argument] on anything else (and on more than 100,000 digits). *)

val to_string : t -> string
val compare : t -> t -> int
val equal : t -> t -> bool
val sign : t -> int
val neg : t -> t
val abs : t -> t
val add : t -> t -> t
val sub : t -> t -> t
val mul : t -> t -> t

val div_rem : t -> t -> t * t
(** Truncated division (quotient rounds toward zero, remainder has the sign of
    the dividend). Raises [Division_by_zero]. *)

val fdiv2 : t -> t
(** Floored division by 2: [fdiv2 (of_int (-81)) = of_int (-41)]. *)

val rem_pos : t -> t -> t
(** Remainder in the half-open range from 0 up to (excluding) the absolute value
    of the divisor, for any sign of the dividend. *)

val shift_left : t -> int -> t
(** Multiplies by [2^k], [k >= 0]. *)

val shift_right : t -> int -> t
(** Truncates the magnitude (rounds toward zero). *)

val bit_length : t -> int
(** Bits of the magnitude; 0 for zero. *)

val pow2 : int -> t
