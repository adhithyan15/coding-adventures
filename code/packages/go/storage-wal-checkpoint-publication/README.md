# storage-wal-checkpoint-publication

Publishes a checkpoint successor to the inactive page by invalidating its guard, writing a staged zero-padded body, then publishing the guard. Each step ends with a durability barrier. The previous selected page remains intact, and the receipt distinguishes attempted progress from a confirmed final publication.

The caller must reopen after a write or barrier error before retrying. An exact currently selected record is an idempotent no-write result. Capture and publication form one sequential device interval; concurrent writers and unrelated pending device writes are outside the contract.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
