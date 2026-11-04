### Preserve metadata history without downloading unused historical blobs

- Use a blobless, full-history checkout for the repository metadata job so
  downloading old file payloads does not consume its contract-test budget.
- Preserve all commit/tree history, branch/tag refs, the complete working tree,
  historical dependency baselines and barcode source evidence. Keep the
  read-only permission, every contract check and the 30-minute timeout.
