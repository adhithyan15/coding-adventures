namespace CodingAdventures.InMemoryDataStoreEngine.FSharp

open System
open System.Collections.Generic
open CodingAdventures.HashMap.FSharp
open CodingAdventures.HashSet.FSharp
open CodingAdventures.Heap.FSharp
open CodingAdventures.HyperLogLog.FSharp
open CodingAdventures.InMemoryDataStoreProtocol.FSharp
open CodingAdventures.RespProtocol.FSharp
open CodingAdventures.SkipList.FSharp

type EntryType =
    | String
    | Hash
    | List
    | Set
    | ZSet
    | Hll

type SortedEntry =
    {
        Score: float
        Member: string
    }

type SortedSet() =
    let comparator left right =
        let byScore = compare left.Score right.Score
        if byScore <> 0 then byScore else StringComparer.Ordinal.Compare(left.Member, right.Member)

    let mutable members = HashMap<string, float>()
    let ordering = SkipList<SortedEntry, unit>(comparator)

    member _.Clone() =
        let next = SortedSet()
        ordering.EntriesList()
        |> List.iter (fun entry -> next.Insert(entry.Key.Score, entry.Key.Member) |> ignore)
        next

    member _.Insert(score: float, memberName: string) =
        let isNew = not (members.Has memberName)
        members.Get memberName |> Option.iter (fun current -> ordering.Delete({ Score = current; Member = memberName }) |> ignore)
        members <- members.Set(memberName, score)
        ordering.Insert({ Score = score; Member = memberName }, ())
        isNew

    member _.Count = members.Size

type EntryValue =
    | StringValue of string
    | HashValue of HashMap<string, string>
    | ListValue of string list
    | SetValue of HashSet<string>
    | ZSetValue of SortedSet
    | HllValue of HyperLogLog

type Entry =
    {
        EntryType: EntryType
        Value: EntryValue
        ExpiresAt: int64 option
    }

module DataStoreTypes =
    let stringEntry value expiresAt = { EntryType = String; Value = StringValue value; ExpiresAt = expiresAt }
    let hashEntry value expiresAt = { EntryType = Hash; Value = HashValue value; ExpiresAt = expiresAt }
    let listEntry value expiresAt = { EntryType = List; Value = ListValue value; ExpiresAt = expiresAt }
    let setEntry value expiresAt = { EntryType = Set; Value = SetValue value; ExpiresAt = expiresAt }
    let zsetEntry value expiresAt = { EntryType = ZSet; Value = ZSetValue value; ExpiresAt = expiresAt }
    let hllEntry value expiresAt = { EntryType = Hll; Value = HllValue value; ExpiresAt = expiresAt }

type Database(?entries: HashMap<string, Entry>, ?ttlHeap: MinHeap<int64 * string>) =
    let mutable entries = defaultArg entries (HashMap<string, Entry>())
    let mutable ttlHeap = defaultArg ttlHeap (MinHeap<int64 * string>(fun (leftTime, leftKey) (rightTime, rightKey) -> if leftTime <> rightTime then compare leftTime rightTime else compare leftKey rightKey))

    static member CurrentTimeMs() = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()

    member _.Clone() =
        let heap = MinHeap<int64 * string>(fun (leftTime, leftKey) (rightTime, rightKey) -> if leftTime <> rightTime then compare leftTime rightTime else compare leftKey rightKey)
        ttlHeap.ToArray() |> List.iter heap.Push
        Database(entries.Clone(), heap)

    member _.Entries with get () = entries and set value = entries <- value
    member _.TtlHeap with get () = ttlHeap and set value = ttlHeap <- value

    member this.Get(key: string) =
        match entries.Get key with
        | Some entry ->
            match entry.ExpiresAt with
            | Some expiresAt when Database.CurrentTimeMs() >= expiresAt -> None
            | _ -> Some entry
        | None -> None

    member this.Set(key: string, entry: Entry) =
        let next = this.Clone()
        next.Entries <- next.Entries.Set(key, entry)
        entry.ExpiresAt |> Option.iter (fun expiresAt -> next.TtlHeap.Push(expiresAt, key))
        next

    member this.Delete(key: string) =
        let next = this.Clone()
        next.Entries <- next.Entries.Delete key
        next

    member this.DbSize() =
        entries.Keys() |> List.filter (this.Get >> Option.isSome) |> List.length

type Store(?databases: Database list, ?activeDb: int) =
    let mutable databases = defaultArg databases (List.init 16 (fun _ -> Database()))
    let mutable activeDb = defaultArg activeDb 0

    static member Empty() = Store()

    member _.Databases with get () = databases and set value = databases <- value
    member _.ActiveDb with get () = activeDb and set value = activeDb <- value

    member _.Clone() = Store(databases |> List.map _.Clone(), activeDb)
    member _.CurrentDb() = databases.[activeDb]
    member this.Get(key: string) = this.CurrentDb().Get key
    member this.Exists(key: string) = this.Get(key) |> Option.isSome

    member this.Set(key: string, entry: Entry) =
        let next = this.Clone()
        next.Databases <- next.Databases |> List.mapi (fun index database -> if index = activeDb then database.Set(key, entry) else database)
        next

    member this.Delete(key: string) =
        let next = this.Clone()
        next.Databases <- next.Databases |> List.mapi (fun index database -> if index = activeDb then database.Delete(key) else database)
        next

    member this.DbSize() = this.CurrentDb().DbSize()
    member this.FlushDb() =
        let next = this.Clone()
        next.Databases <- next.Databases |> List.mapi (fun index database -> if index = activeDb then Database() else database)
        next

    member this.Select(index: int) =
        let next = this.Clone()
        next.ActiveDb <- max 0 (min (databases.Length - 1) index)
        next

type DataStoreModule =
    abstract member Register: DataStoreEngine -> unit

and DataStoreEngine(?store: Store) as this =
    let handlers = Dictionary<string, Store -> string list -> Store * RespValue>(StringComparer.Ordinal)
    let mutable storeState = defaultArg store (Store.Empty())

    let wrongArgCount command = RespErrorValue(sprintf "ERR wrong number of arguments for '%s'" command)
    let wrongType = RespErrorValue "WRONGTYPE Operation against a key holding the wrong kind of value"
    let invalidInteger = RespErrorValue "ERR value is not an integer or out of range"

    let setString key value expiresAt =
        storeState <- storeState.Set(key, DataStoreTypes.stringEntry value expiresAt)

    let parseInteger (value: string) =
        match Int64.TryParse value with
        | true, result -> Some result
        | _ -> None

    let install name handler = handlers.[name] <- handler

    do
        install "PING" (fun store args ->
            match args with
            | [] -> store, RespSimpleString "PONG"
            | [ value ] -> store, RespBulkString(Some value)
            | _ -> store, wrongArgCount "PING")

        install "ECHO" (fun store args ->
            match args with
            | [ value ] -> store, RespBulkString(Some value)
            | _ -> store, wrongArgCount "ECHO")

        install "SET" (fun store args ->
            match args with
            | [ key; value ] -> store.Set(key, DataStoreTypes.stringEntry value None), RespSimpleString "OK"
            | _ -> store, wrongArgCount "SET")

        install "GET" (fun store args ->
            match args with
            | [ key ] ->
                match store.Get key with
                | Some { EntryType = String; Value = StringValue value } -> store, RespBulkString(Some value)
                | Some _ -> store, wrongType
                | None -> store, RespBulkString None
            | _ -> store, wrongArgCount "GET")

        install "INCR" (fun store args ->
            match args with
            | [ key ] ->
                let current =
                    match store.Get key with
                    | Some { EntryType = String; Value = StringValue value } ->
                        match parseInteger value with
                        | Some number -> number
                        | None -> raise (Exception "ERR")
                    | Some _ -> raise (Exception "WRONGTYPE")
                    | None -> 0L
                let next = current + 1L
                store.Set(key, DataStoreTypes.stringEntry (string next) None), RespInteger next
            | _ -> store, wrongArgCount "INCR")

        install "HSET" (fun store args ->
            match args with
            | [ key; field; value ] ->
                let map =
                    match store.Get key with
                    | Some { EntryType = Hash; Value = HashValue current } -> current
                    | Some _ -> raise (Exception "WRONGTYPE")
                    | None -> HashMap<string, string>()
                let added = if map.Has field then 0L else 1L
                store.Set(key, DataStoreTypes.hashEntry (map.Set(field, value)) None), RespInteger added
            | _ -> store, wrongArgCount "HSET")

        install "SADD" (fun store args ->
            match args with
            | key :: members when members.Length > 0 ->
                let set =
                    match store.Get key with
                    | Some { EntryType = Set; Value = SetValue current } -> current
                    | Some _ -> raise (Exception "WRONGTYPE")
                    | None -> HashSet<string>()
                let mutable added = 0L
                let mutable nextSet = set
                for item in members do
                    if not (nextSet.Has item) then added <- added + 1L
                    nextSet <- nextSet.Add item
                store.Set(key, DataStoreTypes.setEntry nextSet None), RespInteger added
            | _ -> store, wrongArgCount "SADD")

        install "LPUSH" (fun store args ->
            match args with
            | key :: values when values.Length > 0 ->
                let current =
                    match store.Get key with
                    | Some { EntryType = List; Value = ListValue items } -> items
                    | Some _ -> raise (Exception "WRONGTYPE")
                    | None -> []
                let next = (List.rev values) @ current
                store.Set(key, DataStoreTypes.listEntry next None), RespInteger(int64 next.Length)
            | _ -> store, wrongArgCount "LPUSH")

        install "ZADD" (fun store args ->
            match args with
            | [ key; scoreText; memberName ] ->
                let score = Double.Parse scoreText
                let zset =
                    match store.Get key with
                    | Some { EntryType = ZSet; Value = ZSetValue current } -> current
                    | Some _ -> raise (Exception "WRONGTYPE")
                    | None -> SortedSet()
                let added = if zset.Insert(score, memberName) then 1L else 0L
                store.Set(key, DataStoreTypes.zsetEntry zset None), RespInteger added
            | _ -> store, wrongArgCount "ZADD")

        install "PFADD" (fun store args ->
            match args with
            | key :: members when members.Length > 0 ->
                let hll =
                    match store.Get key with
                    | Some { EntryType = Hll; Value = HllValue current } -> current.Clone()
                    | Some _ -> raise (Exception "WRONGTYPE")
                    | None -> HyperLogLog()
                let before = hll.Count()
                members |> List.iter (box >> hll.Add)
                let changed = if hll.Count() <> before then 1L else 0L
                store.Set(key, DataStoreTypes.hllEntry hll None), RespInteger changed
            | _ -> store, wrongArgCount "PFADD")

        install "EXPIRE" (fun store args ->
            match args with
            | [ key; secondsText ] ->
                match parseInteger secondsText, store.Get key with
                | Some seconds, Some entry ->
                    let expiresAt = Database.CurrentTimeMs() + (seconds * 1000L)
                    store.Set(key, { entry with ExpiresAt = Some expiresAt }), RespInteger 1L
                | Some _, None -> store, RespInteger 0L
                | None, _ -> store, invalidInteger
            | _ -> store, wrongArgCount "EXPIRE")

        install "TTL" (fun store args ->
            match args with
            | [ key ] ->
                match store.Get key with
                | None -> store, RespInteger -2L
                | Some { ExpiresAt = None } -> store, RespInteger -1L
                | Some { ExpiresAt = Some expiresAt } ->
                    let remaining = max 0L ((expiresAt - Database.CurrentTimeMs()) / 1000L)
                    store, RespInteger remaining
            | _ -> store, wrongArgCount "TTL")

        install "DBSIZE" (fun store args ->
            match args with
            | [] -> store, RespInteger(int64 (store.DbSize()))
            | _ -> store, wrongArgCount "DBSIZE")

        install "FLUSHDB" (fun store args ->
            match args with
            | [] -> store.FlushDb(), RespSimpleString "OK"
            | _ -> store, wrongArgCount "FLUSHDB")

    member _.Store = storeState

    member _.RegisterModule(moduleImpl: DataStoreModule) =
        moduleImpl.Register(this)
        this

    member _.Reset(nextStore: Store) =
        storeState <- nextStore

    member _.Execute(command: DataStoreCommand) =
        this.Execute(command.Name :: command.Args)

    member this.Execute(parts: string list) =
        match parts with
        | [] -> RespErrorValue "ERR empty command"
        | command :: args ->
            match handlers.TryGetValue(command.Trim().ToUpperInvariant()) with
            | true, handler ->
                try
                    let nextStore, response = handler storeState args
                    storeState <- nextStore
                    response
                with
                | :? Exception as ex when ex.Message = "WRONGTYPE" -> wrongType
                | :? Exception as ex when ex.Message = "ERR" -> invalidInteger
            | _ -> RespErrorValue(sprintf "ERR unknown command '%s'" (command.Trim().ToUpperInvariant()))
