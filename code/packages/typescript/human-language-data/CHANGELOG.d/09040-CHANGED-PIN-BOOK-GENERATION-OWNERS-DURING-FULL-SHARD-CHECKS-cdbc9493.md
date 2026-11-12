### Changed — pin book-generation owners during full shard checks

- Transport the complete book-generation owner tree through pinned parent and
  section handles with at most sixteen concurrent file opens per section when
  `shard-cli --check` validates every plan. The single-plan and public
  synchronous readers remain available and unchanged in behavior.
- Reuse one TypeScript document assembler for the synchronous and batch paths,
  retaining metadata, owner identity, canonical order, dangerous-key,
  duplicate, and independent cross-ledger checks. A missing, extra, linked,
  corrupt, or redirected owner must fail rather than become a smaller book.
- Add fixture and real-corpus parity, malformed and linked owner rejection,
  and a pinned-directory swap attempt. The all-plan 30-second test budget,
  complete corpus, and package 85% line-coverage gate are unchanged.
