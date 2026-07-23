set -e

# BUILD — language-ladder (the HL03 unified curriculum learning app)
#
# Depends, via a `file:` dep, on the curriculum data package:
#
#     human-language-data   ← lesson frontmatter parser + taxonomy queries
#                             (a leaf: it has no dependencies of its own)
#
# Per `lessons.md`: BUILD files MUST install every transitive `file:` dep in
# leaf-to-root order BEFORE the program's own `npm install`. Skipping one gives
# a node_modules with gaps — imports that type-check but blow up at runtime.
# Here the chain is only one deep, but the rule is the rule, and the build-tool's
# `-validate-build-files` check enforces it: adding the dep without this line
# fails CI with "missing prerequisite refs for standalone builds".
#
# The `cd` runs in its own subshell, so it does not persist to the next line —
# every command starts fresh from this program's directory.
#
# Tools (npm, npx, node) are expected on PATH (mise shims locally; CI installs
# Node directly via actions/setup-node).

cd ../../../packages/typescript/human-language-data && npm install --silent
npm install
npm run build
npx vitest run
