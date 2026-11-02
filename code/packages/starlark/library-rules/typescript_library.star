# ============================================================================
# typescript_library.star — Build rule for TypeScript library packages
# ============================================================================
#
# TypeScript packages in this monorepo use the npm ecosystem with ESM modules.
# Each package has:
#
#   my-package/
#     src/
#       index.ts          # entry point (main in package.json points here)
#       implementation.ts  # actual code
#     tests/
#       implementation.test.ts  # test files
#     package.json        # npm metadata and dependencies
#     tsconfig.json       # TypeScript compiler configuration
#     vitest.config.ts    # optional Vitest configuration
#
# TYPESCRIPT DEPENDENCY MANAGEMENT
# --------------------------------
# Monorepo packages reference siblings via file: dependencies in package.json:
#
#   "dependencies": {
#     "@coding-adventures/transistors": "file:../transistors"
#   }
#
# When npm ci runs, it creates a symlink from node_modules/@coding-
# adventures/transistors to ../transistors. Vitest can then resolve and
# transform the TypeScript source directly (no compilation step needed).
#
# CRITICAL LESSONS LEARNED (see lessons.md):
#   - package.json "main" must be "src/index.ts" (NOT "dist/index.js")
#     when Vitest transforms TypeScript source directly
#   - ALL transitive file: deps must be listed as direct deps in package.json
#   - Use "npm ci --quiet" in BUILD files, not chained cd/install patterns
#   - Include @vitest/coverage-v8 in devDependencies for coverage
#
# EXAMPLE BUILD FILE
# ------------------
#   load("//rules:typescript_library.star", "ts_library")
#
#   ts_library(
#       name = "logic-gates",
#       srcs = ["src/**/*.ts"],
#       deps = ["typescript/transistors"],
#       test_runner = "vitest",
#   )
#
# ============================================================================

def ts_library(name, srcs = [], deps = [], test_runner = "vitest"):
    # Register a TypeScript library target for the build system.
    #
    # TypeScript libraries use npm's lockfile mode for dependency installation.
    # Test execution is delegated to package scripts so a package can use the
    # platform-provided Node test runner without adding a test framework.
    #
    # Args:
    #     name: The package name, matching the directory under
    #           code/packages/typescript/. For example, "logic-gates" maps to
    #           code/packages/typescript/logic-gates/.
    #
    #           In package.json, the name is typically scoped:
    #           "@coding-adventures/logic-gates". The build target name omits
    #           the scope prefix.
    #
    #     srcs: File paths or glob patterns for change detection.
    #           Typical patterns:
    #               ["src/**/*.ts"]                        — source only
    #               ["src/**/*.ts", "tests/**/*.ts"]       — source and tests
    #               ["src/**/*.ts", "package.json"]        — source and deps
    #
    #           Note: Changes to package.json can affect dependency resolution,
    #           so tracking it is often wise.
    #
    #     deps: Dependencies as "language/package-name" strings.
    #           Examples:
    #               ["typescript/transistors"]
    #               ["typescript/logic-gates", "typescript/arithmetic"]
    #
    #           IMPORTANT: These must match the file: references in
    #           package.json. AND you must include transitive deps! If your
    #           package depends on lexer which depends on state-machine, list
    #           both "typescript/lexer" AND "typescript/state-machine".
    #
    #           This is because npm ci does NOT recursively install file:
    #           deps' own file: deps. Without listing transitive deps
    #           directly, CI builds fail with ERR_MODULE_NOT_FOUND.
    #
    #     test_runner: Which test framework to use. Currently supported:
    #
    #           "vitest"  — (default) Vite-native test framework. Fast, with
    #                       built-in TypeScript support (no tsc step needed),
    #                       Jest-compatible API, and native ESM support.
    #                       Runs the lockfile-local Vitest binary offline.
    #
    #           "node"    — Node's built-in test runner. The build tool first
    #                       compiles the package, then runs test:coverage.
    #
    install_cmd = {"type": "cmd", "program": "npm", "args": ["ci", "--quiet"]}
    build_cmd = {"type": "cmd", "program": "npm", "args": ["run", "build", "--silent"]}
    vitest_cmd = {"type": "cmd", "program": "npm", "args": ["exec", "--offline", "--", "vitest", "run", "--coverage"]}
    test_cmd = {"type": "cmd", "program": "npm", "args": ["run", "test:coverage", "--silent"]}
    commands = [install_cmd, vitest_cmd]
    if test_runner == "node":
        commands = [install_cmd, build_cmd, test_cmd]

    return {
        # "ts_library" triggers TypeScript-specific build logic:
        #   - npm ci for reproducible dependency installation
        #   - offline local runner or package-owned Node test scripts
        # Note: there's no separate lint step here because TypeScript
        # type-checking (tsc --noEmit) serves as the lint step.
        "rule": "ts_library",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "test_runner": test_runner,
        "commands": commands,
    }
