import { mkdir, mkdtemp, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import {
  boundedError,
  chromeFlagsFor,
  readBoundedUtf8,
} from "./release-web-quality.js";

describe("Forme release web-quality effects", () => {
  it("allows browser traffic to bypass the proxy only for the exact artifact origin", () => {
    const flags = chromeFlagsFor("http://127.0.0.1:4321/coding-adventures/blog/");
    expect(flags).toContain("--proxy-server=http://127.0.0.1:4321");
    expect(flags).toContain("--proxy-bypass-list=<-loopback>;127.0.0.1:4321");
    expect(flags).not.toContain("--proxy-bypass-list=127.0.0.1");
  });

  it("redacts a trapping thrown proxy without invoking it", () => {
    let trapped = false;
    const hostile = new Proxy({}, {
      get() {
        trapped = true;
        throw new Error("trap ran");
      },
      getPrototypeOf() {
        trapped = true;
        throw new Error("trap ran");
      },
    });
    expect(boundedError(hostile)).toBe("uninspectable thrown value");
    expect(trapped).toBe(false);
  });

  it("reads only bounded regular single-link files without following symlinks", async () => {
    const root = await mkdtemp(resolve(tmpdir(), "forme-web-quality-read-test-"));
    try {
      const generated = resolve(root, "generated");
      await mkdir(generated);
      const document = resolve(generated, "index.html");
      await writeFile(document, "safe html", "utf8");
      expect(await readBoundedUtf8(document, generated, 32)).toBe("safe html");

      const alias = resolve(generated, "alias.html");
      await symlink(document, alias);
      await expect(readBoundedUtf8(alias, generated, 32)).rejects.toThrow();
      await expect(readBoundedUtf8(document, generated, 4)).rejects.toThrow(/no larger/);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
});
