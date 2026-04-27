namespace CodingAdventures.HashSet.FSharp

open CodingAdventures.HashMap.FSharp

type HashSet<'T when 'T : equality>(?entries: seq<'T>) =
    let map =
        entries
        |> Option.defaultValue Seq.empty
        |> Seq.fold (fun (state: HashMap<'T, bool>) value -> state.Set(value, true)) (HashMap<'T, bool>())

    member _.Count = map.Size
    member _.Size = map.Size

    member _.Clone() =
        HashSet<'T>(map.Keys())

    member _.Has(value: 'T) = map.Has value
    member _.Contains(value: 'T) = map.Has value
    member _.IsEmpty() = map.Size = 0
    member _.ToList() = map.Keys()

    member this.Add(value: 'T) =
        HashSet<'T>((value :: this.ToList()) |> Seq.distinct)

    member this.Remove(value: 'T) =
        HashSet<'T>(this.ToList() |> List.filter ((<>) value))

    member this.Union(other: HashSet<'T>) =
        HashSet<'T>(Seq.append (this.ToList()) (other.ToList()) |> Seq.distinct)

    member this.Intersection(other: HashSet<'T>) =
        HashSet<'T>(this.ToList() |> List.filter other.Has)

    member this.Difference(other: HashSet<'T>) =
        HashSet<'T>(this.ToList() |> List.filter (other.Has >> not))
