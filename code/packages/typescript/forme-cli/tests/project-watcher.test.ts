import { describe, expect, it } from "vitest";
import { resolve } from "node:path";
import {
  watchProject,
  type FsWatcherLike,
  type FsWatchFactory,
} from "../src/project-watcher.js";

class FakeWatcher implements FsWatcherLike {
  closed = false;
  errorListener: ((error: Error) => void) | null = null;
  close(): void { this.closed = true; }
  on(_event: "error", listener: (error: Error) => void): FsWatcherLike {
    this.errorListener = listener;
    return this;
  }
}

function harness(): {
  factory: FsWatchFactory;
  watcher: FakeWatcher;
  emit(event: "change" | "rename", path: string | null): void;
} {
  const watcher = new FakeWatcher();
  let listener: Parameters<FsWatchFactory>[2] = () => {};
  return {
    watcher,
    factory: (_root, options, next) => {
      expect(options).toEqual({ recursive: true });
      listener = next;
      return watcher;
    },
    emit: (event, path) => listener(event, path),
  };
}

describe("project watcher", () => {
  it("streams project changes while filtering generated and dependency trees", async () => {
    const root = "/project";
    const fake = harness();
    const stream = watchProject(root, ["dist", "node_modules", ".git"], fake.factory);
    const iterator = stream[Symbol.asyncIterator]();

    fake.emit("change", "dist/index.html");
    fake.emit("rename", "node_modules/pkg/index.js");
    fake.emit("change", ".git/index");
    const next = iterator.next();
    fake.emit("change", "data/post.md");
    expect(await next).toEqual({
      done: false,
      value: { eventType: "change", path: resolve(root, "data/post.md") },
    });

    await iterator.return?.();
    expect(fake.watcher.closed).toBe(true);
    expect((await iterator.next()).done).toBe(true);
  });

  it("forwards filename-less rebuild signals and watcher errors", async () => {
    const fake = harness();
    const iterator = watchProject("/project", [], fake.factory)[Symbol.asyncIterator]();
    const filenameLess = iterator.next();
    fake.emit("rename", null);
    expect((await filenameLess).value).toEqual({ eventType: "rename", path: null });

    const failure = iterator.next();
    fake.watcher.errorListener?.(new Error("watch failed"));
    await expect(failure).rejects.toThrow("watch failed");
  });

  it("rejects ignore roots outside the project", () => {
    expect(() => watchProject("/project", ["../outside"], harness().factory)).toThrow(/outside/);
  });
});
