/**
 * forme-stage — denied capability-API wrapper tests
 *
 * The denied wrappers are the security boundary.  Each method must
 * throw `CapabilityError` with the right capability string embedded.
 */

import { describe, it, expect } from "vitest";
import { CapabilityError } from "@coding-adventures/forme-errors";
import {
  deniedEnvApi,
  deniedFilesystemApi,
  deniedNetworkApi,
  deniedShellApi,
  deniedStorageApi,
} from "../src/index.js";

describe("deniedStorageApi", () => {
  const api = deniedStorageApi();

  it("read denies with storage:read", async () => {
    await expect(api.read("foo")).rejects.toMatchObject({
      capability: "storage:read",
    });
  });

  it("write denies with storage:write", async () => {
    await expect(api.write("foo", new Uint8Array())).rejects.toMatchObject({
      capability: "storage:write",
    });
  });

  it("exists/list/watch/stat all deny with storage:read", async () => {
    await expect(api.exists("foo")).rejects.toMatchObject({ capability: "storage:read" });
    await expect(api.stat("foo")).rejects.toMatchObject({ capability: "storage:read" });

    const listIter = api.list("foo")[Symbol.asyncIterator]();
    await expect(listIter.next()).rejects.toMatchObject({ capability: "storage:read" });

    const watchIter = api.watch("foo")[Symbol.asyncIterator]();
    await expect(watchIter.next()).rejects.toMatchObject({ capability: "storage:read" });
  });

  it("remove denies with storage:write", async () => {
    await expect(api.remove("foo")).rejects.toMatchObject({
      capability: "storage:write",
    });
  });

  it("error message names the operation and the missing capability", async () => {
    try {
      await api.read("posts/draft.md");
    } catch (e) {
      const ce = e as CapabilityError;
      expect(ce).toBeInstanceOf(CapabilityError);
      expect(ce.message).toContain("storage:read");
      expect(ce.message).toContain("posts/draft.md");
    }
  });
});

describe("deniedNetworkApi", () => {
  const api = deniedNetworkApi();

  it("fetch denies with network:*", async () => {
    await expect(api.fetch("https://example.com")).rejects.toMatchObject({
      capability: "network:*",
    });
  });

  it("error message includes the requested URL", async () => {
    try { await api.fetch("https://example.com/api"); }
    catch (e) {
      expect((e as CapabilityError).message).toContain("https://example.com/api");
    }
  });

  it("accepts Request objects", async () => {
    await expect(api.fetch(new Request("https://example.com"))).rejects.toBeInstanceOf(CapabilityError);
  });
});

describe("deniedEnvApi", () => {
  const api = deniedEnvApi();

  it("get throws synchronously with env:<NAME>", () => {
    expect(() => api.get("GITHUB_TOKEN")).toThrow(CapabilityError);
    try { api.get("GITHUB_TOKEN"); }
    catch (e) {
      expect((e as CapabilityError).capability).toBe("env:GITHUB_TOKEN");
    }
  });

  it("getOrThrow also throws CapabilityError (not the missing-var error)", () => {
    expect(() => api.getOrThrow("ANY")).toThrow(CapabilityError);
  });
});

describe("deniedFilesystemApi", () => {
  const api = deniedFilesystemApi();

  it("readAbsolute denies with filesystem:user", async () => {
    await expect(api.readAbsolute("/etc/passwd")).rejects.toMatchObject({
      capability: "filesystem:user",
    });
  });

  it("writeAbsolute denies with filesystem:user", async () => {
    await expect(api.writeAbsolute("/tmp/x", new Uint8Array())).rejects.toMatchObject({
      capability: "filesystem:user",
    });
  });

  it("homeDir/tempDir throw synchronously with filesystem:user", () => {
    expect(() => api.homeDir()).toThrow(CapabilityError);
    expect(() => api.tempDir()).toThrow(CapabilityError);
  });
});

describe("deniedShellApi", () => {
  const api = deniedShellApi();

  it("run denies with system:shell", async () => {
    await expect(api.run("ls", ["-la"])).rejects.toMatchObject({
      capability: "system:shell",
    });
  });

  it("error message names the command and args", async () => {
    try { await api.run("rm", ["-rf", "/"]); }
    catch (e) {
      const msg = (e as CapabilityError).message;
      expect(msg).toContain("rm");
      expect(msg).toContain("rf");
      expect(msg).toContain("system:shell");
    }
  });
});
