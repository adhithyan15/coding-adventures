namespace UnixHistorySimulator

open System
open System.Text

/// A directory entry maps a name to an inode; list order follows this sequence.
type Entry = { Name: string; Inode: int }
type Inode = { Number: int; IsDirectory: bool; Entries: Entry list; Data: string }

/// Read-only in-memory image. No host filesystem state participates in path lookup.
type FileSystem(inodes: Inode list) =
    let byNumber = inodes |> List.map (fun n -> n.Number, n) |> Map.ofList
    static member DemoImage() =
        FileSystem [
            { Number = 1; IsDirectory = true; Entries = [ { Name = "hello"; Inode = 2 }; { Name = "empty"; Inode = 3 }; { Name = "etc"; Inode = 4 } ]; Data = "" }
            { Number = 2; IsDirectory = false; Entries = []; Data = "Hello, Unix!\n" }
            { Number = 3; IsDirectory = false; Entries = []; Data = "" }
            { Number = 4; IsDirectory = true; Entries = [ { Name = "notes"; Inode = 5 } ]; Data = "" }
            { Number = 5; IsDirectory = false; Entries = []; Data = "A directory maps a name to an inode.\nAn open descriptor tracks a byte offset.\n" }
        ]
    member _.Resolve(path: string) =
        if String.IsNullOrEmpty path || path.Contains(char 0) then Error "EINVAL"
        else
            match Map.tryFind 1 byNumber with
            | None -> Error "ENOENT"
            | Some root ->
                let mutable current = root
                let parents = ResizeArray<Inode>()
                let mutable problem: string option = None
                for name in path.Split('/') |> Array.filter (String.IsNullOrEmpty >> not) do
                    if problem.IsNone then
                        if not current.IsDirectory then problem <- Some "ENOTDIR"
                        elif name = "." then ()
                        elif name = ".." then
                            if parents.Count > 0 then current <- parents[parents.Count - 1]; parents.RemoveAt(parents.Count - 1)
                        else
                            match current.Entries |> List.tryFind (fun e -> e.Name = name) |> Option.bind (fun e -> Map.tryFind e.Inode byNumber) with
                            | None -> problem <- Some "ENOENT"
                            | Some child -> parents.Add current; current <- child
                match problem with
                | Some e -> Error e
                | None when path.EndsWith("/") && not current.IsDirectory -> Error "ENOTDIR"
                | None -> Ok current
    member _.Read(inode: int, offset: int, count: int) =
        if offset < 0 || count < 0 then Error "EINVAL"
        else match Map.tryFind inode byNumber with
             | None -> Error "ENOENT"
             | Some file when file.IsDirectory -> Error "EISDIR"
             | Some file ->
                 let bytes = Encoding.ASCII.GetBytes file.Data
                 if offset >= bytes.Length then Ok ""
                 else Ok(Encoding.ASCII.GetString(bytes, offset, min count (bytes.Length - offset)))

/// Descriptor-local offsets make separate opens independent.
type Descriptor = { Inode: int; Offset: int }
type DescriptorTable(capacity: int) =
    let slots =
        if capacity < 0 then invalidArg "capacity" "Capacity must be nonnegative."
        else Array.create capacity None
    member _.Get(fd: int) = if fd < 0 || fd >= slots.Length then None else slots[fd]
    member _.Open(inode: int) =
        match slots |> Array.tryFindIndex Option.isNone with
        | None -> None
        | Some fd -> slots[fd] <- Some { Inode = inode; Offset = 0 }; Some fd
    member _.Advance(fd: int, count: int) =
        match (if fd < 0 || fd >= slots.Length then None else slots[fd]) with
        | Some slot when count >= 0 && count <= Int32.MaxValue - slot.Offset -> slots[fd] <- Some { slot with Offset = slot.Offset + count }; true
        | _ -> false
    member _.Close(fd: int) =
        match (if fd < 0 || fd >= slots.Length then None else slots[fd]) with
        | None -> false
        | Some _ -> slots[fd] <- None; true

type Event = Map<string, obj>
module EventValues =
    let event fields: Event = fields |> Map.ofList
    let atom (x: 'a) : obj = box x
open EventValues

/// Kernel entry points sequence filesystem lookup and descriptor state changes.
type Kernel(files: FileSystem, descriptors: DescriptorTable) =
    member _.Open(path: string) =
        let baseFields = [ "call", atom "open"; "path", atom path ]
        match files.Resolve path with
        | Error e -> event (baseFields @ [ "error", atom e ])
        | Ok node when node.IsDirectory -> event (baseFields @ [ "error", atom "EISDIR" ])
        | Ok node -> match descriptors.Open node.Number with
                     | None -> event (baseFields @ [ "error", atom "EMFILE" ])
                     | Some fd -> event (baseFields @ [ "fd", atom fd; "inode", atom node.Number; "offset", atom 0 ])
    member _.Read(fd: int, count: int) =
        let baseFields = [ "call", atom "read"; "fd", atom fd ]
        match descriptors.Get fd with
        | None -> event (baseFields @ [ "error", atom "EBADF" ]), ""
        | Some slot ->
            match files.Read(slot.Inode, slot.Offset, count) with
            | Error e -> event (baseFields @ [ "error", atom e ]), ""
            | Ok data ->
                let length = Encoding.ASCII.GetByteCount data
                descriptors.Advance(fd, length) |> ignore
                event (baseFields @ [ "inode", atom slot.Inode; "offset", atom slot.Offset; "bytes", atom length; "nextOffset", atom (slot.Offset + length) ]), data
    member _.Close(fd: int) =
        let fields = [ "call", atom "close"; "fd", atom fd ]
        if descriptors.Close fd then event fields else event (fields @ [ "error", atom "EBADF" ])
    member _.List(path: string) =
        let fields = [ "call", atom "list"; "path", atom path ]
        match files.Resolve path with
        | Error e -> [], event (fields @ [ "error", atom e ])
        | Ok node when not node.IsDirectory -> [], event (fields @ [ "error", atom "ENOTDIR" ])
        | Ok node -> let names = node.Entries |> List.map (fun e -> e.Name)
                     names, event (fields @ [ "names", atom (List.toArray names); "inode", atom node.Number ])

type Result = { output: string; events: Event list }

/// Teaching shell adapter; components above can be used directly without command parsing.
module Shell =
    let runCommand (command: string) =
        let kernel = Kernel(FileSystem.DemoImage(), DescriptorTable 4)
        let output, events = StringBuilder(), ResizeArray<Event>()
        let add e =
            events.Add e
            match Map.tryFind "error" e with
            | Some code -> output.Append("error: ").Append(code).Append('\n') |> ignore
            | None -> ()
        let words = command.Split([| ' '; '\t'; '\n'; '\r'; '\f'; '\v' |], StringSplitOptions.RemoveEmptyEntries)
        if words.Length = 0 then ()
        elif words[0] = "cat" && words.Length = 2 then
            let opened = kernel.Open words[1]
            add opened
            match Map.tryFind "fd" opened with
            | Some fd ->
                let number = unbox<int> fd
                let mutable more = true
                while more do
                    let e, data = kernel.Read(number, 8)
                    add e
                    if data = "" then more <- false else output.Append(data) |> ignore
                add (kernel.Close number)
            | None -> ()
        elif words[0] = "ls" && words.Length <= 2 then
            let names, e = kernel.List(if words.Length = 2 then words[1] else "/")
            add e
            names |> List.iter (fun name -> output.Append(name).Append('\n') |> ignore)
        else output.Append("usage: cat PATH or ls [PATH]\n") |> ignore
        { output = output.ToString(); events = List.ofSeq events }
