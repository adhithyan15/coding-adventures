package brainfuckriscvcompiler

import (
	"bytes"
	"errors"
	"os"
	"path/filepath"

	"github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck"
	brainfuckircompiler "github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck-ir-compiler"
	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	iroptimizer "github.com/adhithyan15/coding-adventures/code/packages/go/ir-optimizer"
	irtoriscvcompiler "github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-riscv-compiler"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	riscvassembler "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-assembler"
	riscv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

const DefaultMemorySize = 65536

type BrainfuckRiscVCompilerOptions struct {
	BuildConfig *brainfuckircompiler.BuildConfig
	OptimizeIR  *bool
	MemorySize  int
	Input       []byte
}

type PackageResult struct {
	Source       string
	AST          *parser.ASTNode
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
	Package    *PackageResult
	Output     []byte
	ExitCode   uint32
	HostExited bool
	Halted     bool
	Steps      int
	Host       *riscv.HostIO
}

func (r *RunResult) OutputString() string {
	if r == nil {
		return ""
	}
	return string(r.Output)
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

type BrainfuckRiscVCompiler struct {
	buildConfig brainfuckircompiler.BuildConfig
	optimizer   *iroptimizer.IrOptimizer
	memorySize  int
	input       []byte
}

func NewBrainfuckRiscVCompiler(options *BrainfuckRiscVCompilerOptions) *BrainfuckRiscVCompiler {
	config := brainfuckircompiler.ReleaseConfig()
	optimize := true
	memorySize := DefaultMemorySize
	var input []byte
	if options != nil {
		if options.BuildConfig != nil {
			config = *options.BuildConfig
		}
		if options.OptimizeIR != nil {
			optimize = *options.OptimizeIR
		}
		if options.MemorySize > 0 {
			memorySize = options.MemorySize
		}
		input = append([]byte(nil), options.Input...)
	}

	optimizer := iroptimizer.DefaultPasses()
	if !optimize {
		optimizer = iroptimizer.NoOp()
	}

	return &BrainfuckRiscVCompiler{
		buildConfig: config,
		optimizer:   optimizer,
		memorySize:  memorySize,
		input:       input,
	}
}

func (c *BrainfuckRiscVCompiler) CompileSource(source string) (*PackageResult, error) {
	ast, err := brainfuck.ParseBrainfuck(source)
	if err != nil {
		return nil, wrapStage("parse", err)
	}

	irResult, err := brainfuckircompiler.Compile(ast, "input.bf", c.buildConfig)
	if err != nil {
		return nil, wrapStage("compile-ir", err)
	}

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
		return nil, wrapStage("assemble-riscv", errAssemblerMismatch)
	}

	return &PackageResult{
		Source:       source,
		AST:          ast,
		RawIR:        rawIR,
		Optimization: optimization,
		OptimizedIR:  optimizedIR,
		MachineCode:  machineCode,
		Assembled:    assembled,
		Assembly:     machineCode.Assembly,
		Binary:       assembled.Bytes,
	}, nil
}

func (c *BrainfuckRiscVCompiler) RunSource(source string) (*RunResult, error) {
	return c.RunSourceWithInput(source, c.input)
}

func (c *BrainfuckRiscVCompiler) RunSourceWithInput(source string, input []byte) (*RunResult, error) {
	result, err := c.CompileSource(source)
	if err != nil {
		return nil, err
	}

	host := riscv.NewHostIO(input)
	sim := riscv.NewRiscVSimulatorWithHost(c.memorySize, host)
	traces := sim.Run(result.Binary)

	return &RunResult{
		Package:    result,
		Output:     append([]byte(nil), host.Output...),
		ExitCode:   host.ExitCode,
		HostExited: host.Exited,
		Halted:     sim.CPU.Halted,
		Steps:      len(traces),
		Host:       host,
	}, nil
}

func (c *BrainfuckRiscVCompiler) WriteAssemblyFile(source, outputPath string) (*PackageResult, error) {
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

func (c *BrainfuckRiscVCompiler) WriteBinaryFile(source, outputPath string) (*PackageResult, error) {
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
	return NewBrainfuckRiscVCompiler(nil).CompileSource(source)
}

func PackSource(source string) (*PackageResult, error) {
	return CompileSource(source)
}

func RunSource(source string) (*RunResult, error) {
	return NewBrainfuckRiscVCompiler(nil).RunSource(source)
}

func RunSourceWithInput(source string, input []byte) (*RunResult, error) {
	return NewBrainfuckRiscVCompiler(nil).RunSourceWithInput(source, input)
}

func WriteAssemblyFile(source, outputPath string) (*PackageResult, error) {
	return NewBrainfuckRiscVCompiler(nil).WriteAssemblyFile(source, outputPath)
}

func WriteBinaryFile(source, outputPath string) (*PackageResult, error) {
	return NewBrainfuckRiscVCompiler(nil).WriteBinaryFile(source, outputPath)
}

func writeFile(outputPath string, data []byte) error {
	if dir := filepath.Dir(outputPath); dir != "." && dir != "" {
		if err := os.MkdirAll(dir, 0o755); err != nil {
			return err
		}
	}
	return os.WriteFile(outputPath, data, 0o644)
}

var errAssemblerMismatch = errors.New("assembler output does not match direct backend bytes")

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
