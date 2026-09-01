(cd ../directed-graph && npm ci --quiet)
(cd ../cli-builder && npm ci --quiet)
(cd ../grammar-tools && npm ci --quiet)
(cd ../lexer && npm ci --quiet)
(cd ../dartmouth-basic-lexer && npm ci --quiet)
(cd ../state-machine && npm ci --quiet)
(cd ../parser && npm ci --quiet)
node -e "const cwd = require('node:path').basename(process.cwd()); if (cwd !== 'dartmouth-basic-parser') throw new Error('unexpected build directory: ' + cwd)"
npm ci --quiet
npx vitest run --coverage
