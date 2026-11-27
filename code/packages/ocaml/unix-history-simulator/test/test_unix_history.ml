(* Component regressions are pure and intentionally produce no success output. *)

let require condition message = if not condition then failwith message

let test_filesystem_ranges_and_paths () =
  let filesystem = Unix_history.Filesystem.create_demo () in
  require
    (Unix_history.Filesystem.read_range "abc" ~offset:1 ~count:9 = Ok "bc")
    "short byte-range read";
  require
    (Unix_history.Filesystem.read_range "abc" ~offset:3 ~count:1 = Ok "")
    "EOF byte-range read";
  require
    (Unix_history.Filesystem.read_range "abc" ~offset:(-1) ~count:1 = Error Unix_history.EINVAL)
    "negative byte offset";
  require
    (Unix_history.Filesystem.read_range "abc" ~offset:0 ~count:(-1) = Error Unix_history.EINVAL)
    "negative byte count";
  require
    (Unix_history.Filesystem.open_file filesystem "/missing" = Error Unix_history.ENOENT)
    "missing path";
  require
    (Unix_history.Filesystem.open_file filesystem "/hello/.." = Error Unix_history.ENOTDIR)
    "traversal through a regular file";
  require
    (Unix_history.Filesystem.open_file filesystem "/hello/" = Error Unix_history.ENOTDIR)
    "trailing slash on a regular file";
  require
    (Unix_history.Filesystem.open_file filesystem "/bad\000path" = Error Unix_history.EINVAL)
    "NUL path";
  require (Unix_history.Filesystem.open_file filesystem "" = Error Unix_history.EINVAL) "empty path"

let test_descriptor_capacity_and_reuse () =
  let rejected =
    try
      ignore (Unix_history.Descriptor_table.create ~capacity:(-1));
      false
    with Invalid_argument _ -> true
  in
  require rejected "negative descriptor capacity must be rejected";
  let empty = Unix_history.Descriptor_table.create ~capacity:0 in
  require
    (Unix_history.Descriptor_table.allocate empty ~inode:2 ~data:"x" = Error Unix_history.EMFILE)
    "zero capacity cannot allocate";
  let table = Unix_history.Descriptor_table.create ~capacity:1 in
  require (Unix_history.Descriptor_table.allocate table ~inode:2 ~data:"xyz" = Ok 0)
    "lowest descriptor allocation";
  (match Unix_history.Descriptor_table.read table ~fd:0 ~count:2 with
  | Ok (2, 0, "xy", 2) -> ()
  | _ -> failwith "first descriptor read");
  require
    (Unix_history.Descriptor_table.read table ~fd:0 ~count:(-1) = Error Unix_history.EINVAL)
    "negative descriptor read count";
  require (Unix_history.Descriptor_table.offset table ~fd:0 = Ok 2)
    "failed read does not change the offset";
  require (Unix_history.Descriptor_table.close table ~fd:0 = Ok ()) "close descriptor";
  require (Unix_history.Descriptor_table.allocate table ~inode:3 ~data:"" = Ok 0)
    "closed slot is reused"

let test_kernel_trace () =
  let kernel = Unix_history.Kernel.create_demo () in
  let trace = Unix_history.Kernel.run_command kernel "cat /hello" in
  require (trace.output = "Hello, Unix!\n") "cat output";
  match trace.events with
  | [ Unix_history.Opened { inode = 2; fd = 0; _ };
      Unix_history.Read { bytes = 8; next_offset = 8; _ };
      Unix_history.Read { bytes = 5; next_offset = 13; _ };
      Unix_history.Read { bytes = 0; next_offset = 13; _ };
      Unix_history.Closed { fd = 0 } ] -> ()
  | _ -> failwith "cat ordered event trace"

let () =
  test_filesystem_ranges_and_paths ();
  test_descriptor_capacity_and_reuse ();
  test_kernel_trace ()
