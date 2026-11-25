(* The sabotage seams of the fuzz corpus runner (VER06r 11).

   Every engine entry point the runner calls goes through this record, so a
   negative control can install ONE wrong function and the run must FAIL.  It is
   a value passed down (never a global): production code uses [default]. *)

type t = {
  bit_flip : string -> Bigint.t -> Bigint.t -> string;
  byte_flip : string -> Bigint.t -> Bigint.t -> string;
  byte_arithmetic :
    string -> Bigint.t -> Bigint.t -> Bigint.t -> Fuzz_mutate.endian -> string;
  boundary_insert :
    string -> Bigint.t -> Bigint.t -> Bigint.t -> Fuzz_mutate.endian -> string;
  block_delete : string -> Bigint.t -> Bigint.t -> string;
  block_insert : string -> Bigint.t -> string -> string;
  block_duplicate : string -> Bigint.t -> Bigint.t -> Bigint.t -> string;
  splice : string -> string -> Bigint.t -> Bigint.t -> string;
  mutate_sequence :
    string -> Bigint.t -> Bigint.t -> int -> int -> Fuzz_mutate.seq_result;
  corpus_admit : int64 list -> int64 list -> bool * int;
  duplicate_crash : string -> string list -> string -> string list -> bool;
  failure_id : string -> bool;
  format_tap : int -> string -> string -> string -> string;
  tfv_valid : string -> bool;
}

let default =
  {
    bit_flip = Fuzz_mutate.bit_flip;
    byte_flip = Fuzz_mutate.byte_flip;
    byte_arithmetic = Fuzz_mutate.byte_arithmetic;
    boundary_insert = Fuzz_mutate.boundary_insert;
    block_delete = Fuzz_mutate.block_delete;
    block_insert = Fuzz_mutate.block_insert;
    block_duplicate = Fuzz_mutate.block_duplicate;
    splice = Fuzz_mutate.splice;
    mutate_sequence = Fuzz_mutate.mutate_sequence_counted;
    corpus_admit = Fuzz_bitmap.corpus_admit;
    duplicate_crash = Fuzz_signature.duplicate_crash;
    failure_id = Fuzz_failure_ids.is_registered;
    format_tap = Fuzz_signature.format_tap_failure;
    tfv_valid = Tfv.valid;
  }
