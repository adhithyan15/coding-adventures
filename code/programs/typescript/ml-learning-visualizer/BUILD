set -eu

(cd ../../../packages/typescript/directed-graph && npm install)
(cd ../../../packages/typescript/matrix && npm install)
(cd ../../../packages/typescript/multi-directed-graph && npm install)
(cd ../../../packages/typescript/neural-network && npm install)
(cd ../../../packages/typescript/neural-graph-vm && npm install)
(cd ../../../packages/typescript/pixel-container && npm install)
(cd ../../../packages/typescript/paint-instructions && npm install)
(cd ../../../packages/typescript/paint-vm && npm install)
(cd ../../../packages/typescript/paint-vm-svg && npm install)
(cd ../../../packages/typescript/two-layer-network && npm install)
(cd ../../../packages/typescript/state-machine && npm install)
(cd ../../../packages/typescript/grammar-tools && npm install)
(cd ../../../packages/typescript/lexer && npm install)
(cd ../../../packages/typescript/parser && npm install)
(cd ../../../packages/typescript/lattice-lexer && npm install)
(cd ../../../packages/typescript/lattice-parser && npm install)
(cd ../../../packages/typescript/lattice-ast-to-css && npm install)
(cd ../../../packages/typescript/lattice-transpiler && npm install)
npm install
npm run build
npx vitest run
