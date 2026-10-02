### Added — a test that Engram's npm lockfiles cover the versions the React and Electron projects pin

New `tests/engram_npm_lockfiles.rs` emits Engram's React and Electron projects
and checks that every dependency version in the emitted package.json is the one
locked in `code/programs/mosaic/engram-app/npm/{web,electron}/package-lock.json`.
Engram's release lanes install from those locks with `npm ci`, which refuses a
mismatch. Because the build tool reruns a crate's tests only through
`[dependencies]`, and engram-app merely dev-depends on this crate, a version
bump here would otherwise first fail at Engram release time. Bump an npm
version, then run the matching `build-{web,electron}.sh --update-lock`.
