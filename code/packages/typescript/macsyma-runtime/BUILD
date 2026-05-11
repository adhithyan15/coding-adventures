cd ../directed-graph && npm ci --quiet
cd ../cli-builder && npm ci --quiet
cd ../grammar-tools && npm ci --quiet
cd ../state-machine && npm ci --quiet
cd ../lexer && npm ci --quiet
cd ../parser && npm ci --quiet
while ! mkdir /tmp/ts-symbolic-ir-npm.lock 2>/dev/null; do sleep 1; done; (cd ../symbolic-ir && npm ci --quiet); EC=$?; rmdir /tmp/ts-symbolic-ir-npm.lock 2>/dev/null; exit $EC
cd ../symbolic-vm && npm ci --quiet
cd ../macsyma-lexer && npm ci --quiet
cd ../macsyma-parser && npm ci --quiet
cd ../macsyma-compiler && npm ci --quiet
npm ci --quiet
npx vitest run --coverage
