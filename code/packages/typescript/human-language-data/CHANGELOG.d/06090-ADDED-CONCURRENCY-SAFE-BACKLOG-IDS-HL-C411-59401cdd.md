### Added — concurrency-safe backlog ids (HL-C411)

- Add `npm run backlog:id -- "<subject>"`, which preserves the readable next
  decimal while deriving an eight-hex identity from the NFC-normalized subject.
- Require every post-`05000` backlog fragment to carry that fingerprint, so two
  concurrent branches can share a decimal and rank without sharing an id.
- Keep all earlier append-only fragments unchanged and validate the cutover in
  the existing repo-wide doc-shard gate.
