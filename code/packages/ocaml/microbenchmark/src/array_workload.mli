(** Fixed same-array indexed versus native-iteration workload. *)

val workload_id : string
val make_array : unit -> int array
val expected_checksum : int64
val indexed_batch : int array -> int64 -> (int64, string) result
val iteration_batch : int array -> int64 -> (int64, string) result
