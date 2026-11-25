# Elixir rank_statistics

Sample median and exact order-statistic confidence interval functions for finite samples. OTP arbitrary-precision integers keep the binomial cutoff exact; midpoint arithmetic preserves subnormal results and avoids overflow at maximum finite values. Calls return `{:ok, value}` or `{:error, reason}` and do not mutate input lists.

Run `BUILD` or `BUILD_windows` to execute the ExUnit suite. The suite includes
the package-local copy of the shared paired-observation fixture.
