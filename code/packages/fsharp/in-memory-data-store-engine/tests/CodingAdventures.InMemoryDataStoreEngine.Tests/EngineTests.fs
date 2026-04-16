namespace CodingAdventures.InMemoryDataStoreEngine.FSharp.Tests

open CodingAdventures.InMemoryDataStoreEngine.FSharp
open CodingAdventures.InMemoryDataStoreProtocol.FSharp
open CodingAdventures.RespProtocol.FSharp
open Xunit

type EngineTests() =
    [<Fact>]
    member _.``executes core commands``() =
        let engine = DataStoreEngine()
        Assert.Equal(RespSimpleString "PONG", engine.Execute([ "PING" ]))
        Assert.Equal(RespSimpleString "OK", engine.Execute([ "SET"; "counter"; "1" ]))
        Assert.Equal(RespBulkString(Some "1"), engine.Execute([ "GET"; "counter" ]))
        Assert.Equal(RespInteger 2L, engine.Execute([ "INCR"; "counter" ]))

    [<Fact>]
    member _.``handles secondary structures``() =
        let engine = DataStoreEngine()
        Assert.Equal(RespInteger 1L, engine.Execute([ "HSET"; "hash"; "field"; "value" ]))
        Assert.Equal(RespInteger 2L, engine.Execute([ "SADD"; "set"; "a"; "b" ]))
        Assert.Equal(RespInteger 2L, engine.Execute([ "LPUSH"; "list"; "a"; "b" ]))
        Assert.Equal(RespInteger 1L, engine.Execute([ "ZADD"; "zset"; "1"; "alice" ]))
        Assert.Equal(RespInteger 1L, engine.Execute([ "PFADD"; "hll"; "alice"; "bob" ]))

    [<Fact>]
    member _.``supports ttl dbsize and flushdb``() =
        let engine = DataStoreEngine()
        Assert.Equal(RespSimpleString "OK", engine.Execute([ "SET"; "temp"; "1" ]))
        Assert.Equal(RespInteger 1L, engine.Execute([ "EXPIRE"; "temp"; "10" ]))

        match engine.Execute([ "TTL"; "temp" ]) with
        | RespInteger ttl -> Assert.True(ttl >= 0L)
        | other -> failwithf "unexpected TTL response: %A" other

        Assert.Equal(RespInteger 1L, engine.Execute([ "DBSIZE" ]))
        Assert.Equal(RespSimpleString "OK", engine.Execute([ "FLUSHDB" ]))
        Assert.Equal(RespInteger 0L, engine.Execute([ "DBSIZE" ]))

    [<Fact>]
    member _.``supports store helpers reset and errors``() =
        let engine = DataStoreEngine()
        let store = Store.Empty().Set("alpha", DataStoreTypes.stringEntry "1" None)

        engine.Reset(store)
        Assert.True(engine.Store.Exists "alpha")
        let command: DataStoreCommand = { Name = "ECHO"; Args = [ "hello" ] }
        Assert.Equal(RespBulkString(Some "hello"), engine.Execute(command))
        Assert.Equal(RespErrorValue "ERR unknown command 'NOPE'", engine.Execute([ "nope" ]))
        Assert.Equal(RespErrorValue "ERR empty command", engine.Execute([]))
        Assert.Equal(RespErrorValue "ERR wrong number of arguments for 'GET'", engine.Execute([ "GET"; "a"; "b" ]))

    [<Fact>]
    member _.``covers data structure helpers and validation branches``() =
        let sorted = SortedSet()
        Assert.True(sorted.Insert(1.0, "alice"))
        Assert.False(sorted.Insert(2.0, "alice"))
        Assert.Equal(1, sorted.Count)
        Assert.Equal(1, sorted.Clone().Count)

        let db =
            Database()
                .Set("expired", DataStoreTypes.stringEntry "gone" (Some (Database.CurrentTimeMs() - 1L)))
                .Set("live", DataStoreTypes.stringEntry "ok" None)

        Assert.Equal(None, db.Get "expired")
        Assert.Equal(1, db.DbSize())

        let store = Store.Empty().Set("temp", DataStoreTypes.stringEntry "1" None).Delete("temp").Select(99)
        Assert.False(store.Exists "temp")
        Assert.Equal(15, store.ActiveDb)

        let engine = DataStoreEngine()
        Assert.Equal(RespSimpleString "OK", engine.Execute([ "SET"; "name"; "abc" ]))
        Assert.Equal(RespErrorValue "ERR value is not an integer or out of range", engine.Execute([ "INCR"; "name" ]))
        Assert.Equal(RespErrorValue "ERR value is not an integer or out of range", engine.Execute([ "EXPIRE"; "missing"; "oops" ]))
