package dartmouthbasicriscvcompiler

import (
	"bytes"
	"errors"
	"os"
	"path/filepath"
	"strings"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	dartmouthbasicircompiler "github.com/adhithyan15/coding-adventures/code/packages/go/dartmouth-basic-ir-compiler"
	dartmouthbasicparser "github.com/adhithyan15/coding-adventures/code/packages/go/dartmouth-basic-parser"
	iroptimizer "github.com/adhithyan15/coding-adventures/code/packages/go/ir-optimizer"
	irtoriscvcompiler "github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-riscv-compiler"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	riscvassembler "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-assembler"
	riscv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

const DefaultMemorySize = 65536

type DartmouthBasicRiscVCompilerOptions struct {
	BuildConfig *dartmouthbasicircompiler.BuildConfig
	OptimizeIR  *bool
	MemorySize  int
}

type PackageResult struct {
	Source       string
	AST          *parser.ASTNode
	RawIR        *ir.IrProgram
	VarRegs      map[string]int
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
	Package   *PackageResult
	Output    string
	ExitValue uint32
	Halted    bool
	Steps     int
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

type DartmouthBasicRiscVCompiler struct {
	buildConfig *dartmouthbasicircompiler.BuildConfig
	optimizer   *iroptimizer.IrOptimizer
	memorySize  int
}

func NewDartmouthBasicRiscVCompiler(options *DartmouthBasicRiscVCompilerOptions) *DartmouthBasicRiscVCompiler {
	optimize := true
	memorySize := DefaultMemorySize
	var config *dartmouthbasicircompiler.BuildConfig
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

	return &DartmouthBasicRiscVCompiler{
		buildConfig: config,
		optimizer:   optimizer,
		memorySize:  memorySize,
	}
}

func (c *DartmouthBasicRiscVCompiler) CompileSource(source string) (*PackageResult, error) {
	ast, err := dartmouthbasicparser.ParseDartmouthBasic(source)
	if err != nil {
		return nil, wrapStage("parse", err)
	}

	irResult, err := dartmouthbasicircompiler.CompileDartmouthBasic(ast, c.buildConfig)
	if err != nil {
		return nil, wrapStage("lower-basic", err)
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
		return nil, wrapStage("assemble-riscv", errors.New("assembler output does not match direct backend bytes"))
	}

	return &PackageResult{
		Source:       source,
		AST:          ast,
		RawIR:        rawIR,
		VarRegs:      irResult.VarRegs,
		Optimization: optimization,
		OptimizedIR:  optimizedIR,
		MachineCode:  machineCode,
		Assembled:    assembled,
		Assembly:     machineCode.Assembly,
		Binary:       assembled.Bytes,
	}, nil
}

func (c *DartmouthBasicRiscVCompiler) RunSource(source string) (*RunResult, error) {
	result, err := c.CompileSource(source)
	if err != nil {
		return nil, err
	}

	host := riscv.NewHostIO(nil)
	sim := riscv.NewRiscVSimulatorWithHost(c.memorySize, host)
	traces := sim.Run(result.Binary)

	return &RunResult{
		Package:   result,
		Output:    host.OutputString(),
		ExitValue: sim.CPU.Registers.Read(10),
		Halted:    sim.CPU.Halted,
		Steps:     len(traces),
	}, nil
}

func (c *DartmouthBasicRiscVCompiler) WriteAssemblyFile(source, outputPath string) (*PackageResult, error) {
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

func (c *DartmouthBasicRiscVCompiler) WriteBinaryFile(source, outputPath string) (*PackageResult, error) {
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
	return NewDartmouthBasicRiscVCompiler(&DartmouthBasicRiscVCompilerOptions{
		BuildConfig: configRef(dartmouthbasicircompiler.ReleaseConfig()),
	}).CompileSource(source)
}

func PackSource(source string) (*PackageResult, error) {
	return CompileSource(source)
}

func RunSource(source string) (*RunResult, error) {
	return NewDartmouthBasicRiscVCompiler(&DartmouthBasicRiscVCompilerOptions{
		BuildConfig: configRef(dartmouthbasicircompiler.ReleaseConfig()),
	}).RunSource(source)
}

func WriteAssemblyFile(source, outputPath string) (*PackageResult, error) {
	return NewDartmouthBasicRiscVCompiler(&DartmouthBasicRiscVCompilerOptions{
		BuildConfig: configRef(dartmouthbasicircompiler.ReleaseConfig()),
	}).WriteAssemblyFile(source, outputPath)
}

func WriteBinaryFile(source, outputPath string) (*PackageResult, error) {
	return NewDartmouthBasicRiscVCompiler(&DartmouthBasicRiscVCompilerOptions{
		BuildConfig: configRef(dartmouthbasicircompiler.ReleaseConfig()),
	}).WriteBinaryFile(source, outputPath)
}

func configRef(config dartmouthbasicircompiler.BuildConfig) *dartmouthbasicircompiler.BuildConfig {
	copy := config
	return &copy
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
	return &PackageError{
		Stage:   stage,
		Message: strings.TrimSpace(err.Error()),
		Cause:   err,
	}
}
