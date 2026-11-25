(* The engine's own exceptions (VER05q section 4).

   The runner classifies exceptions by constructor, never by message text, so
   each way the engine can refuse has its own constructor. *)

(* A public entry point was called with arguments it refuses (min > max, a
   negative step budget, ...). Fail closed: never a pass. *)
exception Argument_error of string

(* A shrinker was asked to start from a value that does NOT falsify the
   property: a caller bug, loud by design. *)
exception Not_falsifying

(* Generation could not produce a value (a filter that never accepts, a spent
   budget). The runner reports it as property-exhausted. *)
exception Exhausted of string

(* The ENGINE cannot continue (a domain ceiling, a process-wide leak cap): never
   a verdict about the target, and never a pass.  The fuzz executor raises it
   rather than report a result it cannot stand behind (VER06r 4, 7). *)
exception Engine_error of string
