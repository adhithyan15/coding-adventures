## 0.112.0 — 2026-08-11 — static real scalar copy output

Straight-line copies between tracked local real scalars now preserve the
source's canonical static value as an independent snapshot. Reassigning the
source updates only its own tracked value, while all existing control-flow,
call, capture, dynamic-value, and finiteness guards remain unchanged.

