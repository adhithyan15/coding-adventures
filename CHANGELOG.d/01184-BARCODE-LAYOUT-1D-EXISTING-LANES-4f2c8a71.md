### Barcode layout 1D existing-lane conformance

- Adopt the bounded `barcode-layout-1d-v1` fixture contract in the C#, F#, Go,
  Haskell, Perl, Python, Rust, and TypeScript package lanes.
- Add strict portable facades, dynamic corpus runners, capability manifests,
  hostile-input limits, zero-authority text rejection, and owned-result checks
  while preserving each package's legacy API.
- Reconcile the Rust barcode consumers with the portable layout facade and
  remove their unused native text-shaping authority.
