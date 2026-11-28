#!/usr/bin/env bash
# build-tool: deps=typescript/test-bootstrap
set -euo pipefail
(cd ../test-bootstrap && npm ci --quiet && npm run build --silent)
npm ci --quiet
npm run build --silent
npm test --silent
