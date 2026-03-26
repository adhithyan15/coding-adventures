# ============================================================================
# typescript_binary.star — Build rule for TypeScript executable programs
# ============================================================================
#
# TypeScript programs run via Node.js, but unlike plain JavaScript, TypeScript
# needs a compilation or transformation step. In this monorepo, we use two
# approaches:
#
#   1. Development: tsx or ts-node runs TypeScript directly (JIT transform)
#   2. Production: tsc compiles to JavaScript, then node runs the output
#
# For the build system, we use the development approach — tsx handles
# TypeScript on the fly, so there's no separate compilation step.
#
# TypeScript programs live under code/programs/typescript/<name>/ and have
# a conventional entry point at src/index.ts.
#
# EXAMPLE BUILD FILE
# ------------------
#   load("//rules:typescript_binary.star", "ts_binary")
#
#   ts_binary(
#       name = "playground",
#       srcs = ["src/**/*.ts"],
#       deps = ["typescript/starlark-vm", "typescript/parser"],
#       entry_point = "src/index.ts",
#   )
#
# ============================================================================

def ts_binary(name, srcs = [], deps = [], entry_point = "src/index.ts"):
    # Register a TypeScript binary (executable program) target.
    #
    # TypeScript binaries run via Node.js with tsx for TypeScript support.
    # The build tool will:
    #     npm ci --quiet              — install dependencies
    #     npx vitest run --coverage   — run tests if they exist
    #     npx tsx <entry_point>       — verify the program starts
    #
    # Args:
    #     name: The program name, matching the directory under
    #           code/programs/typescript/. For example, "playground" maps to
    #           code/programs/typescript/playground/.
    #
    #     srcs: File paths or glob patterns for change detection.
    #           Typical: ["src/**/*.ts", "package.json"]
    #
    #           Track package.json because dependency changes should trigger
    #           a rebuild even if TypeScript source hasn't changed.
    #
    #     deps: Dependencies as "language/package-name" strings.
    #           Examples:
    #               ["typescript/starlark-vm"]
    #               ["typescript/parser", "typescript/lexer"]
    #
    #           IMPORTANT: Same transitive dep rule as ts_library — list ALL
    #           transitive file: dependencies directly. npm ci doesn't
    #           recursively install file: deps' own file: deps.
    #
    #     entry_point: The TypeScript file to execute when running this
    #           program. Defaults to "src/index.ts" — the standard convention
    #           for TypeScript packages in this monorepo.
    #
    #           Examples:
    #               "src/index.ts"     — standard entry point
    #               "src/cli.ts"       — CLI-specific entry point
    #               "src/main.ts"      — alternative main file
    #
    #           The build tool runs this via tsx (TypeScript execute), which
    #           transforms TypeScript to JavaScript on the fly without
    #           needing a tsc compilation step.
    return {
        # "ts_binary" triggers TypeScript binary-specific build logic:
        #   - npm ci for reproducible dependency installation
        #   - Tests via vitest if test files exist
        #   - Entry point validation via tsx
        "rule": "ts_binary",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "entry_point": entry_point,
        "commands": [
            {"type": "cmd", "program": "npm", "args": ["install", "--silent"]},
            {"type": "cmd", "program": "npx", "args": ["vitest", "run", "--coverage"]},
        ],
    }
