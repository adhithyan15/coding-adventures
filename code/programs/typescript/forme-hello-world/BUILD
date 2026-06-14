set -e

# BUILD — forme-hello-world
#
# End-to-end Forme demo program.  Depends (via file: deps) on the four
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
#     forme-errors            ← StageError / CapabilityError / CancellationError
#     forme-capability        ← capability parser+matcher
#     forme-stage             ← Stage<In,Out> + StageContext
#     blake2b                 ← hash primitive
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
#     forme-render-static     ← stage 3
#     forme-emit-fs           ← stage 4
#
# Each `cd` runs in its own subshell so it does NOT persist to the next
# line — every command starts fresh from this program's directory.
#
# Tools (npm, npx, node, tsx) are expected on PATH (mise shims locally;
# CI installs Node directly via actions/setup-node).

cd ../../../packages/typescript/document-ast && npm install --silent
cd ../../../packages/typescript/forme-types && npm install --silent
cd ../../../packages/typescript/forme-errors && npm install --silent
cd ../../../packages/typescript/forme-capability && npm install --silent
cd ../../../packages/typescript/forme-stage && npm install --silent
cd ../../../packages/typescript/blake2b && npm install --silent
cd ../../../packages/typescript/forme-identity && npm install --silent
cd ../../../packages/typescript/forme-cache && npm install --silent
cd ../../../packages/typescript/forme-pipeline-config && npm install --silent
cd ../../../packages/typescript/directed-graph && npm install --silent
cd ../../../packages/typescript/state-machine && npm install --silent
cd ../../../packages/typescript/gfm-parser && npm install --silent
cd ../../../packages/typescript/document-ast-to-html && npm install --silent
cd ../../../packages/typescript/forme-orchestrator && npm install --silent
cd ../../../packages/typescript/forme-source-fs && npm install --silent
cd ../../../packages/typescript/forme-parse-markdown && npm install --silent
cd ../../../packages/typescript/forme-render-static && npm install --silent
cd ../../../packages/typescript/forme-emit-fs && npm install --silent
npm install --silent
npx vitest run
