set -e

# BUILD — language-ladder (the HL03 unified curriculum learning app)
#
# Depends, via a `file:` dep, on the curriculum data package:
#
#     pixel-container → paint-instructions → paint-vm → paint-vm-svg
#                                      ↘ human-language-data
#                                            ↘ language-ladder
#
# human-language-data reuses paint-vm-svg for the deterministic book/app figure
# pipeline. The app imports the committed SVGs directly, but the package still
# has to type-check standalone, so its renderer chain is installed first.
#
# Per `lessons.md`: BUILD files MUST install every transitive `file:` dep in
# leaf-to-root order BEFORE the program's own `npm install`. Skipping one gives
# a node_modules with gaps — imports that type-check but blow up at runtime.
# The build-tool's `-validate-build-files` check enforces the complete chain:
# adding the dependency without these lines fails CI with "missing prerequisite
# refs for standalone builds".
#
# The `cd` runs in its own subshell, so it does not persist to the next line —
# every command starts fresh from this program's directory.
#
# Tools (npm, npx, node) are expected on PATH (mise shims locally; CI installs
# Node directly via actions/setup-node).

(cd ../../../packages/typescript/pixel-container && npm ci --silent)
(cd ../../../packages/typescript/paint-instructions && npm ci --silent)
(cd ../../../packages/typescript/paint-vm && npm ci --silent)
(cd ../../../packages/typescript/paint-vm-svg && npm ci --silent)
(cd ../../../packages/typescript/human-language-data && npm ci --silent)
npm install
npm run typecheck
npm run build
npm run check:bundle
npx vitest run
