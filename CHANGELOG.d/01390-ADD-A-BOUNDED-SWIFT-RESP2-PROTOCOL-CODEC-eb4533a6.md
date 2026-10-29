### Add a bounded Swift RESP2 protocol codec

- Add an authority-free Swift codec for all seven distinct RESP2 value forms,
  including binary-safe bulk strings and separate null bulk and null array
  values.
- Bound lines, numeric headers, frames, output, nesting, aggregate values,
  retained fragments, and repeated fragmented work before proportional
  allocation or output growth.
- Preserve failure-atomic incremental decoding across arbitrary fragmentation
  with stable errors, package-native tests, BUILD fronts, and empty declared
  capabilities.
