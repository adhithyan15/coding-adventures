# A TypeScript package can pass its own typecheck and fail a stricter source-importing consumer

Splitting Script Ductus into owner modules passed the package's `tsc --noEmit`, but Language
Ladder imports the package's TypeScript source through a `file:` dependency and compiles it under
the app's stricter `noUnusedLocals` setting. Broad type-only imports left by the mechanical split
therefore failed the downstream typecheck even though the library was green.

**Rule:** after a TypeScript source split, run the typecheck of every source-importing `file:`
consumer, not only the changed package. Consumer compiler options can be stricter. Remove imports
with zero use sites rather than weakening either tsconfig; then run the consumer's full BUILD,
including bundle checks that package tests cannot reach.
