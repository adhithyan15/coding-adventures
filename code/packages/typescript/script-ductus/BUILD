(cd ../pixel-container && npm ci --quiet)
(cd ../paint-instructions && npm ci --quiet)
(cd ../paint-vm && npm ci --quiet)
(cd ../paint-vm-svg && npm ci --quiet)
(cd ../human-language-data && npm ci --quiet)
(cd ../human-language-data && npm run build --silent)
npm install --silent
npm run check:doc-shards
npx vitest run --coverage
