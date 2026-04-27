namespace CodingAdventures.RespProtocol.FSharp

open System
open System.Collections.Generic
open System.Text

type RespValue =
    | RespSimpleString of string
    | RespErrorValue of string
    | RespInteger of int64
    | RespBulkString of string option
    | RespArray of RespValue list option

type RespDecodeResult =
    {
        Value: RespValue
        Consumed: int
    }

type RespDecodeAllResult =
    {
        Values: RespValue list
        Consumed: int
    }

exception RespDecodeError of string

module RespProtocol =
    let simpleString value = RespSimpleString value
    let errorValue value = RespErrorValue value
    let integer value = RespInteger value
    let bulkString value = RespBulkString value
    let array value = RespArray value

    let concatBytes (chunks: byte[] seq) =
        let arrays = chunks |> Seq.toArray
        let total = arrays |> Array.sumBy _.Length
        let output = Array.zeroCreate<byte> total
        let mutable offset = 0
        for chunk in arrays do
            Buffer.BlockCopy(chunk, 0, output, offset, chunk.Length)
            offset <- offset + chunk.Length
        output

    let rec encode value =
        match value with
        | RespSimpleString text -> Encoding.UTF8.GetBytes(sprintf "+%s\r\n" text)
        | RespErrorValue text -> Encoding.UTF8.GetBytes(sprintf "-%s\r\n" text)
        | RespInteger number -> Encoding.UTF8.GetBytes(sprintf ":%d\r\n" number)
        | RespBulkString None -> Encoding.UTF8.GetBytes("$-1\r\n")
        | RespBulkString (Some text) ->
            let bytes = Encoding.UTF8.GetBytes text
            concatBytes [ Encoding.UTF8.GetBytes(sprintf "$%d\r\n" bytes.Length); bytes; Encoding.UTF8.GetBytes("\r\n") ]
        | RespArray None -> Encoding.UTF8.GetBytes("*-1\r\n")
        | RespArray (Some values) ->
            concatBytes ([ Encoding.UTF8.GetBytes(sprintf "*%d\r\n" values.Length) ] @ (values |> List.map encode))

    let private readLine (buffer: byte[]) start =
        let mutable index = start
        let mutable found = None
        while found.IsNone && index < buffer.Length - 1 do
            if buffer.[index] = byte '\r' && buffer.[index + 1] = byte '\n' then
                found <- Some (buffer.[start .. index - 1], index + 2)
            else
                index <- index + 1
        found

    let rec decode (buffer: byte[]) =
        if buffer.Length = 0 then
            None
        else
            match char buffer.[0] with
            | '+' ->
                readLine buffer 1 |> Option.map (fun (line, consumed) -> { Value = RespSimpleString(Encoding.UTF8.GetString line); Consumed = consumed })
            | '-' ->
                readLine buffer 1 |> Option.map (fun (line, consumed) -> { Value = RespErrorValue(Encoding.UTF8.GetString line); Consumed = consumed })
            | ':' ->
                readLine buffer 1
                |> Option.map (fun (line, consumed) -> { Value = RespInteger(Int64.Parse(Encoding.UTF8.GetString line)); Consumed = consumed })
            | '$' ->
                match readLine buffer 1 with
                | None -> None
                | Some (line, consumed) ->
                    let length = Int32.Parse(Encoding.UTF8.GetString line)
                    if length = -1 then
                        Some { Value = RespBulkString None; Consumed = consumed }
                    elif buffer.Length < consumed + length + 2 then
                        None
                    else
                        let text = Encoding.UTF8.GetString(buffer.[consumed .. consumed + length - 1])
                        Some { Value = RespBulkString(Some text); Consumed = consumed + length + 2 }
            | '*' ->
                match readLine buffer 1 with
                | None -> None
                | Some (line, consumed) ->
                    let count = Int32.Parse(Encoding.UTF8.GetString line)
                    if count = -1 then
                        Some { Value = RespArray None; Consumed = consumed }
                    else
                        let mutable offset = consumed
                        let mutable values = []
                        let mutable failed = false
                        for _ in 1 .. count do
                            match decode buffer.[offset..] with
                            | Some result ->
                                values <- values @ [ result.Value ]
                                offset <- offset + result.Consumed
                            | None -> failed <- true
                        if failed then None else Some { Value = RespArray(Some values); Consumed = offset }
            | _ ->
                match readLine buffer 0 with
                | None -> None
                | Some (line, consumed) ->
                    let parts =
                        (Encoding.UTF8.GetString line).Split([| ' '; '\t' |], StringSplitOptions.RemoveEmptyEntries)
                        |> Array.toList
                        |> List.map (Some >> RespBulkString)
                    Some { Value = RespArray(Some parts); Consumed = consumed }

    let decodeText (text: string) = decode (Encoding.UTF8.GetBytes text)

    let decodeAll (buffer: byte[]) =
        let mutable values = []
        let mutable offset = 0
        let mutable running = true
        while running && offset < buffer.Length do
            match decode buffer.[offset..] with
            | Some result ->
                values <- values @ [ result.Value ]
                offset <- offset + result.Consumed
            | None -> running <- false
        { Values = values; Consumed = offset }

    let decodeAllText (text: string) = decodeAll (Encoding.UTF8.GetBytes text)

type RespDecoder() =
    let queue = Queue<RespValue>()
    let mutable buffer = Array.empty<byte>

    let rec drain () =
        match RespProtocol.decode buffer with
        | Some result ->
            queue.Enqueue result.Value
            buffer <- buffer.[result.Consumed..]
            if buffer.Length > 0 then drain ()
        | None -> ()

    member _.Feed(data: byte[]) =
        buffer <- Array.append buffer data
        drain ()

    member this.Feed(text: string) =
        this.Feed(Encoding.UTF8.GetBytes text)

    member _.HasMessage() = queue.Count > 0

    member _.GetMessage() =
        if queue.Count = 0 then
            raise (RespDecodeError "decoder buffer is empty")
        queue.Dequeue()
