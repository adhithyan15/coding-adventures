import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { bootstrap, localInstallOrder, runCommand } from "../bin/bootstrap.mjs";

const roots = [];

afterEach(async () => {
  for (const root of roots.splice(0)) await rm(root, { recursive: true, force: true });
});

async function packageAt(root, name, manifest = {}) {
  const directory = path.join(root, name);
  await mkdir(directory, { recursive: true });
  await writeFile(path.join(directory, "package.json"), JSON.stringify({ name, ...manifest }));
  return directory;
}

describe("local dependency bootstrap", () => {
  it("orders transitive file dependencies leaf-first and deterministically", async () => {
    const root = await mkdtemp(path.join(tmpdir(), "forme-cli-bootstrap-"));
    roots.push(root);
    const leaf = await packageAt(root, "leaf");
    const middle = await packageAt(root, "middle", { dependencies: { leaf: "file:../leaf" } });
    const project = await packageAt(root, "project", {
      dependencies: { middle: "file:../middle" },
      devDependencies: { leaf: "file:../leaf" },
    });

    expect(await localInstallOrder(project)).toEqual([leaf, middle, project]);
  });

  it("reports local dependency cycles before running npm", async () => {
    const root = await mkdtemp(path.join(tmpdir(), "forme-cli-bootstrap-cycle-"));
    roots.push(root);
    const left = await packageAt(root, "left", { dependencies: { right: "file:../right" } });
    await packageAt(root, "right", { dependencies: { left: "file:../left" } });
    await expect(localInstallOrder(left)).rejects.toThrow(/dependency cycle/);
  });

  it("ignores registry dependencies", async () => {
    const root = await mkdtemp(path.join(tmpdir(), "forme-cli-bootstrap-registry-"));
    roots.push(root);
    const project = await packageAt(root, "project", {
      dependencies: { typescript: "^5.0.0" },
    });
    expect(await localInstallOrder(project)).toEqual([project]);
  });

  it("installs the computed order with injectable process and log adapters", async () => {
    const root = await mkdtemp(path.join(tmpdir(), "forme-cli-bootstrap-run-"));
    roots.push(root);
    const leaf = await packageAt(root, "leaf");
    const project = await packageAt(root, "project", {
      dependencies: { leaf: "file:../leaf" },
    });
    const calls = [];
    const logs = [];
    await bootstrap(project, {
      npmCommand: "npm-test",
      log: line => logs.push(line),
      install: async (command, args, cwd) => { calls.push({ command, args, cwd }); },
    });
    expect(calls.map(call => call.cwd)).toEqual([leaf, project]);
    expect(calls.every(call => call.command === "npm-test")).toBe(true);
    expect(calls[0].args).toEqual(["install", "--silent", "--package-lock=false"]);
    expect(logs).toEqual(["[bootstrap] leaf", "[bootstrap] project"]);
  });

  it("reports child-process success, exit failure, and spawn failure", async () => {
    const root = await mkdtemp(path.join(tmpdir(), "forme-cli-bootstrap-child-"));
    roots.push(root);
    await expect(runCommand(process.execPath, ["-e", "process.exit(0)"], root)).resolves.toBeUndefined();
    await expect(runCommand(process.execPath, ["-e", "process.exit(7)"], root)).rejects.toThrow(/status 7/);
    await expect(runCommand(path.join(root, "missing-command"), [], root)).rejects.toThrow();
  });
});
