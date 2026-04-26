set -euo pipefail

cd ../../../packages/typescript/directed-graph && npm install
cd ../state-machine && npm install
cd ../grammar-tools && npm install
cd ../lexer && npm install
cd ../parser && npm install
cd ../lattice-lexer && npm install
cd ../lattice-parser && npm install
cd ../lattice-ast-to-css && npm install
cd ../lattice-transpiler && npm install
cd ../../../programs/typescript/ml-learning-visualizer && npm install
npm run build
npx vitest run
