module github.com/adhithyan15/coding-adventures/code/programs/go/heap-construction-benchmark

go 1.23

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/heap v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/microbenchmark v0.0.0
)

require github.com/adhithyan15/coding-adventures/code/packages/go/rank-statistics v0.0.0 // indirect

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/heap => ../../../packages/go/heap
	github.com/adhithyan15/coding-adventures/code/packages/go/microbenchmark => ../../../packages/go/microbenchmark
	github.com/adhithyan15/coding-adventures/code/packages/go/rank-statistics => ../../../packages/go/rank-statistics
)
