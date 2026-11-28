# WASM microbenchmark external consumer input

- The external Node consumer runs from the supported `wasm/microbenchmark`
  BUILD after artifact creation. Its exact script now selects and participates
  in the producer package hash and Kiln graph export. Removed the redundant
  standalone test-root BUILD, which was discovered as an unsupported
  `unknown/wasm-microbenchmark-consumer` package. Unknown package handling stays
  fail-closed.
