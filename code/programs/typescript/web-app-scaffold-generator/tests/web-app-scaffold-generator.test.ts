import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  generateElectronWrapper,
  generateVisualizationApp,
  parseCommand,
  runCli,
} from "../src/index.js";

const tempDirs: string[] = [];
const originalCwd = process.cwd();

function makeRepoRoot(): string {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), "web-app-scaffold-generator-"));
  tempDirs.push(repoRoot);
  fs.mkdirSync(path.join(repoRoot, ".github", "workflows"), { recursive: true });
  fs.mkdirSync(path.join(repoRoot, "code", "programs", "typescript"), { recursive: true });
  return repoRoot;
}

afterEach(() => {
  process.chdir(originalCwd);
  for (const tempDir of tempDirs.splice(0)) {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

describe("parseCommand", () => {
  it("parses a visualization command", () => {
    const command = parseCommand([
      "code39-visualizer",
      "--template",
      "visualization",
      "--pages-slug",
      "code39",
      "--package-deps",
      "code39,draw-instructions-svg",
    ]);

    expect(command.template).toBe("visualization");
    expect(command.pagesSlug).toBe("code39");
    expect(command.packageDeps).toEqual(["code39", "draw-instructions-svg"]);
  });

  it("allows electron-wrapper parsing before renderer validation", () => {
    const command = parseCommand(["code39-desktop", "--template", "electron-wrapper"]);

    expect(command.template).toBe("electron-wrapper");
    expect(command.rendererApp).toBeUndefined();
  });

  it("rejects missing option values", () => {
    expect(() => parseCommand(["code39-visualizer", "--template"])).toThrowError(
      "missing value for --template",
    );
  });

  it("rejects unknown templates", () => {
    expect(() => parseCommand(["code39-visualizer", "--template", "storybook"])).toThrowError(
      'unknown template "storybook"',
    );
  });
});

describe("generateVisualizationApp", () => {
  it("creates a renderer app and deploy workflow", async () => {
    const repoRoot = makeRepoRoot();

    await generateVisualizationApp({
      repoRoot,
      name: "code39-visualizer",
      description: "Interactive Code 39 barcode visualizer",
      pagesSlug: "code39",
      packageDeps: ["code39", "draw-instructions-svg"],
    });

    const appRoot = path.join(repoRoot, "code", "programs", "typescript", "code39-visualizer");
    const packageJson = fs.readFileSync(path.join(appRoot, "package.json"), "utf8");
    const build = fs.readFileSync(path.join(appRoot, "BUILD"), "utf8");
    const main = fs.readFileSync(path.join(appRoot, "src", "main.tsx"), "utf8");
    const lattice = fs.readFileSync(path.join(appRoot, "src", "styles", "app.lattice"), "utf8");
    const viteConfig = fs.readFileSync(path.join(appRoot, "vite.config.ts"), "utf8");
    const workflow = fs.readFileSync(path.join(repoRoot, ".github", "workflows", "deploy-code39.yml"), "utf8");

    expect(packageJson).toContain(`"@coding-adventures/code39": "file:../../../packages/typescript/code39"`);
    expect(packageJson).toContain(`"@coding-adventures/draw-instructions-svg": "file:../../../packages/typescript/draw-instructions-svg"`);
    expect(packageJson).toContain(`"@coding-adventures/lattice-transpiler": "file:../../../packages/typescript/lattice-transpiler"`);
    expect(main).toContain(`installLatticeStyles()`);
    expect(lattice).toContain(`@mixin panel-surface`);
    expect(build).toContain("cd ../../../packages/typescript/lattice-transpiler && npm install");
    expect(build).toContain("cd ../../../packages/typescript/code39 && npm install");
    expect(viteConfig).toContain('base: "/coding-adventures/code39/"');
    expect(workflow).toContain('destination_dir: code39');
    expect(workflow).toContain('code/programs/typescript/code39-visualizer/**');
    expect(workflow).toContain('code/packages/typescript/lattice-transpiler/**');
  });
});

describe("generateElectronWrapper", () => {
  it("creates an Electron wrapper and release workflow", async () => {
    const repoRoot = makeRepoRoot();

    await generateElectronWrapper({
      repoRoot,
      name: "code39-desktop",
      description: "Electron wrapper for Code 39",
      rendererApp: "code39-visualizer",
      rendererPackageDeps: ["code39", "draw-instructions-svg"],
      productName: "Code 39",
      appId: "com.codingadventures.code39",
      tagPrefix: "code39-desktop",
    });

    const appRoot = path.join(repoRoot, "code", "programs", "typescript", "code39-desktop");
    const packageJson = fs.readFileSync(path.join(appRoot, "package.json"), "utf8");
    const electronMain = fs.readFileSync(path.join(appRoot, "electron", "main.ts"), "utf8");
    const workflow = fs.readFileSync(path.join(repoRoot, ".github", "workflows", "release-code39-desktop.yml"), "utf8");

    expect(packageJson).toContain(`"main": "dist-electron/main.js"`);
    expect(electronMain).toContain(`window.loadFile(path.join(__dirname, "../renderer/index.html"));`);
    expect(workflow).toContain(`code/programs/typescript/code39-visualizer`);
    expect(workflow).toContain(`code/programs/typescript/code39-desktop/release/*.exe`);
    expect(workflow).toContain(`tags:\n      - "code39-desktop-v*"`);
  });
});

describe("runCli", () => {
  it("creates a visualization app from the CLI in the repo root", async () => {
    const repoRoot = makeRepoRoot();
    process.chdir(repoRoot);

    const originalLog = console.log;
    const logs: string[] = [];
    console.log = (message?: unknown) => {
      logs.push(String(message));
    };

    try {
      await runCli([
        "code39-visualizer",
        "--template",
        "visualization",
        "--description",
        "Interactive Code 39 barcode visualizer",
        "--pages-slug",
        "code39",
        "--package-deps",
        "code39,draw-instructions-svg",
      ]);
    } finally {
      console.log = originalLog;
    }

    const appRoot = path.join(repoRoot, "code", "programs", "typescript", "code39-visualizer");
    expect(fs.existsSync(path.join(appRoot, "package.json"))).toBe(true);
    expect(logs).toHaveLength(1);
    expect(logs[0]).toContain("Created visualization app at ");
    expect(logs[0]).toContain("code/programs/typescript/code39-visualizer");
  });

  it("requires renderer-app for electron wrappers", async () => {
    const repoRoot = makeRepoRoot();
    process.chdir(repoRoot);

    await expect(runCli(["code39-desktop", "--template", "electron-wrapper"])).rejects.toThrowError(
      "--renderer-app is required for --template electron-wrapper",
    );
  });

  it("creates an electron wrapper from the CLI", async () => {
    const repoRoot = makeRepoRoot();
    process.chdir(repoRoot);

    const originalLog = console.log;
    const logs: string[] = [];
    console.log = (message?: unknown) => {
      logs.push(String(message));
    };

    try {
      await runCli([
        "code39-desktop",
        "--template",
        "electron-wrapper",
        "--renderer-app",
        "code39-visualizer",
        "--renderer-package-deps",
        "code39,draw-instructions-svg",
        "--product-name",
        "Code 39 Desktop",
        "--app-id",
        "com.codingadventures.code39desktop",
        "--tag-prefix",
        "code39-desktop",
      ]);
    } finally {
      console.log = originalLog;
    }

    const appRoot = path.join(repoRoot, "code", "programs", "typescript", "code39-desktop");
    expect(fs.existsSync(path.join(appRoot, "electron", "main.ts"))).toBe(true);
    expect(logs).toHaveLength(1);
    expect(logs[0]).toContain("Created electron-wrapper app at ");
    expect(logs[0]).toContain("code/programs/typescript/code39-desktop");
  });
});
