package nibriscvcompiler

import (
	"bytes"
	"errors"
	"os"
	"path/filepath"
	"strings"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	iroptimizer "github.com/adhithyan15/coding-adventures/code/packages/go/ir-optimizer"
	irtoriscvcompiler "github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-riscv-compiler"
	nibircompiler "github.com/adhithyan15/coding-adventures/code/packages/go/nib-ir-compiler"
	nibparser "github.com/adhithyan15/coding-adventures/code/packages/go/nib-parser"
	nibtypechecker "github.com/adhithyan15/coding-adventures/code/packages/go/nib-type-checker"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	riscvassembler "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-assembler"
	riscv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

const (
	DefaultMemorySize       = 65536
	NibReturnValueRegisterX = 6
)

type NibRiscVCompilerOptions struct {
	BuildConfig *nibircompiler.BuildConfig
	OptimizeIR  *bool
	MemorySize  int
}

type PackageResult struct {
	Source       string
	AST          *parser.ASTNode
	TypedAST     *nibtypechecker.TypedAST
	RawIR        *ir.IrProgram
	Optimization iroptimizer.OptimizationResult
	OptimizedIR  *ir.IrProgram
	MachineCode  *irtoriscvcompiler.MachineCodeResult
	Assembled    *riscvassembler.AssembleResult
	Assembly     string
	Binary       []byte
	AssemblyPath string
	BinaryPath   string
}

type RunResult struct {
	Package     *PackageResult
	ReturnValue uint32
	ExitValue   uint32
	Halted      bool
	Steps       int
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

type NibRiscVCompiler struct {
	buildConfig *nibircompiler.BuildConfig
	optimizer   *iroptimizer.IrOptimizer
	memorySize  int
}

func NewNibRiscVCompiler(options *NibRiscVCompilerOptions) *NibRiscVCompiler {
	optimize := true
	memorySize := DefaultMemorySize
	var config *nibircompiler.BuildConfig
	if options != nil {
		config = options.BuildConfig
		if options.OptimizeIR != nil {
			optimize = *options.OptimizeIR
		}
		if options.MemorySize > 0 {
			memorySize = options.MemorySize
		}
	}

	optimizer := iroptimizer.DefaultPasses()
	if !optimize {
		optimizer = iroptimizer.NoOp()
	}

	return &NibRiscVCompiler{
		buildConfig: config,
		optimizer:   optimizer,
		memorySize:  memorySize,
	}
}

func (c *NibRiscVCompiler) CompileSource(source string) (*PackageResult, error) {
	ast, err := nibparser.ParseNib(source)
	if err != nil {
		return nil, wrapStage("parse", err)
	}

	typeResult := nibtypechecker.CheckNib(ast)
	if !typeResult.OK {
		messages := make([]string, len(typeResult.Errors))
		for index, diagnostic := range typeResult.Errors {
			messages[index] = diagnostic.Message
		}
		return nil, wrapStage("type-check", errors.New(strings.Join(messages, "\n")))
	}

	config := nibircompiler.CallSafeConfig()
	if c.buildConfig != nil {
		config = *c.buildConfig
	}
	irResult := nibircompiler.CompileNib(typeResult.TypedAST, config)
	rawIR := irResult.Program
	optimization := c.optimizer.Optimize(rawIR)
	optimizedIR := optimization.Program

	machineCode, err := irtoriscvcompiler.NewIrToRiscVCompiler().Compile(optimizedIR)
	if err != nil {
		return nil, wrapStage("lower-riscv", err)
	}

	assembled, err := riscvassembler.Assemble(machineCode.Assembly)
	if err != nil {
		return nil, wrapStage("assemble-riscv", err)
	}
	if !bytes.Equal(assembled.Bytes, machineCode.Bytes) {
		return nil, wrapStage("assemble-riscv", errors.New("assembler output does not match direct backend bytes"))
	}

	return &PackageResult{
		Source:       source,
		AST:          ast,
		TypedAST:     typeResult.TypedAST,
		RawIR:        rawIR,
		Optimization: optimization,
		OptimizedIR:  optimizedIR,
		MachineCode:  machineCode,
		Assembled:    assembled,
		Assembly:     machineCode.Assembly,
		Binary:       assembled.Bytes,
	}, nil
}

func (c *NibRiscVCompiler) RunSource(source string) (*RunResult, error) {
	result, err := c.CompileSource(source)
	if err != nil {
		return nil, err
	}

	sim := riscv.NewRiscVSimulator(c.memorySize)
	traces := sim.Run(result.Binary)

	return &RunResult{
		Package:     result,
		ReturnValue: sim.CPU.Registers.Read(NibReturnValueRegisterX),
		ExitValue:   sim.CPU.Registers.Read(10),
		Halted:      sim.CPU.Halted,
		Steps:       len(traces),
	}, nil
}

func (c *NibRiscVCompiler) WriteAssemblyFile(source, outputPath string) (*PackageResult, error) {
	result, err := c.CompileSource(source)
	if err != nil {
		return nil, err
	}
	if err := writeFile(outputPath, []byte(result.Assembly)); err != nil {
		return nil, wrapStage("write", err)
	}
	result.AssemblyPath = outputPath
	return result, nil
}

func (c *NibRiscVCompiler) WriteBinaryFile(source, outputPath string) (*PackageResult, error) {
	result, err := c.CompileSource(source)
	if err != nil {
		return nil, err
	}
	if err := writeFile(outputPath, result.Binary); err != nil {
		return nil, wrapStage("write", err)
	}
	result.BinaryPath = outputPath
	return result, nil
}

func CompileSource(source string) (*PackageResult, error) {
	return NewNibRiscVCompiler(nil).CompileSource(source)
}

func PackSource(source string) (*PackageResult, error) {
	return CompileSource(source)
}

func RunSource(source string) (*RunResult, error) {
	return NewNibRiscVCompiler(nil).RunSource(source)
}

func WriteAssemblyFile(source, outputPath string) (*PackageResult, error) {
	return NewNibRiscVCompiler(nil).WriteAssemblyFile(source, outputPath)
}

func WriteBinaryFile(source, outputPath string) (*PackageResult, error) {
	return NewNibRiscVCompiler(nil).WriteBinaryFile(source, outputPath)
}

func writeFile(outputPath string, data []byte) error {
	if dir := filepath.Dir(outputPath); dir != "." && dir != "" {
		if err := os.MkdirAll(dir, 0o755); err != nil {
			return err
		}
	}
	return os.WriteFile(outputPath, data, 0o644)
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
