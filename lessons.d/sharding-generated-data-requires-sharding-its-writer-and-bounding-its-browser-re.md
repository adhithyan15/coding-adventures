# Sharding generated data requires sharding its writer and bounding its browser reader

The book and narration hash manifests had already moved from one corpus file to one file per
language, but two agents regenerating different chapters of Spanish still rewrote the same array.
Moving each chapter to `NNNN.json` removes that collision only if the generator writes those owners
directly and `--check` treats the directory as an exact set. A checker that verifies expected files
but ignores unexpected ones lets deleted chapters survive as authoritative stale data; a generator
that also writes a compatibility aggregate simply moves the original conflict next door.

The browser has the opposite pressure. A recursive `import.meta.glob` over chapter owners would
replace 23 language loaders with 1,088 chapter loaders and make the build graph grow with every C2
chapter. The ownership seam on disk is not automatically the correct delivery seam in the browser.

**Rule:** give generated artifacts the smallest stable write owner, reject both missing and
unexpected owners, and fold them behind a bounded consumer boundary. Here that means one tracked
file per chapter, no tracked aggregate, and one lazy build-time virtual module per registered
language.
