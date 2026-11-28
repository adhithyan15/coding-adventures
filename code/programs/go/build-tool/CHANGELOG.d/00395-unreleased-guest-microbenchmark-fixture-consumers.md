# Unreleased — Guest microbenchmark fixture consumers

Register the Twig and WASM Rust guest packages as exact readers of the portable
paired-results fixture. Selection, Rust filtering, graph export and fallback
cache hashing now cover thirty-nine consumers: thirty-six native libraries, the
paired runner and these two guest fixture readers. Mosaic/Starlark use their own
workload fixtures and are not added to this canonical-input family.
