module github.com/adhithyan15/coding-adventures/code/packages/go/dartmouth-basic-riscv-compiler

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/dartmouth-basic-ir-compiler v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/dartmouth-basic-parser v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-optimizer v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-riscv-compiler v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/parser v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/riscv-assembler v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/branch-predictor v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/cache v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/clock v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-source-map v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/core v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/cpu-simulator v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/dartmouth-basic-lexer v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/hazard-detection v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine v0.0.0 // indirect
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/branch-predictor => ../branch-predictor
	github.com/adhithyan15/coding-adventures/code/packages/go/cache => ../cache
	github.com/adhithyan15/coding-adventures/code/packages/go/clock => ../clock
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir => ../compiler-ir
	github.com/adhithyan15/coding-adventures/code/packages/go/compiler-source-map => ../compiler-source-map
	github.com/adhithyan15/coding-adventures/code/packages/go/core => ../core
	github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline => ../cpu-pipeline
	github.com/adhithyan15/coding-adventures/code/packages/go/cpu-simulator => ../cpu-simulator
	github.com/adhithyan15/coding-adventures/code/packages/go/dartmouth-basic-ir-compiler => ../dartmouth-basic-ir-compiler
	github.com/adhithyan15/coding-adventures/code/packages/go/dartmouth-basic-lexer => ../dartmouth-basic-lexer
	github.com/adhithyan15/coding-adventures/code/packages/go/dartmouth-basic-parser => ../dartmouth-basic-parser
	github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph => ../directed-graph
	github.com/adhithyan15/coding-adventures/code/packages/go/grammar-tools => ../grammar-tools
	github.com/adhithyan15/coding-adventures/code/packages/go/hazard-detection => ../hazard-detection
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-optimizer => ../ir-optimizer
	github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-riscv-compiler => ../ir-to-riscv-compiler
	github.com/adhithyan15/coding-adventures/code/packages/go/lexer => ../lexer
	github.com/adhithyan15/coding-adventures/code/packages/go/parser => ../parser
	github.com/adhithyan15/coding-adventures/code/packages/go/riscv-assembler => ../riscv-assembler
	github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator => ../riscv-simulator
	github.com/adhithyan15/coding-adventures/code/packages/go/state-machine => ../state-machine
)
