# build-tool: deps=typescript/forme-manifest,typescript/forme-plugin-host
cd ../directed-graph && npm ci --silent
cd ../state-machine && npm ci --silent
cd ../cli-builder && npm ci --silent
cd ../grammar-tools && npm ci --silent
cd ../lexer && npm ci --silent
cd ../toml-lexer && npm ci --silent
cd ../parser && npm ci --silent
cd ../toml-parser && npm ci --silent
cd ../document-ast && npm ci --silent
cd ../blake2b && npm ci --silent
cd ../sha512 && npm ci --silent
cd ../ed25519 && npm ci --silent
cd ../forme-types && npm ci --silent && npm run build
cd ../forme-errors && npm ci --silent && npm run build
cd ../forme-capability && npm ci --silent
cd ../forme-manifest && npm ci --silent
cd ../forme-pipeline-config && npm ci --silent
cd ../forme-stage && npm ci --silent
cd ../forme-plugin-runner-ts && npm ci --silent && npm run build
cd ../forme-plugin-host && npm ci --silent && npm run build
cd ../forme-sandbox-core
npm ci --silent && npm run build && npx vitest run --coverage
