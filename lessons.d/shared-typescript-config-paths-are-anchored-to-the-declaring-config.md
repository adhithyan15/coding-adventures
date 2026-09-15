# Shared TypeScript config paths are anchored to the declaring config

An extending `tsconfig.json` does not rebase an inherited relative `rootDir` or
`outDir` to the child package. TypeScript resolves those paths from the shared
config that declared them, so `rootDir: "src"` in a lane-level base config can
produce TS6059 for every child package and direct output into the lane-level
tree. For shared compiler configs, use TypeScript 5.5 or newer and declare
package-local paths with `${configDir}/src` and `${configDir}/dist`; then audit
every locked compiler and verify every inheritor with `tsc --showConfig`.

Also remember that a failed `tsc` invocation may still emit JavaScript,
declarations, and maps when `noEmitOnError` is not enabled. After reproducing a
compiler-path failure, inspect the source tree for generated artifacts and
remove only the exact verified outputs before continuing.
