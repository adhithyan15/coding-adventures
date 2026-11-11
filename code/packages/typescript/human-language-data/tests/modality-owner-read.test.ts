import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";
import { afterEach, expect, it, vi } from "vitest";
import { readAnchoredModalityTrack } from "../src/modality-owner-read.js";

vi.mock("node:child_process", () => ({ spawn: vi.fn() }));

afterEach(() => {
  vi.useRealTimers();
  vi.clearAllMocks();
});

function fakeChild() {
  const child = Object.assign(new EventEmitter(), {
    stdin: new PassThrough(),
    stdout: new PassThrough(),
    stderr: new PassThrough(),
    kill: vi.fn(() => true),
  });
  vi.mocked(spawn).mockReturnValue(child as unknown as ChildProcessWithoutNullStreams);
  return child;
}

it("kills a silent anchored reader at its deadline and waits for close", async () => {
  vi.useFakeTimers();
  const child = fakeChild();
  const pending = readAnchoredModalityTrack("C:/curriculum", "spanish", ["_meta.json"]);
  let settled = false;
  void pending.then(() => { settled = true; }, () => { settled = true; });
  try {
    await vi.advanceTimersByTimeAsync(20_000);
    expect(child.kill).toHaveBeenCalledWith("SIGKILL");
    expect(settled).toBe(false);
  } finally {
    child.emit("close", null);
  }
  await expect(pending).rejects.toThrow(/timed out/);
});

it("clears the deadline after a complete exact response", async () => {
  vi.useFakeTimers();
  const child = fakeChild();
  const pending = readAnchoredModalityTrack("C:/curriculum", "spanish", ["_meta.json"]);
  child.stdout.emit("data", Buffer.from('{"files":{"_meta.json":"¡hola!"}}'));
  child.emit("close", 0);
  await expect(pending).resolves.toEqual(new Map([["_meta.json", "¡hola!"]]));
  await vi.advanceTimersByTimeAsync(20_000);
  expect(child.kill).not.toHaveBeenCalled();
});

it("does not settle a child error before its streams close", async () => {
  const child = fakeChild();
  const pending = readAnchoredModalityTrack("C:/curriculum", "spanish", ["_meta.json"]);
  let settled = false;
  void pending.then(() => { settled = true; }, () => { settled = true; });
  child.emit("error", new Error("private child detail"));
  for (let turn = 0; turn < 3; turn += 1) await Promise.resolve();
  expect(settled).toBe(false);
  child.emit("close", null);
  await expect(pending).rejects.toThrow("modality anchored reader could not start");
});

it("kills an oversized response and waits for close before rejecting", async () => {
  const child = fakeChild();
  const pending = readAnchoredModalityTrack("C:/curriculum", "spanish", ["_meta.json"]);
  let settled = false;
  void pending.then(() => { settled = true; }, () => { settled = true; });
  child.stdout.emit("data", Buffer.alloc(64 * 1024 * 1024 + 1));
  expect(child.kill).toHaveBeenCalledWith("SIGKILL");
  expect(settled).toBe(false);
  child.emit("close", null);
  await expect(pending).rejects.toThrow(/failed/);
});
