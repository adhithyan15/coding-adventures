set -e

(cd ../document-ast && npm ci --quiet)
(cd ../forme-authoring-core && npm ci --quiet)
(cd ../forme-types && npm ci --quiet)
(cd ../forme-errors && npm ci --quiet)
(cd ../forme-capability && npm ci --quiet)
(cd ../forme-stage && npm ci --quiet)
(cd ../forme-pipeline-config && npm ci --quiet)
(cd ../blake2b && npm ci --quiet)
(cd ../sha512 && npm ci --quiet)
(cd ../ed25519 && npm ci --quiet)
(cd ../forme-manifest && npm ci --quiet)
(cd ../forme-plugin-runner-ts && npm ci --quiet)
(cd ../forme-plugin-host && npm ci --quiet)
(cd ../forme-identity && npm ci --quiet)
(cd ../forme-cache && npm ci --quiet)
(cd ../forme-orchestrator && npm ci --quiet)
(cd ../forme-dev-server && npm ci --quiet)
(cd ../forme-authoring-editor && npm ci --quiet)
(cd ../forme-authoring-preview && npm ci --quiet)
(cd ../forme-deploy-runner-core && npm ci --quiet)
(cd ../forme-authoring-publish && npm ci --quiet)
npm ci --quiet
npm run build
npm run test:coverage
