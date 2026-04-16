namespace CodingAdventures.InMemoryDataStoreEngine.FSharp.Tests

open CodingAdventures.InMemoryDataStoreEngine.FSharp
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
