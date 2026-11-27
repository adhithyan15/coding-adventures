(* The canonical form of a generated value (VER05q section 6).

   The engine never looks inside a user's value: it asks the generator for this
   explicit description and renders, measures and compares THAT.  Polymorphic
   compare and equality raise on functional values and loop on cyclic data, so
   they are never applied to anything a user can build. *)

type t =
  | Nil
  | Bool of bool
  | Int of Bigint.t
  | Bytes of string
  | Text of string  (** UTF-8 *)
  | List of t list
  | Tuple of t list
