while ! mkdir /tmp/ts-symbolic-ir-npm.lock 2>/dev/null; do sleep 1; done; (cd ../symbolic-ir && npm ci --quiet); EC=$?; rmdir /tmp/ts-symbolic-ir-npm.lock 2>/dev/null; exit $EC
while ! mkdir /tmp/ts-cas-substitution-npm.lock 2>/dev/null; do sleep 1; done; (cd ../cas-substitution && npm ci --quiet); EC=$?; rmdir /tmp/ts-cas-substitution-npm.lock 2>/dev/null; exit $EC
npm ci --quiet
npm run build
npx vitest run --coverage
