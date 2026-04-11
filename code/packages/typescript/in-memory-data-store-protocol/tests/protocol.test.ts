import { describe, expect, it } from "vitest";
import { array, bulkString, simpleString } from "@coding-adventures/resp-protocol";
import {
  commandFromResp,
  commandToResp,
  commandName,
  respValueToString,
} from "../src/index.js";

describe("in-memory-data-store-protocol", () => {
  it("parses RESP command arrays", () => {
    const command = commandFromResp(
      array([bulkString("SET"), bulkString("counter"), bulkString("1")]),
    );
    expect(command).toEqual({ name: "SET", args: ["counter", "1"] });
  });

  it("rejects empty or unsupported frames", () => {
    expect(commandFromResp(array([]))).toBeNull();
    expect(commandFromResp(simpleString("OK"))).toBeNull();
  });

  it("round-trips commands back to RESP", () => {
    const command = { name: "PING", args: [] };
    expect(commandToResp(command)).toEqual(array([bulkString("PING")]));
  });

  it("normalizes command names", () => {
    expect(commandName(["  ping  "])).toBe("PING");
  });

  it("converts RESP values to strings when possible", () => {
    expect(respValueToString(bulkString("hello"))).toBe("hello");
  });
});
