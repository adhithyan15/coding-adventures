# Go Bloom filter

A configurable Bloom filter for probabilistic membership checks with zero false
negatives and tunable false-positive probability.

```go
filter := bloomfilter.MustNew(1000, 0.01)
filter.Add("hello")
present := filter.Contains("hello")
```
