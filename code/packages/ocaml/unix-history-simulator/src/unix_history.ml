(*
   unix_history.ml — Pure First Edition Unix teaching components.

   The model uses an in-memory tree and explicit descriptor state. It does not
   call the host filesystem, create processes, read the environment, or print
   a trace. The fixture adapter owns all JSON and host I/O.
*)

type error = ENOENT | ENOTDIR | EISDIR | EBADF | EMFILE | EINVAL

let string_of_error = function
  | ENOENT -> "ENOENT"
  | ENOTDIR -> "ENOTDIR"
  | EISDIR -> "EISDIR"
  | EBADF -> "EBADF"
  | EMFILE -> "EMFILE"
  | EINVAL -> "EINVAL"

type event =
  | Opened of { path : string; inode : int; fd : int }
  | Open_failed of { path : string; error : error }
  | Read of {
      fd : int;
      inode : int;
      offset : int;
      bytes : int;
      next_offset : int;
    }
  | Read_failed of { fd : int; error : error }
  | Closed of { fd : int }
  | Close_failed of { fd : int; error : error }
  | Listed of { path : string; inode : int; names : string list }
  | List_failed of { path : string; error : error }

type trace = { output : string; events : event list }

module Filesystem = struct
  type node =
    | File of { inode : int; name : string; data : string }
    | Directory of { inode : int; name : string; entries : node list }

  type t = { root : node }

  let name = function File file -> file.name | Directory dir -> dir.name

  let create_demo () =
    let hello = File { inode = 2; name = "hello"; data = "Hello, Unix!\n" } in
    let empty = File { inode = 3; name = "empty"; data = "" } in
    let etc =
      Directory
        {
          inode = 4;
          name = "etc";
          entries = [ File { inode = 5; name = "notes"; data = "" } ];
        }
    in
    {
      root =
        Directory { inode = 1; name = "/"; entries = [ hello; empty; etc ] };
    }

  let path_parts path =
    if path = "" || String.contains path '\000' then Error EINVAL
    else
      let parts =
        String.split_on_char '/' path
        |> List.filter (fun part -> part <> "")
      in
      Ok parts

  let resolve filesystem path =
    match path_parts path with
    | Error error -> Error error
    | Ok parts -> (
        let rec descend stack remaining =
          match remaining with
          | [] -> Ok (List.hd stack)
          | "." :: tail -> (
              match List.hd stack with
              | File _ -> Error ENOTDIR
              | Directory _ -> descend stack tail)
          | ".." :: tail -> (
              match List.hd stack with
              | File _ -> Error ENOTDIR
              | Directory _ ->
                  let parent =
                    match stack with
                    | _current :: (_ :: _ as rest) -> rest
                    | _ -> stack
                  in
                  descend parent tail)
          | part :: tail -> (
              match List.hd stack with
              | File _ -> Error ENOTDIR
              | Directory dir -> (
                  match
                    List.find_opt (fun child -> name child = part) dir.entries
                  with
                  | None -> Error ENOENT
                  | Some child -> descend (child :: stack) tail))
        in
        let trailing_slash =
          String.length path > 0 && path.[String.length path - 1] = '/'
        in
        match descend [ filesystem.root ] parts with
        | Error error -> Error error
        | Ok (File _) when trailing_slash -> Error ENOTDIR
        | result -> result)

  let open_file filesystem path =
    match resolve filesystem path with
    | Error error -> Error error
    | Ok (Directory _) -> Error EISDIR
    | Ok (File file) -> Ok (file.inode, file.data)

  let list filesystem path =
    match resolve filesystem path with
    | Error error -> Error error
    | Ok (File _) -> Error ENOTDIR
    | Ok (Directory dir) -> Ok (dir.inode, List.map name dir.entries)

  let read_range data ~offset ~count =
    if offset < 0 || count < 0 then Error EINVAL
    else if offset >= String.length data then Ok ""
    else Ok (String.sub data offset (min count (String.length data - offset)))
end

module Descriptor_table = struct
  type entry = { inode : int; data : string; mutable offset : int }
  type t = { capacity : int; slots : (int, entry) Hashtbl.t }

  let create ~capacity =
    if capacity < 0 then invalid_arg "descriptor capacity must not be negative";
    { capacity; slots = Hashtbl.create (max 1 capacity) }

  let allocate table ~inode ~data =
    let rec first_free fd =
      if fd >= table.capacity then None
      else if Hashtbl.mem table.slots fd then first_free (fd + 1)
      else Some fd
    in
    match first_free 0 with
    | None -> Error EMFILE
    | Some fd ->
        Hashtbl.add table.slots fd { inode; data; offset = 0 };
        Ok fd

  let read table ~fd ~count =
    if count < 0 then Error EINVAL
    else
      match Hashtbl.find_opt table.slots fd with
      | None -> Error EBADF
      | Some entry -> (
          match
            Filesystem.read_range entry.data ~offset:entry.offset ~count
          with
          | Error error -> Error error
          | Ok chunk ->
              let offset = entry.offset in
              entry.offset <- offset + String.length chunk;
              Ok (entry.inode, offset, chunk, entry.offset))

  let close table ~fd =
    if Hashtbl.mem table.slots fd then (
      Hashtbl.remove table.slots fd;
      Ok ())
    else Error EBADF

  let offset table ~fd =
    match Hashtbl.find_opt table.slots fd with
    | None -> Error EBADF
    | Some entry -> Ok entry.offset
end

module Kernel = struct
  type t = { filesystem : Filesystem.t; descriptors : Descriptor_table.t }

  let create ~filesystem ~descriptors = { filesystem; descriptors }

  let create_demo ?(capacity = 4) () =
    create
      ~filesystem:(Filesystem.create_demo ())
      ~descriptors:(Descriptor_table.create ~capacity)

  let open_file kernel path =
    match Filesystem.open_file kernel.filesystem path with
    | Error error -> Error error
    | Ok (inode, data) -> (
        match Descriptor_table.allocate kernel.descriptors ~inode ~data with
        | Error error -> Error error
        | Ok fd -> Ok (fd, inode))

  let read kernel ~fd ~count =
    Descriptor_table.read kernel.descriptors ~fd ~count

  let close kernel ~fd = Descriptor_table.close kernel.descriptors ~fd
  let list kernel path = Filesystem.list kernel.filesystem path

  let split_ascii_whitespace text =
    let is_space = function
      | ' ' | '\t' | '\n' | '\r' | '\012' | '\011' -> true
      | _ -> false
    in
    let length = String.length text in
    let rec skip index =
      if index < length && is_space text.[index] then skip (index + 1)
      else index
    in
    let rec take index =
      if index < length && not (is_space text.[index]) then take (index + 1)
      else index
    in
    let rec loop index words =
      let start = skip index in
      if start >= length then List.rev words
      else
        let finish = take start in
        loop finish (String.sub text start (finish - start) :: words)
    in
    loop 0 []

  let rec run_command kernel command =
    let events = ref [] in
    let emit event = events := event :: !events in
    let output = Buffer.create 32 in
    let words = split_ascii_whitespace command in
    (match words with
    | [ "cat"; path ] -> (
        match open_file kernel path with
        | Error error ->
            emit (Open_failed { path; error });
            Buffer.add_string output ("error: " ^ string_of_error error ^ "\n")
        | Ok (fd, inode) -> (
            emit (Opened { path; inode; fd });
            let rec read_next () =
              match read kernel ~fd ~count:8 with
              | Error error -> emit (Read_failed { fd; error })
              | Ok (inode, offset, bytes, next_offset) ->
                  emit
                    (Read
                       {
                         fd;
                         inode;
                         offset;
                         bytes = String.length bytes;
                         next_offset;
                       });
                  Buffer.add_string output bytes;
                  if bytes <> "" then read_next ()
            in
            read_next ();
            match close kernel ~fd with
            | Ok () -> emit (Closed { fd })
            | Error error -> emit (Close_failed { fd; error })))
    | [ "ls" ] -> run_list kernel "/" output emit
    | [ "ls"; path ] -> run_list kernel path output emit
    | [] -> ()
    | [ "cat" ] -> Buffer.add_string output "usage: cat PATH or ls [PATH]\n"
    | _ -> Buffer.add_string output "usage: cat PATH or ls [PATH]\n");
    { output = Buffer.contents output; events = List.rev !events }

  and run_list kernel path output emit =
    match list kernel path with
    | Error error ->
        emit (List_failed { path; error });
        Buffer.add_string output ("error: " ^ string_of_error error ^ "\n")
    | Ok (inode, names) ->
        emit (Listed { path; inode; names });
        List.iter (fun name -> Buffer.add_string output (name ^ "\n")) names
end
