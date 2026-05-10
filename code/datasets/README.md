# datasets

Top-level home for datasets used by demos, tests, learning materials,
and benchmarks across the repo.  Sits alongside `code/packages/`,
`code/programs/`, `code/specs/`, etc.

Each dataset lives in its own subdirectory with a `README.md`
describing its contents and a `LICENSE` file pinning down the licence
terms.  Data lives **here** (rather than scattered next to consuming
programs) so:

- A program that uses a dataset doesn't accidentally inherit ownership
  of it.
- Multiple programs can share the same dataset without duplicating bytes.
- Licensing is centralised: each dataset's licence is right next to its
  data, easy to audit.
- Tooling for image-format conversion, dataset generation, etc. has an
  obvious home.

## Current datasets

| Directory | What's there | Licence |
|-----------|--------------|---------|
| [`test-images/`](test-images/) | 5 synthetic 256×256 RGB PPM images for image-processing demos. | CC0 |

## Contributing a new dataset

1. Pick a short directory name (lower-case, hyphens-not-underscores).
2. Inside it, add:
   - a `README.md` describing the dataset and how to use it,
   - a `LICENSE` file pinning the licence terms (must be open or
     public-domain — anything else needs a separate discussion),
   - the data files themselves, and
   - any generator script (if the data is synthesised).
3. Add a row to the table above.

Synthesised datasets should be byte-for-byte deterministic so re-runs
of the generator don't churn `git diff`.
