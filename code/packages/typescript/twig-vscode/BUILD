# Build the generated VS Code extension package.  Compiles TypeScript
# into ./out/ ready for vsce packaging or local install, then runs the
# unit tests with vitest.
npm ci --silent
npx tsc -p .
npm test
