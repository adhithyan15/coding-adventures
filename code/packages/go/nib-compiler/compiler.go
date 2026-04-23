package nibcompiler

import (
	"fmt"
	"os"
	"strings"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	intel4004assembler "github.com/adhithyan15/coding-adventures/code/packages/go/intel-4004-assembler"
	intel4004packager "github.com/adhithyan15/coding-adventures/code/packages/go/intel-4004-packager"
	iroptimizer "github.com/adhithyan15/coding-adventures/code/packages/go/ir-optimizer"
	irtointel4004compiler "github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-intel-4004-compiler"
	nibircompiler "github.com/adhithyan15/coding-adventures/code/packages/go/nib-ir-compiler"
	nibparser "github.com/adhithyan15/coding-adventures/code/packages/go/nib-parser"
	nibtypechecker "github.com/adhithyan15/coding-adventures/code/packages/go/nib-type-checker"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

type PackageResult struct {
	Source       string
	AST          *parser.ASTNode
	TypedAST     *nibtypechecker.TypedAST
	RawIR        *ir.IrProgram
	Optimization iroptimizer.OptimizationResult
	OptimizedIR  *ir.IrProgram
	Assembly     string
	Binary       []byte
	HexText      string
	Origin       int
	HexPath      string
}

type PackageError struct {
	Stage   string
	Message string
	Cause   error
}

func (e PackageError) Error() string {
	return e.Message
}

func (e PackageError) String() string {
	return "[" + e.Stage + "] " + e.Message
}

type NibCompiler struct {
	Backend *irtointel4004compiler.IrToIntel4004Compiler
	Options Options
}

type Options struct {
	BuildConfig nibircompiler.BuildConfig
	OptimizeIR  bool
}

func NewNibCompiler() *NibCompiler {
	return &NibCompiler{
		Backend: irtointel4004compiler.NewIrToIntel4004Compiler(),
		Options: Options{
			BuildConfig: nibircompiler.ReleaseConfig(),
			OptimizeIR:  true,
		},
	}
}

func CompileSource(source string) (PackageResult, error) {
	return NewNibCompiler().CompileSource(source, nil)
}

func PackSource(source string) (PackageResult, error) {
	return CompileSource(source)
}

func WriteHexFile(source string, outputPath string) (PackageResult, error) {
	return NewNibCompiler().WriteHexFile(source, outputPath, nil)
}

func (c *NibCompiler) CompileSource(source string, options *Options) (PackageResult, error) {
	opts := c.Options
	if options != nil {
		if options.BuildConfig != (nibircompiler.BuildConfig{}) {
			opts.BuildConfig = options.BuildConfig
		}
		opts.OptimizeIR = options.OptimizeIR
	}

	ast, err := nibparser.ParseNib(source)
	if err != nil {
		return PackageResult{}, PackageError{Stage: "parse", Message: err.Error(), Cause: err}
	}

	typeResult := nibtypechecker.CheckNib(ast)
	if !typeResult.OK {
		lines := []string{}
		for _, diagnostic := range typeResult.Errors {
			lines = append(lines, fmt.Sprintf("Line %d, Col %d: %s", diagnostic.Line, diagnostic.Column, diagnostic.Message))
		}
		return PackageResult{}, PackageError{Stage: "type-check", Message: strings.Join(lines, "\n")}
	}

	rawIR := nibircompiler.CompileNib(typeResult.TypedAST, opts.BuildConfig).Program

	optimizer := iroptimizer.NoOp()
	if opts.OptimizeIR {
		optimizer = iroptimizer.DefaultPasses()
	}
	optimization := optimizer.Optimize(rawIR)

	assembly, err := c.Backend.Compile(optimization.Program)
	if err != nil {
		return PackageResult{}, PackageError{Stage: "validate", Message: err.Error(), Cause: err}
	}

	binary, err := intel4004assembler.Assemble(assembly)
	if err != nil {
		return PackageResult{}, PackageError{Stage: "assemble", Message: err.Error(), Cause: err}
	}

	hexText, err := intel4004packager.EncodeHex(binary, 0)
	if err != nil {
		return PackageResult{}, PackageError{Stage: "package", Message: err.Error(), Cause: err}
	}

	return PackageResult{
		Source:       source,
		AST:          ast,
		TypedAST:     typeResult.TypedAST,
		RawIR:        rawIR,
		Optimization: optimization,
		OptimizedIR:  optimization.Program,
		Assembly:     assembly,
		Binary:       binary,
		HexText:      hexText,
		Origin:       0,
	}, nil
}

func (c *NibCompiler) WriteHexFile(source string, outputPath string, options *Options) (PackageResult, error) {
	result, err := c.CompileSource(source, options)
	if err != nil {
		return PackageResult{}, err
	}
	if writeErr := os.WriteFile(outputPath, []byte(result.HexText), 0o644); writeErr != nil {
		return PackageResult{}, PackageError{Stage: "write", Message: writeErr.Error(), Cause: writeErr}
	}
	result.HexPath = outputPath
	return result, nil
}
