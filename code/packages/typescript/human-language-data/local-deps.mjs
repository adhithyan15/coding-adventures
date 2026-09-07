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
import { existsSync, readFileSync, realpathSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));

/**
 * The only directory tree this script is allowed to run `npm` in.
 *
 * A `file:` specifier is just a path, and `resolve` will happily follow
 * `file:../../../../../tmp/x` or `file:/anywhere`. `npm ci` then runs THAT
 * directory's `preinstall`/`install`/`postinstall` scripts, so an unbounded
 * walk turns "one line changed in a manifest" into "code of somebody's choosing
 * runs at install time, from a directory an unprivileged local user can
 * create". A hostile specifier in a repository manifest is already install-time
 * execution on its own, so this is a containment boundary rather than the last
 * line of defence — but it is the difference between a change that looks
 * obviously wrong in review and one that does not.
 *
 * It doubles as the check that this script is running inside the repository at
 * all: if this package is ever installed as a published tarball, its siblings
 * are not here, the closure is empty, and nothing is executed.
 */
const PACKAGES_ROOT = resolve(HERE, "..");

/** npm sets this while running a lifecycle script; it stops nested recursion. */
const GUARD = "HUMAN_LANGUAGE_DATA_LOCAL_DEPS";

/**
 * Is `dir` really inside the sibling-packages root, and not the root itself?
 *
 * REAL paths on both sides, because a lexical comparison is not containment: a
 * directory that is a symlink to somewhere else passes `relative()` while the
 * operating system happily resolves it to the target, and `npm ci` would then
 * run — along with that target's lifecycle scripts — outside the repository.
 * `realpathSync` throws for a path that does not exist, which is the same
 * answer: not a sibling package, do not install it.
 */
function inPackagesRoot(dir) {
  let real;
  try {
    real = realpathSync(dir);
  } catch {
    return false;
  }
  const inside = relative(realpathSync(PACKAGES_ROOT), real);
  return inside !== "" && !inside.startsWith("..") && !isAbsolute(inside);
}

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
    const dir = resolve(packageDir, specifier.slice("file:".length));
    if (!inPackagesRoot(dir)) {
      throw new Error(
        `local-deps: '${name}' resolves to ${dir}, outside ${PACKAGES_ROOT}. ` +
          `This script only installs sibling packages; fix the 'file:' path.`,
      );
    }
    out.push({ name, dir });
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

/**
 * Return the executable and fixed argv for one reproducible npm install.
 *
 * Windows cannot execute the `npm.cmd` shim through `execFileSync` without a
 * shell (`spawnSync npm.cmd EINVAL`). npm exposes the JavaScript CLI that is
 * running the lifecycle as `npm_execpath`, so invoking that file with the
 * current Node executable keeps the shell out of the boundary entirely.
 */
export function npmCiInvocation({
  platform = process.platform,
  nodeExecutable = process.execPath,
  npmExecutable = process.env.npm_execpath,
} = {}) {
  if (platform !== "win32") {
    return { executable: "npm", args: ["ci", "--silent"] };
  }
  if (!npmExecutable) {
    throw new Error(
      "local-deps: npm_execpath is required to run npm without a shell on Windows.",
    );
  }
  return {
    executable: nodeExecutable,
    args: [npmExecutable, "ci", "--silent"],
  };
}

function install(packageDir) {
  // `npm install` here would resolve versions fresh from the registry during
  // what the caller invoked as `npm ci`, silently dropping the pinning `npm ci`
  // exists to provide. A sibling without a lockfile is a repository problem,
  // and saying so is better than quietly installing something else.
  if (!existsSync(join(packageDir, "package-lock.json"))) {
    throw new Error(
      `local-deps: ${packageDir} has no package-lock.json, so its install ` +
        `could not be reproducible. Commit one there.`,
    );
  }
  // `execFileSync`, not a shell: the argv is fixed and nothing here is
  // interpolated into a command line, so there is no argument-injection surface.
  const invocation = npmCiInvocation();
  execFileSync(invocation.executable, invocation.args, {
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

const entrypoint = process.argv[1];
if (
  entrypoint &&
  realpathSync(entrypoint) === realpathSync(fileURLToPath(import.meta.url))
) {
  main();
}
