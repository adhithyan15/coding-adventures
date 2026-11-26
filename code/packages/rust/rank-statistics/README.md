# Rank statistics

`coding_adventures_rank_statistics` provides the sample median and exact
distribution-free order-statistic confidence interval for a population median.
It is the Rust counterpart of the shared VER07c rank-statistics contract.

```rust
use coding_adventures_rank_statistics::{median, median_interval};

let ratios = [0.91, 0.94, 0.95, 0.97, 0.99, 1.01];
let estimate = median(&ratios)?;
let interval = median_interval(&ratios, 95)?;
assert_eq!(estimate, 0.96);
assert!(interval.is_some());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Inputs must be non-empty and finite. Inputs are copied before sorting and are
never modified. If the sample is too small for the requested confidence, the
interval is `None`. The exact cutoff uses a small local unsigned-integer helper;
the crate has no runtime dependencies.

Run `cargo test -p coding_adventures_rank_statistics` from the Rust workspace.
