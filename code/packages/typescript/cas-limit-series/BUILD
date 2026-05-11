flock /tmp/ts-symbolic-ir-npm.lock sh -c "cd ../symbolic-ir && npm ci --quiet"
flock /tmp/ts-cas-substitution-npm.lock sh -c "cd ../cas-substitution && npm ci --quiet"
npm ci --quiet
npm run build
npx vitest run --coverage
