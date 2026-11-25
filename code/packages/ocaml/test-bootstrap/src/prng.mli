(** SplitMix64 on [int64] words (VER05 2, VER05q 3).

    A stream is a mutable cell and must not be shared between domains. *)

type t

val golden_gamma : int64
val mix : int64 -> int64

val create : ?threshold:(int64 -> int64) -> int64 -> t
(** [create seed]. [threshold] is the rejection threshold of [bounded] (default
    {!default_threshold}); it exists so a negative control can break it. *)

val state : t -> int64
val next_u64 : t -> int64

val default_threshold : int64 -> int64
(** [(2^64 - n) mod n], computed unsigned. *)

val bounded_u64 : t -> int64 -> int64
(** [bounded_u64 r n]: uniform on 0 .. n-1 for [n] read as UNSIGNED, [n <> 0].
    [bounded(1)] consumes one word. *)

val bounded : t -> Bigint.t -> Bigint.t
(** Uniform on 0 .. n-1 for [1 <= n <= 2^64]; [2^64] is one raw word. Raises
    [Error.Argument_error] outside that range. *)

val bounded_int : t -> int -> int
(** [bounded] for a native [n >= 1] (a native [int] is 63-bit, so every value is
    below 2^62 and fits a word). *)

val bounded_big : t -> Bigint.t -> Bigint.t
(** Uniform on 0 .. span-1 for any [span >= 1] (VER05 7.2): {!bounded} up to
    2^64, k-word rejection above. *)

val fnv1a64 : string -> int64
val sub_seed : int64 -> string -> int64
