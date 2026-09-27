# CodingAdventures.Itf.CSharp

Interleaved 2 of 5 encoder that emits shared 1D barcode runs and backend-neutral paint scenes.

Validation follows `barcode-symbologies-v1`: at most 4,096 Unicode scalars,
then non-empty even length, then ASCII digits. Stable IDs are exposed through
`InvalidItfInputException.ErrorId`.
