cd ../directed-graph && npm ci --quiet
cd ../cli-builder && npm ci --quiet
cd ../grammar-tools && npm ci --quiet
cd ../state-machine && npm ci --quiet
cd ../lexer && npm ci --quiet
cd ../parser && npm ci --quiet
flock /tmp/ts-symbolic-ir-npm.lock sh -c "cd ../symbolic-ir && npm ci --quiet"
cd ../symbolic-vm && npm ci --quiet
cd ../macsyma-lexer && npm ci --quiet
cd ../macsyma-parser && npm ci --quiet
cd ../macsyma-compiler && npm ci --quiet
npm ci --quiet
npx vitest run --coverage
