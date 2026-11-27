namespace HistoricalUnixFixtureAdapter

open System
open System.Collections.Generic
open System.Text.Json
open UnixHistorySimulator

module FixtureCli =
    let private plainEvent (e: Event) =
        let output = Dictionary<string, obj>()
        for KeyValue(key, value) in e do output.Add(key, value)
        output

    let private selfTest () =
        let rejectsNegativeCapacity =
            try
                DescriptorTable(-1) |> ignore
                false
            with :? ArgumentException -> true
        let fs = FileSystem.DemoImage()
        let table = DescriptorTable 2
        let first, second = table.Open 2, table.Open 2
        let passed =
            rejectsNegativeCapacity &&
            fs.Resolve("") = Error "EINVAL" && fs.Resolve("/bad\000path") = Error "EINVAL" &&
            fs.Resolve("/hello/..") = Error "ENOTDIR" && fs.Read(1, 0, 1) = Error "EISDIR" &&
            fs.Read(2, -1, 1) = Error "EINVAL" &&
            (match fs.Resolve "/empty" with Ok inode -> inode.Number = 3 | _ -> false) &&
            first = Some 0 && second = Some 1 && table.Open(2).IsNone && table.Get(-1).IsNone &&
            table.Advance(0, 3) && not (table.Advance(0, -1)) &&
            table.Get(0).Value.Offset = 3 && table.Get(1).Value.Offset = 0 &&
            table.Close(0) && not (table.Close(0)) && table.Open(2) = Some 0 &&
            (Kernel(fs, table).Read(99, 4) |> fst |> Map.find "error" |> unbox<string>) = "EBADF"
        passed

    [<EntryPoint>]
    let main args =
        if args.Length = 1 && args[0] = "--self-test" then
            if selfTest() then 0 else 1
        elif args.Length <> 1 then
            eprintfn "usage: FixtureCli COMMAND"
            2
        else
            let result = Shell.runCommand args[0]
            let payload = dict [ "output", box result.output; "events", box (result.events |> List.map plainEvent) ]
            let options = JsonSerializerOptions(PropertyNamingPolicy = JsonNamingPolicy.CamelCase)
            printfn "%s" (JsonSerializer.Serialize(payload, options))
            0
