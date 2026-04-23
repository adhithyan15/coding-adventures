module github.com/adhithyan15/coding-adventures/code/packages/go/riscv-assembler

go 1.26

require github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator v0.0.0

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/branch-predictor v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/cache v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/clock v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/core v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/cpu-simulator v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/hazard-detection v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0 // indirect
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/branch-predictor => ../branch-predictor
	github.com/adhithyan15/coding-adventures/code/packages/go/cache => ../cache
	github.com/adhithyan15/coding-adventures/code/packages/go/clock => ../clock
	github.com/adhithyan15/coding-adventures/code/packages/go/core => ../core
	github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline => ../cpu-pipeline
	github.com/adhithyan15/coding-adventures/code/packages/go/cpu-simulator => ../cpu-simulator
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../directed-graph
	github.com/adhithyan15/coding-adventures/code/packages/go/hazard-detection => ../hazard-detection
	github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator => ../riscv-simulator
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../state-machine
)
