// ---------------------------------------------------------------------------
// local-deps.mjs — make `npm ci` here enough to actually run the tests
// ---------------------------------------------------------------------------
//
// The bug this exists to kill
// ---------------------------
// `tests/figure.test.ts` and `tests/figure-cli.test.ts` failed on every fresh
// local checkout with:
//
//     Cannot find package '@coding-adventures/paint-vm'
//       imported from .../packages/typescript/paint-vm-svg/src/index.ts
//
// The package is not missing. It is right there, one directory over. The
// problem is where Node looks for it.
//
// This package depends on siblings by path — `"@coding-adventures/paint-vm-svg":
// "file:../paint-vm-svg"` — and npm materialises a `file:` dependency as a
// SYMLINK: `node_modules/@coding-adventures/paint-vm-svg -> ../../paint-vm-svg`.
// Those siblings ship TypeScript source (`"main": "src/index.ts"`), so when the
// test imports `paint-vm-svg`, the file that actually gets loaded lives at its
// REAL path, outside this package. Module resolution then walks up from THERE:
//
//     packages/typescript/paint-vm-svg/node_modules   <- looked in, empty
//     packages/typescript/node_modules                <- looked in, absent
//     packages/node_modules, code/node_modules, ...   <- absent
//     packages/typescript/human-language-data/...     <- NEVER looked in
//
// So `paint-vm-svg` can only find its own dependencies in its own
// `node_modules`, and `npm ci` in this directory never creates one there.
// Every sibling in the chain needs its own install, leaf first. That is the
// repository's single most-recurring failure (see `lessons.md`: "BUILD files
// must install ALL transitive local deps in leaf-to-root order"), and the BUILD
// file here has always done it — which is why CI was green while every local
// run of the figure suite was red.
//
// Why a postinstall and not just the BUILD line
// ----------------------------------------------
// The BUILD file is run by the repository's build tool. A person — or an agent
// — working on figures runs `npm ci && npm run build && npx vitest run`, and
// nothing in that sequence reads BUILD. Developing a figure feature against a
// suite that cannot execute is how a broken generator ships, so the chain
// install has to happen on `npm install`/`npm ci` itself. This script is the
// same leaf-to-root walk the BUILD does, computed from the manifests instead of
// hand-listed, so adding a `file:` dependency does not silently un-fix it.
//
// It is deliberately conservative. It installs a sibling only when that sibling
// declares local dependencies of its own AND they are not already linked, so a
// warm tree costs a handful of `stat` calls and CI's pre-installed chain is
// left untouched. `HUMAN_LANGUAGE_DATA_LOCAL_DEPS=skip` opts out entirely, for
// an offline or vendored install.
// ---------------------------------------------------------------------------

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
/** npm sets this while running a lifecycle script; it stops nested recursion. */
const GUARD = "HUMAN_LANGUAGE_DATA_LOCAL_DEPS";

/** A package's `file:`-linked dependencies, as absolute directories. */
function localDependencies(packageDir) {
  const manifestPath = join(packageDir, "package.json");
  if (!existsSync(manifestPath)) return [];
  let manifest;
  try {
    manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  } catch {
    return [];
  }
  const declared = {
    ...(manifest.dependencies ?? {}),
    ...(manifest.devDependencies ?? {}),
  };
  const out = [];
  for (const [name, specifier] of Object.entries(declared)) {
    if (typeof specifier !== "string" || !specifier.startsWith("file:")) continue;
    out.push({ name, dir: resolve(packageDir, specifier.slice("file:".length)) });
  }
  return out;
}

/**
 * The whole `file:` closure below `root`, leaf first.
 *
 * A post-order walk is the install order: a package is emitted only after
 * everything it depends on, which is exactly what "leaf-to-root" means. The
 * `visiting` set makes a dependency cycle a no-op rather than a hang — this
 * script is not the place to diagnose one.
 */
function closureLeafFirst(root) {
  const ordered = [];
  const done = new Set();
  const visiting = new Set();
  const walk = (dir) => {
    if (done.has(dir) || visiting.has(dir)) return;
    visiting.add(dir);
    for (const dependency of localDependencies(dir)) walk(dependency.dir);
    visiting.delete(dir);
    done.add(dir);
    ordered.push(dir);
  };
  walk(root);
  return ordered.filter((dir) => dir !== root);
}

/** Has this package already got every local dependency linked into place? */
function alreadyLinked(packageDir) {
  return localDependencies(packageDir).every(({ name }) =>
    existsSync(join(packageDir, "node_modules", name)),
  );
}

function install(packageDir) {
  const command = existsSync(join(packageDir, "package-lock.json")) ? "ci" : "install";
  execFileSync(process.platform === "win32" ? "npm.cmd" : "npm", [command, "--silent"], {
    cwd: packageDir,
    stdio: "inherit",
    env: { ...process.env, [GUARD]: "running" },
  });
}

function main() {
  if (process.env[GUARD] === "skip" || process.env[GUARD] === "running") return;
  for (const packageDir of closureLeafFirst(HERE)) {
    if (alreadyLinked(packageDir)) continue;
    process.stdout.write(`local-deps: installing ${packageDir}\n`);
    install(packageDir);
  }
}

main();
