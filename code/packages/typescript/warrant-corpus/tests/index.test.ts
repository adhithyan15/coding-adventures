import { describe, expect, it } from "vitest";
import * as api from "../src/index.ts";

describe("the public surface", () => {
  it("exports the runner's entry points and error classes", () => {
    for (const name of ["main", "loadCorpus", "parseBinding", "resolveExports", "Subject", "SubjectFailure", "buildManifest", "frameSources", "sourceDigest", "validateManifest", "execute", "prepare", "outcomePassed"]) {
      expect(typeof (api as Record<string, unknown>)[name], name).toBe("function");
    }
    expect(new api.CorpusError("c", "m")).toBeInstanceOf(api.LaneError);
    expect(new api.BindingError("c", "m")).toBeInstanceOf(api.LaneError);
    expect(new api.UsageError("c", "m").code).toBe("c");
    expect(api.VERSION).toBe("0.1.0");
  });
});
