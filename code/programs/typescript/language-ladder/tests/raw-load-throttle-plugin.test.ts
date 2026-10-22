import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  RAW_LOAD_CONCURRENCY,
  rawLoadThrottlePlugin,
  type RawLoadThrottleOptions,
} from "../raw-load-throttle-plugin.ts";

// The plugin's `load` is a plain async function; call it the way Rolldown does.
type LoadResult = { code: string; moduleType?: string } | null;
function loader(options: RawLoadThrottleOptions): (id: string) => Promise<LoadResult> {
  const plugin = rawLoadThrottlePlugin(options);
  const load = plugin.load as (this: unknown, id: string) => Promise<LoadResult>;
  return (id) => load.call({}, id);
}

const root = join("/repo", "code", "learning", "human-languages");
const lesson = (name: string) => join(root, "italian", "lessons", `${name}.md`);

describe("rawLoadThrottlePlugin", () => {
  it("is a build-only pre plugin", () => {
    const plugin = rawLoadThrottlePlugin({ curriculumRoot: root });
    expect(plugin.apply).toBe("build");
    expect(plugin.enforce).toBe("pre");
  });

  it("answers a lesson ?raw import the way vite:asset does", async () => {
    const load = loader({ curriculumRoot: root, readText: async () => 'Ciao "mondo"\n' });
    const result = await load(`${lesson("IT-C277-istante")}?raw`);
    expect(result?.code).toBe(`export default ${JSON.stringify('Ciao "mondo"\n')}`);
    expect(result?.moduleType).toBe("js");
  });

  it("leaves every other id to Vite", async () => {
    const reads: string[] = [];
    const load = loader({ curriculumRoot: root, readText: async (path) => (reads.push(path), "") });
    expect(await load(lesson("IT-C277-istante"))).toBeNull();
    expect(await load(`${join(root, "data", "scripts", "latin.json")}?raw`)).toBeNull();
    expect(await load(`${join("/elsewhere", "README.md")}?raw`)).toBeNull();
    expect(await load(`${join(root, "..", "README.md")}?raw`)).toBeNull();
    expect(await load("\0virtual:thing.md?raw")).toBeNull();
    expect(await load(`${lesson("IT-C277-istante")}?rawish`)).toBeNull();
    expect(reads).toEqual([]);
  });

  it("never has more than `limit` reads in flight and keeps every result", async () => {
    const limit = 3;
    let inFlight = 0;
    let peak = 0;
    const load = loader({
      curriculumRoot: root,
      limit,
      readText: async (path) => {
        inFlight += 1;
        peak = Math.max(peak, inFlight);
        await new Promise((resolve) => setTimeout(resolve, 1));
        inFlight -= 1;
        return path;
      },
    });

    const ids = Array.from({ length: 40 }, (_, i) => `${lesson(`IT-C${i}`)}?raw`);
    const results = await Promise.all(ids.map(load));

    expect(peak).toBe(limit);
    expect(results.map((result) => result?.code)).toEqual(
      ids.map((id) => `export default ${JSON.stringify(id.slice(0, -"?raw".length))}`),
    );
  });

  it("frees a slot when a read fails", async () => {
    const load = loader({
      curriculumRoot: root,
      limit: 1,
      readText: async (path) => {
        if (path.endsWith("bad.md")) throw new Error("boom");
        return "ok";
      },
    });
    await expect(load(`${lesson("bad")}?raw`)).rejects.toThrow("boom");
    expect((await load(`${lesson("good")}?raw`))?.code).toBe('export default "ok"');
  });

  it("rejects a limit that is not a positive integer", () => {
    expect(RAW_LOAD_CONCURRENCY).toBe(64);
    for (const limit of [0, -1, 1.5, Number.NaN]) {
      expect(() => rawLoadThrottlePlugin({ curriculumRoot: root, limit })).toThrow(RangeError);
    }
  });
});
