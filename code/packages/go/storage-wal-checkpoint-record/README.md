# storage-wal-checkpoint-record

Encodes and decodes the fixed 40-byte DBRF frame for a versioned WAL checkpoint phase, epoch, and prior log boundary. Decode preserves lower frame refusal identities before applying checkpoint-specific validation.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
