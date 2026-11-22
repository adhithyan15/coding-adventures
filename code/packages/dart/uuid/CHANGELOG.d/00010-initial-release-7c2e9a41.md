## 0.1.0

- Initial Dart lane of the uuid identity: `Uuid` value type over 16
  big-endian bytes with v1, v3, v4, v5, and v7 factories, `parse`/`isValid`
  over standard, compact, braced, and URN forms, version/variant/nil/max
  inspection, `Comparable` ordering, and the four RFC 4122 namespaces.
- v3/v5 hash through the in-lane `coding_adventures_md5` and
  `coding_adventures_sha1` path dependencies; names are UTF-8 encoded.
- v1 uses a random node ID with the multicast bit set and microsecond-derived
  timestamps; v7 embeds the Unix-millisecond clock.
- Tests mirror the reference suite, pin RFC vectors plus an independently
  oracled Unicode v5 vector, round-trip byte patterns through every form, and
  add tamper/avalanche/hostile-input negative controls (95% branch+function
  coverage gate in BUILD).
