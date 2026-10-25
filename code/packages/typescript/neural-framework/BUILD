# matrix's own lock installs its dependency-free addon shim without building it.
# Reference training uses native Matrix arithmetic and never loads the addon.
(cd ../matrix && npm ci --omit=dev --ignore-scripts)
npm ci --ignore-scripts
npm run build
npm run test:coverage
npm run example
