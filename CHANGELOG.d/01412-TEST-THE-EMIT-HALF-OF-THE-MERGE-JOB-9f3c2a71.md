### Test the emit half of the merge job

- `TestSandboxRecordEmit` (linux/amd64) drives `record prepare`, `record step`
  and `record collect` over two clean actions and asserts the run is emitted and
  written: two manifests and two passes, read back off disk through
  `ledger.Store.Lookup` and `ledger.ManifestSet`.
- Until now only the void half was tested. `TestSandboxRecordCollect` proves a
  dirty run writes nothing; nothing proved a clean run writes the right thing,
  so the segment encoding and the read-back were only exercised by unit tests
  with hand-built records.
- The new test is named in `ci/assert_sandbox_tests.py`'s REQUIRED list, so the
  sandbox job fails if it stops running.
