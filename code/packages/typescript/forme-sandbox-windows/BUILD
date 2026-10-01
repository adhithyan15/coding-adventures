# build-tool: deps=typescript/forme-sandbox-core
cd ../document-ast && npm install --silent
cd ../blake2b && npm install --silent
cd ../sha512 && npm install --silent
cd ../ed25519 && npm install --silent
cd ../forme-types && npm install --silent && npm run build
cd ../forme-errors && npm install --silent && npm run build
cd ../forme-capability && npm install --silent
cd ../forme-manifest && npm install --silent
cd ../forme-pipeline-config && npm install --silent
cd ../forme-stage && npm install --silent && npm run build
cd ../forme-plugin-runner-ts && npm install --silent && npm run build
cd ../forme-plugin-host && npm install --silent && npm run build
cd ../forme-sandbox-core && npm install --silent && npm run build
cd ../forme-sandbox-windows
npm install --silent && npm run build && npx vitest run --coverage
