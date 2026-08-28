import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { localInstallOrder } from "./bootstrap.mjs";

async function writeManifest(directory, manifest) {
  await mkdir(directory, { recursive: true });
  await writeFile(
    path.join(directory, "package.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
}

test("localInstallOrder installs transitive dependencies before consumers", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "forme-landing-bootstrap-"));

  try {
    const app = path.join(root, "app");
    const parser = path.join(root, "packages", "parser");
    const ast = path.join(root, "packages", "ast");

    await writeManifest(ast, { name: "ast" });
    await writeManifest(parser, {
      name: "parser",
      dependencies: { ast: "file:../ast" },
    });
    await writeManifest(app, {
      name: "app",
      dependencies: { parser: "file:../packages/parser" },
    });

    assert.deepEqual(await localInstallOrder(app), [ast, parser, app]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("localInstallOrder deduplicates shared local dependencies", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "forme-landing-bootstrap-"));

  try {
    const app = path.join(root, "app");
    const left = path.join(root, "packages", "left");
    const right = path.join(root, "packages", "right");
    const shared = path.join(root, "packages", "shared");

    await writeManifest(shared, { name: "shared" });
    await writeManifest(left, {
      name: "left",
      dependencies: { shared: "file:../shared" },
    });
    await writeManifest(right, {
      name: "right",
      dependencies: { shared: "file:../shared" },
    });
    await writeManifest(app, {
      name: "app",
      dependencies: {
        left: "file:../packages/left",
        right: "file:../packages/right",
      },
    });

    const order = await localInstallOrder(app);
    assert.equal(order.filter((directory) => directory === shared).length, 1);
    assert.ok(order.indexOf(shared) < order.indexOf(left));
    assert.ok(order.indexOf(shared) < order.indexOf(right));
    assert.equal(order.at(-1), app);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("localInstallOrder reports local dependency cycles", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "forme-landing-bootstrap-"));

  try {
    const first = path.join(root, "first");
    const second = path.join(root, "second");
    await writeManifest(first, {
      name: "first",
      dependencies: { second: "file:../second" },
    });
    await writeManifest(second, {
      name: "second",
      dependencies: { first: "file:../first" },
    });

    await assert.rejects(localInstallOrder(first), /dependency cycle/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
