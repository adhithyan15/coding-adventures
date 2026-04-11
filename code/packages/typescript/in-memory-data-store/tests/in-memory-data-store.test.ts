import { describe, expect, it } from "vitest";
import { array, bulkString, encode, simpleString } from "@coding-adventures/resp-protocol";
import { commandFromResp } from "@coding-adventures/in-memory-data-store-protocol";
import { createInMemoryDataStore } from "../src/index.js";

describe("in-memory-data-store", () => {
  it("executes RESP frames end to end", () => {
    const store = createInMemoryDataStore();
    const frame = array([bulkString("PING")]);

    expect(store.executeFrame(frame)).toEqual(simpleString("PONG"));
  });

  it("processes multiple RESP commands and encodes responses", () => {
    const store = createInMemoryDataStore();
    const first = encode(array([bulkString("SET"), bulkString("counter"), bulkString("1")]));
    const second = encode(array([bulkString("GET"), bulkString("counter")]));
    const input = new Uint8Array(first.length + second.length);
    input.set(first, 0);
    input.set(second, first.length);

    const responses = store.process(input);
    expect(responses).toEqual([simpleString("OK"), bulkString("1")]);
  });

  it("ignores blank RESP arrays", () => {
    const store = createInMemoryDataStore();
    expect(store.executeFrame(array([]))).toBeNull();
  });

  it("can translate commands before execution", () => {
    const command = commandFromResp(array([bulkString("PING")]));
    expect(command).toEqual({ name: "PING", args: [] });
  });
});
