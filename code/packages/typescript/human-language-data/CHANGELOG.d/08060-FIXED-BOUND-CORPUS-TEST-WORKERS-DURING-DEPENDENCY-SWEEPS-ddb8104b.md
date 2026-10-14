### Fixed - bound corpus test workers during dependency sweeps

- Limit Vitest to two workers so full-corpus scans and child CLI processes do not multiply every repository build worker into another CPU-sized pool. Keep all assertions, coverage and test deadlines unchanged. Current main also retains its corpus measurement and fixture-copy optimizations.
