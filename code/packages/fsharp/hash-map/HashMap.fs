namespace CodingAdventures.HashMap.FSharp

open System.Collections.Generic

type HashMap<'K, 'V when 'K : equality>(?entries: seq<'K * 'V>) =
    let map =
        let inner = Dictionary<'K, 'V>()
        entries
        |> Option.defaultValue Seq.empty
        |> Seq.iter (fun (key, value) -> inner.[key] <- value)
        inner

    member _.Count = map.Count
    member _.Size = map.Count

    member _.Clone() =
        HashMap<'K, 'V>(map |> Seq.map (fun (KeyValue(key, value)) -> key, value))

    member _.Get(key: 'K) =
        match map.TryGetValue key with
        | true, value -> Some value
        | _ -> None

    member _.Has(key: 'K) = map.ContainsKey key

    member this.Set(key: 'K, value: 'V) =
        let dict = Dictionary<'K, 'V>(map)
        dict.[key] <- value
        HashMap<'K, 'V>(dict |> Seq.map (fun (KeyValue(k, v)) -> k, v))

    member this.Delete(key: 'K) =
        let dict = Dictionary<'K, 'V>(map)
        dict.Remove key |> ignore
        HashMap<'K, 'V>(dict |> Seq.map (fun (KeyValue(k, v)) -> k, v))

    member _.Clear() = HashMap<'K, 'V>()

    member _.Keys() = map.Keys |> Seq.toList
    member _.Values() = map.Values |> Seq.toList
    member _.Entries() = map |> Seq.map (fun (KeyValue(key, value)) -> key, value) |> Seq.toList
    member _.ToDictionary() = Dictionary<'K, 'V>(map)

module HashMap =
    let fromEntries entries = HashMap(entries)
