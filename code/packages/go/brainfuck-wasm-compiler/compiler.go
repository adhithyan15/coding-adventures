package brainfuckwasmcompiler

import (
	"errors"
	"os"
	"strings"

	brainfuck "github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck"
	brainfuckircompiler "github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck-ir-compiler"
	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	iroptimizer "github.com/adhithyan15/coding-adventures/code/packages/go/ir-optimizer"
	irtowasmcompiler "github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-wasm-compiler"
	irtowasmvalidator "github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-wasm-validator"
	parser "github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	wasmmoduleencoder "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-module-encoder"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
	wasmvalidator "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-validator"
)

type BrainfuckWasmCompilerOptions struct {
	Filename string
	Optimize *bool
}

type PackageResult struct {
	Source          string
	Filename        string
	AST             *parser.ASTNode
	RawIR           *ir.IrProgram
	Optimization    iroptimizer.OptimizationResult
	OptimizedIR     *ir.IrProgram
	Module          *wasmtypes.WasmModule
	ValidatedModule *wasmvalidator.ValidatedModule
	Binary          []byte
	WasmPath        string
}

type PackageError struct {
	Stage   string
	Message string
	Cause   error
}

func (e *PackageError) Error() string {
	return e.Message
}

func (e *PackageError) Unwrap() error {
	return e.Cause
}

type BrainfuckWasmCompiler struct {
	filename  string
	optimizer *iroptimizer.IrOptimizer
}

func NewBrainfuckWasmCompiler(options *BrainfuckWasmCompilerOptions) *BrainfuckWasmCompiler {
	filename := "program.bf"
	optimize := true
	if options != nil {
		if options.Filename != "" {
			filename = options.Filename
		}
		if options.Optimize != nil {
			optimize = *options.Optimize
		}
	}

	optimizer := iroptimizer.DefaultPasses()
	if !optimize {
		optimizer = iroptimizer.NoOp()
	}

	return &BrainfuckWasmCompiler{
		filename:  filename,
		optimizer: optimizer,
	}
}

func (c *BrainfuckWasmCompiler) CompileSource(source string) (*PackageResult, error) {
	ast, err := brainfuck.ParseBrainfuck(source)
	if err != nil {
		return nil, wrapStage("parse", err)
	}

	rawIR, err := func() (*ir.IrProgram, error) {
		result, err := brainfuckircompiler.Compile(ast, c.filename, brainfuckircompiler.ReleaseConfig())
		if err != nil {
			return nil, err
		}
		return result.Program, nil
	}()
	if err != nil {
		return nil, wrapStage("ir", err)
	}

	optimization := c.optimizer.Optimize(rawIR)
	optimizedIR := optimization.Program

	if validationErrors := irtowasmvalidator.Validate(optimizedIR); len(validationErrors) > 0 {
		messages := make([]string, len(validationErrors))
		for index, current := range validationErrors {
			messages[index] = current.Message
		}
		return nil, wrapStage("lowering-validate", errors.New(strings.Join(messages, "; ")))
	}

	module, err := irtowasmcompiler.NewIrToWasmCompiler().Compile(optimizedIR)
	if err != nil {
		return nil, wrapStage("lower", err)
	}

	validatedModule, err := wasmvalidator.Validate(module)
	if err != nil {
		return nil, wrapStage("validate", err)
	}

	binary, err := wasmmoduleencoder.EncodeModule(module)
	if err != nil {
		return nil, wrapStage("encode", err)
	}

	return &PackageResult{
		Source:          source,
		Filename:        c.filename,
		AST:             ast,
		RawIR:           rawIR,
		Optimization:    optimization,
		OptimizedIR:     optimizedIR,
		Module:          module,
		ValidatedModule: validatedModule,
		Binary:          binary,
	}, nil
}

func (c *BrainfuckWasmCompiler) WriteWasmFile(source, outputPath string) (*PackageResult, error) {
	result, err := c.CompileSource(source)
	if err != nil {
		return nil, err
	}
	if err := os.WriteFile(outputPath, result.Binary, 0o644); err != nil {
		return nil, wrapStage("write", err)
	}
	result.WasmPath = outputPath
	return result, nil
}

func CompileSource(source string) (*PackageResult, error) {
	return NewBrainfuckWasmCompiler(nil).CompileSource(source)
}

func PackSource(source string) (*PackageResult, error) {
	return CompileSource(source)
}

func WriteWasmFile(source, outputPath string) (*PackageResult, error) {
	return NewBrainfuckWasmCompiler(nil).WriteWasmFile(source, outputPath)
}

func wrapStage(stage string, err error) error {
	if err == nil {
		return nil
	}
	if packageErr, ok := err.(*PackageError); ok {
		return packageErr
	}
	return &PackageError{
		Stage:   stage,
		Message: err.Error(),
		Cause:   err,
	}
}
