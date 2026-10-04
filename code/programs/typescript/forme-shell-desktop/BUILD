set -e

(cd ../../../packages/typescript/document-ast && npm ci --quiet)
(cd ../../../packages/typescript/forme-types && npm ci --quiet)
(cd ../../../packages/typescript/forme-interactivity-ir && npm ci --quiet)
(cd ../../../packages/typescript/forme-errors && npm ci --quiet)
(cd ../../../packages/typescript/forme-capability && npm ci --quiet)
(cd ../../../packages/typescript/forme-stage && npm ci --quiet)
(cd ../../../packages/typescript/forme-pipeline-config && npm ci --quiet)
(cd ../../../packages/typescript/blake2b && npm ci --quiet)
(cd ../../../packages/typescript/sha512 && npm ci --quiet)
(cd ../../../packages/typescript/ed25519 && npm ci --quiet)
(cd ../../../packages/typescript/forme-manifest && npm ci --quiet)
(cd ../../../packages/typescript/forme-plugin-runner-ts && npm ci --quiet)
(cd ../../../packages/typescript/forme-plugin-host && npm ci --quiet)
(cd ../../../packages/typescript/forme-identity && npm ci --quiet)
(cd ../../../packages/typescript/forme-cache && npm ci --quiet)
(cd ../../../packages/typescript/directed-graph && npm ci --quiet)
(cd ../../../packages/typescript/state-machine && npm ci --quiet)
(cd ../../../packages/typescript/gfm-parser && npm ci --quiet)
(cd ../../../packages/typescript/document-ast-to-html && npm ci --quiet)
(cd ../../../packages/typescript/forme-aot-meta-link-tags && npm ci --quiet)
(cd ../../../packages/typescript/forme-aot-rss-discovery-link && npm ci --quiet)
(cd ../../../packages/typescript/forme-style-ir && npm ci --quiet)
(cd ../../../packages/typescript/forme-style-to-css && npm ci --quiet)
(cd ../../../packages/typescript/forme-aot-css-slicer && npm ci --quiet)
(cd ../../../packages/typescript/forme-render-static && npm ci --quiet)
(cd ../../../packages/typescript/forme-theme-classless && npm ci --quiet)
(cd ../../../packages/typescript/forme-router && npm ci --quiet)
(cd ../../../packages/typescript/forme-emit-fs && npm ci --quiet)
(cd ../../../packages/typescript/forme-orchestrator && npm ci --quiet)
(cd ../../../packages/typescript/forme-authoring-core && npm ci --quiet)
(cd ../../../packages/typescript/forme-dev-server && npm ci --quiet)
(cd ../../../packages/typescript/forme-authoring-preview && npm ci --quiet)
(cd ../../../packages/typescript/forme-deploy-runner-core && npm ci --quiet)
(cd ../../../packages/typescript/forme-authoring-editor && npm ci --quiet)
(cd ../../../packages/typescript/forme-authoring-publish && npm ci --quiet)
(cd ../../../packages/typescript/forme-authoring-shell && npm ci --quiet)
npm ci --quiet
npm run build
npm run test:coverage
if [ "$(uname -s)" = "Darwin" ]; then
  npm run worker:sea
  npm run worker:sea:test
  npm run native:test
  npm run native:check
  npm run tauri:build
else
  echo "forme-shell-desktop: macOS native gate skipped on $(uname -s)"
fi
