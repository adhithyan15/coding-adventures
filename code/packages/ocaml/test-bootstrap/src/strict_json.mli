(** A small, STRICT JSON reader (VER05q section 7).

    A conformance fixture is DATA, and a lenient parser quietly papers over
    exactly the corruption a conformance check exists to notice. *)

type t =
  | Null
  | Bool of bool
  | Number of string
      (** kept as its literal text: never converted to a float *)
  | String of string  (** UTF-8 *)
  | Array of t list
  | Object of (string * t) list

val max_depth : int
val max_bytes : int
val max_nodes : int

val parse : string -> (t, string) result
(** Never raises. Refuses: documents over 8 MiB or 200,000 values (object keys
    count), nesting over 64 containers (refused BEFORE recursing), invalid
    UTF-8, a byte-order mark, raw control characters in strings, lone or
    reversed surrogate escapes, leading zeros, [+1], [NaN], comments, trailing
    commas, trailing data, and duplicate object keys (compared by decoded
    bytes). *)
