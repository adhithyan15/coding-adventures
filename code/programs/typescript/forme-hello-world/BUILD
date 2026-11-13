set -e

# BUILD — forme-hello-world
#
# End-to-end Forme demo program.  Depends (via file: deps) on the five
# pipeline stages, the orchestrator, the pipeline-config types, and
# (transitively) the entire FM01 kernel + the document-ast / gfm parser
# / blake2b primitives those build on top of.
#
# Per `lessons.md` 2026-04-21 (and again 2026-05-02): BUILD files MUST
# install every transitive `file:` dep in leaf-to-root order before the
# program's own `npm install`.  Skipping any one of them produces a
# package whose imports resolve at type-check time but break at runtime
# because node_modules has gaps.
#
# Order is the union of every direct dep's BUILD prerequisites,
# topologically flattened (leaves first):
#
#     document-ast            ← Content IR (no Forme deps)
#     forme-types             ← kind vocabulary
#     forme-interactivity-ir  ← hostile-data FM05 validation and snapshots
#     forme-errors            ← StageError / CapabilityError / CancellationError
#     forme-capability        ← capability parser+matcher
#     forme-stage             ← Stage<In,Out> + StageContext
#     forme-aot-meta-link-tags ← canonical + description head tags
#     forme-aot-rss-discovery-link ← RSS/Atom discovery head tags
#     blake2b                 ← hash primitive
#     sha512                  ← Ed25519 hash primitive
#     ed25519                 ← manifest signature verification
#     forme-manifest          ← plugin manifest parser and verifier
#     forme-plugin-runner-ts  ← plugin-side FM02 runner SDK
#     forme-plugin-host       ← sandboxed StageRef plugin loading
#     forme-identity          ← LogicalId + RevisionId
#     forme-cache             ← orchestrator cache backend
#     forme-pipeline-config   ← PipelineConfig types + validateConfig
#     directed-graph          ← used by forme-render-static (via doc-ast-to-html)
#     state-machine           ← used by forme-render-static (via gfm-parser)
#     gfm-parser              ← used by forme-parse-markdown
#     document-ast-to-html    ← used by forme-render-static
#     forme-orchestrator      ← runs the pipeline
#     forme-source-fs         ← stage 1
#     forme-parse-markdown    ← stage 2
#     forme-router            ← stage 3
#     forme-render-static     ← stage 4
#     forme-emit-fs           ← stage 5
#
# Each `cd` runs in its own subshell so it does NOT persist to the next
# line — every command starts fresh from this program's directory.
#
# Tools (npm, npx, node, tsx) are expected on PATH (mise shims locally;
# CI installs Node directly via actions/setup-node).

cd ../../../packages/typescript/cli-builder && npm ci --silent
cd ../../../packages/typescript/grammar-tools && npm ci --silent
cd ../../../packages/typescript/lexer && npm ci --silent
cd ../../../packages/typescript/toml-lexer && npm ci --silent
cd ../../../packages/typescript/parser && npm ci --silent
cd ../../../packages/typescript/toml-parser && npm ci --silent
cd ../../../packages/typescript/document-ast && npm ci --silent
cd ../../../packages/typescript/forme-types && npm ci --silent
cd ../../../packages/typescript/forme-interactivity-ir && npm ci --silent
cd ../../../packages/typescript/forme-errors && npm ci --silent
cd ../../../packages/typescript/forme-capability && npm ci --silent
cd ../../../packages/typescript/forme-stage && npm ci --silent
cd ../../../packages/typescript/forme-aot-meta-link-tags && npm ci --silent
cd ../../../packages/typescript/forme-aot-rss-discovery-link && npm ci --silent
cd ../../../packages/typescript/forme-style-ir && npm ci --silent
cd ../../../packages/typescript/forme-style-to-css && npm ci --silent
cd ../../../packages/typescript/forme-aot-css-slicer && npm ci --silent
cd ../../../packages/typescript/blake2b && npm ci --silent
cd ../../../packages/typescript/sha512 && npm ci --silent
cd ../../../packages/typescript/ed25519 && npm ci --silent
cd ../../../packages/typescript/forme-manifest && npm ci --silent
cd ../../../packages/typescript/forme-plugin-runner-ts && npm ci --silent
cd ../../../packages/typescript/forme-plugin-host && npm ci --silent
cd ../../../packages/typescript/forme-identity && npm ci --silent
cd ../../../packages/typescript/forme-cache && npm ci --silent
cd ../../../packages/typescript/forme-pipeline-config && npm ci --silent
cd ../../../packages/typescript/directed-graph && npm ci --silent
cd ../../../packages/typescript/state-machine && npm ci --silent
cd ../../../packages/typescript/gfm-parser && npm ci --silent
cd ../../../packages/typescript/document-ast-to-html && npm ci --silent
cd ../../../packages/typescript/forme-orchestrator && npm ci --silent
cd ../../../packages/typescript/forme-source-fs && npm ci --silent
cd ../../../packages/typescript/forme-parse-markdown && npm ci --silent
cd ../../../packages/typescript/forme-router && npm ci --silent
cd ../../../packages/typescript/forme-render-static && npm ci --silent
cd ../../../packages/typescript/forme-theme-classless && npm ci --silent
cd ../../../packages/typescript/forme-emit-fs && npm ci --silent
npm ci --silent
npx vitest run
