# Chain-install every transitive `file:` dependency leaf-to-root, then self.
# (CLAUDE.md: "BUILD files must install ALL transitive dependencies in
#  leaf-to-root order".) The closure below was derived by walking the
# dependencies of each dep; order matters so that npm ci in a dependent finds
# its already-installed dependencies.
cd ../directed-graph && npm ci --quiet
cd ../state-machine && npm ci --quiet
cd ../cli-builder && npm ci --quiet
cd ../grammar-tools && npm ci --quiet
cd ../lexer && npm ci --quiet
cd ../excel-lexer && npm ci --quiet
cd ../parser && npm ci --quiet
cd ../symbolic-ir && npm ci --quiet
cd ../cas-factor && npm ci --quiet
cd ../cas-pattern-matching && npm ci --quiet
cd ../cas-simplify && npm ci --quiet
cd ../symbolic-vm && npm ci --quiet
cd ../excel-parser && npm ci --quiet
npm ci --quiet
npx vitest run --coverage
