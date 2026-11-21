npm ci --prefix ../../../packages/typescript/test-bootstrap --quiet
npm run build --prefix ../../../packages/typescript/test-bootstrap
cd ../../../packages/typescript/analog-waveform && npm ci --quiet && npm run build --silent
cd ../../../packages/typescript/pixel-container && npm ci --quiet
cd ../../../packages/typescript/paint-instructions && npm ci --quiet
cd ../../../packages/typescript/paint-vm && npm ci --quiet
cd ../../../packages/typescript/paint-vm-canvas && npm ci --quiet
cd ../../../packages/typescript/ui-components && npm ci --quiet
cd ../../../packages/typescript/power-supply && npm ci --quiet
cd ../../../packages/typescript/electronics && npm ci --quiet
npm ci --quiet
npx vitest run
