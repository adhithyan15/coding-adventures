# Unreleased — native microbenchmark fixture consumers

Register the Rust, TypeScript, Ruby, and C++ paired benchmark and rank-statistics
packages as consumers of the canonical paired-results fixture. Fixture edits
now select all twelve implemented consumers; language filtering and Kiln graph
export preserve the same exact input. Tests reject a missing consumer in every
lane and exclude lanes without a benchmark package.

Include the fixture in legacy fallback hashes for the same twelve consumers;
bytes-only fixture edits invalidate each package through the runtime dispatcher.
