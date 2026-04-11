import { describe, expect, it } from "vitest";
import { array, bulkString, simpleString } from "@coding-adventures/resp-protocol";
import {
  DataStoreEngine,
  Store,
  stringEntry,
} from "../src/index.js";

describe("in-memory-data-store-engine", () => {
  it("executes string commands", () => {
    const engine = new DataStoreEngine();
    expect(engine.execute(["PING"])).toEqual(simpleString("PONG"));
    expect(engine.execute(["SET", "counter", "1"])).toEqual(simpleString("OK"));
    expect(engine.execute(["GET", "counter"])).toEqual(bulkString("1"));
    expect(engine.execute(["INCR", "counter"])).toEqual({
      kind: "integer",
      value: 2,
    });
  });

  it("handles hashes, sets, lists, sorted sets, and HLLs", () => {
    const engine = new DataStoreEngine();

    expect(engine.execute(["HSET", "hash", "field", "value"])).toEqual({
      kind: "integer",
      value: 1,
    });
    expect(engine.execute(["SADD", "set", "a", "b"])).toEqual({
      kind: "integer",
      value: 2,
    });
    expect(engine.execute(["LPUSH", "list", "a", "b"])).toEqual({
      kind: "integer",
      value: 2,
    });
    expect(engine.execute(["ZADD", "zset", "1", "alice", "2", "bob"])).toEqual({
      kind: "integer",
      value: 2,
    });
    expect(engine.execute(["PFADD", "hll", "alice", "bob"])).toEqual({
      kind: "integer",
      value: 1,
    });
  });

  it("supports keyspace and ttl commands", () => {
    const engine = new DataStoreEngine();
    engine.execute(["SET", "temp", "1"]);
    expect(engine.execute(["EXPIRE", "temp", "10"])).toEqual({
      kind: "integer",
      value: 1,
    });
    expect(engine.execute(["TTL", "temp"]).kind).toBe("integer");
    expect(engine.execute(["DBSIZE"])).toEqual({
      kind: "integer",
      value: 1,
    });
    expect(engine.execute(["FLUSHDB"])).toEqual(simpleString("OK"));
  });

  it("exposes the current store snapshot", () => {
    const engine = new DataStoreEngine(Store.empty());
    engine.reset(Store.empty().set("alpha", stringEntry("1")));
    expect(engine.store.exists("alpha")).toBe(true);
  });

  it("returns errors for unknown commands", () => {
    const engine = new DataStoreEngine();
    expect(engine.execute(["NOPE"])).toEqual({
      kind: "error",
      value: "ERR unknown command 'NOPE'",
    });
  });

  it("keeps command arrays usable for RESP integration", () => {
    const engine = new DataStoreEngine();
    expect(engine.execute(["ECHO", "hello"])).toEqual(bulkString("hello"));
  });
});
