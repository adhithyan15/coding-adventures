(* Public pure API for the HOS00 teaching model. *)

type error = ENOENT | ENOTDIR | EISDIR | EBADF | EMFILE | EINVAL
val string_of_error : error -> string

type event =
  | Opened of { path : string; inode : int; fd : int }
  | Open_failed of { path : string; error : error }
  | Read of { fd : int; inode : int; offset : int; bytes : int; next_offset : int }
  | Read_failed of { fd : int; error : error }
  | Closed of { fd : int }
  | Close_failed of { fd : int; error : error }
  | Listed of { path : string; inode : int; names : string list }
  | List_failed of { path : string; error : error }

type trace = { output : string; events : event list }

module Filesystem : sig
  type t
  val create_demo : unit -> t
  val open_file : t -> string -> (int * string, error) result
  val list : t -> string -> (int * string list, error) result
  val read_range : string -> offset:int -> count:int -> (string, error) result
end

module Descriptor_table : sig
  type t
  val create : capacity:int -> t
  val allocate : t -> inode:int -> data:string -> (int, error) result
  val read : t -> fd:int -> count:int -> (int * int * string * int, error) result
  val close : t -> fd:int -> (unit, error) result
  val offset : t -> fd:int -> (int, error) result
end

module Kernel : sig
  type t
  val create : filesystem:Filesystem.t -> descriptors:Descriptor_table.t -> t
  val create_demo : ?capacity:int -> unit -> t
  val open_file : t -> string -> (int * int, error) result
  val read : t -> fd:int -> count:int -> (int * int * string * int, error) result
  val close : t -> fd:int -> (unit, error) result
  val list : t -> string -> (int * string list, error) result
  val run_command : t -> string -> trace
end
