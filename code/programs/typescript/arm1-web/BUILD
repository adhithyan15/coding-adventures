#!/usr/bin/env bash
# BUILD — arm1-web
# Interactive ARM1 processor simulator with multi-layer visualization.
#
# Installs the arm1-simulator package dependency first (leaf-to-root order),
# then runs tests for this program package.
set -euo pipefail

# Install the arm1-simulator dependency.
(cd ../arm1-simulator && npm install --silent)

# Install this package's dependencies and run tests.
npm install --silent
npx vitest run --coverage
