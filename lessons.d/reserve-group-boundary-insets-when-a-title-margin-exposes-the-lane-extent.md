---
category: Rust
---

# Reserve group boundary insets when a title margin exposes the lane extent

The first subgraph-margin repair passed80 invalid admissions and332 of336
geometry cells, but four horizontal grouped cap cases still clipped a member
node by two units. The old48-unit tail had left only four units after the node;
group bounds clamp six units inside the lane. Include that group inset when
computing the nonzero-margin tail. Preserve the failed receipt and exact zero
behavior; test nodes inside groups as well as nodes inside lanes.
