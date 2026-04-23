namespace CodingAdventures.SkipList.FSharp

open System.Collections.Generic

type Comparator<'T> = 'T -> 'T -> int

type SkipList<'K, 'V when 'K : comparison>(?comparator: Comparator<'K>) =
    let cmp = defaultArg comparator compare
    let items = ResizeArray<'K * 'V>()

    let findIndex key =
        items |> Seq.tryFindIndex (fun (current, _) -> cmp current key = 0)

    let rec findInsert key low high =
        if low >= high then low
        else
            let mid = (low + high) / 2
            if cmp (fst items.[mid]) key < 0 then
                findInsert key (mid + 1) high
            else
                findInsert key low mid

    member _.Insert(key: 'K, value: 'V) =
        match findIndex key with
        | Some index -> items.[index] <- key, value
        | None ->
            let index = findInsert key 0 items.Count
            items.Insert(index, (key, value))

    member _.Delete(key: 'K) =
        match findIndex key with
        | Some index -> items.RemoveAt(index); true
        | None -> false

    member _.Search(key: 'K) =
        findIndex key |> Option.map (fun index -> snd items.[index])

    member _.Contains(key: 'K) = findIndex key |> Option.isSome
    member _.Rank(key: 'K) = findIndex key
    member _.EntriesList() = items |> Seq.toList |> List.map KeyValuePair
    member _.Length = items.Count
    member _.IsEmpty() = items.Count = 0
