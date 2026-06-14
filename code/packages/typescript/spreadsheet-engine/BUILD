# Chain-install every transitive `file:` dependency leaf-to-root, then self.
# (CLAUDE.md: "BUILD files must install ALL transitive dependencies in
#  leaf-to-root order".) The closure below was derived by walking the
# dependencies of each dep; order matters so that npm ci in a dependent finds
# its already-installed dependencies.
#
# NOTE on cas-factor: symbolic-vm depends on `@coding-adventures/cas-factor`,
# but cas-factor is NOT chain-installed explicitly here on purpose. cas-factor
# is a leaf (no local deps of its own), and symbolic-vm's own `npm ci` below
# materializes it transitively, so an explicit `cd ../cas-factor` line is
# unnecessary. It is also actively *harmful* to add one: symbolic-ir and
# symbolic-vm currently ship without BUILD files, so the build graph does not
# know they exist — which means it also does not know cas-factor is a
# prerequisite of this package. The BUILD-file validator therefore rejects any
# reference to the (otherwise-discovered) cas-factor package as an "undeclared
# local package ref". Letting symbolic-vm pull it in keeps this BUILD valid
# without having to retrofit BUILD files onto symbolic-ir/symbolic-vm (which
# would cascade into six existing cas-* BUILD files that under-declare their
# own symbolic-ir prerequisite).
cd ../directed-graph && npm ci --quiet
cd ../state-machine && npm ci --quiet
cd ../cli-builder && npm ci --quiet
cd ../grammar-tools && npm ci --quiet
cd ../lexer && npm ci --quiet
cd ../excel-lexer && npm ci --quiet
cd ../parser && npm ci --quiet
cd ../symbolic-ir && npm ci --quiet
cd ../cas-pattern-matching && npm ci --quiet
cd ../cas-simplify && npm ci --quiet
cd ../symbolic-vm && npm ci --quiet
cd ../excel-parser && npm ci --quiet
npm ci --quiet
npx vitest run --coverage
