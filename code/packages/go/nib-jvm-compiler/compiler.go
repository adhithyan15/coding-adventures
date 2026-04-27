package nibjvmcompiler

import (
	"fmt"
	"strings"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	iroptimizer "github.com/adhithyan15/coding-adventures/code/packages/go/ir-optimizer"
	irtojvmclassfile "github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-jvm-class-file"
	jvmclassfile "github.com/adhithyan15/coding-adventures/code/packages/go/jvm-class-file"
	nibircompiler "github.com/adhithyan15/coding-adventures/code/packages/go/nib-ir-compiler"
	nibparser "github.com/adhithyan15/coding-adventures/code/packages/go/nib-parser"
	nibtypechecker "github.com/adhithyan15/coding-adventures/code/packages/go/nib-type-checker"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

type NibJvmCompilerOptions struct {
	ClassName       string
	BuildConfig     *nibircompiler.BuildConfig
	Optimize        *bool
	EmitMainWrapper *bool
}

type PackageResult struct {
	Source        string
	ClassName     string
	AST           *parser.ASTNode
	TypedAST      *nibtypechecker.TypedAST
	RawIR         *ir.IrProgram
	Optimization  iroptimizer.OptimizationResult
	OptimizedIR   *ir.IrProgram
	Artifact      *irtojvmclassfile.JVMClassArtifact
	ParsedClass   *jvmclassfile.JVMClassFile
	ClassBytes    []byte
	ClassFilePath string
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

type NibJvmCompiler struct {
	className       string
	buildConfig     nibircompiler.BuildConfig
	optimize        bool
	emitMainWrapper bool
}

func NewNibJvmCompiler(options *NibJvmCompilerOptions) *NibJvmCompiler {
	className := "NibProgram"
	buildConfig := nibircompiler.ReleaseConfig()
	optimize := true
	emitMainWrapper := true

	if options != nil {
		if options.ClassName != "" {
			className = options.ClassName
		}
		if options.BuildConfig != nil {
			buildConfig = *options.BuildConfig
		}
		if options.Optimize != nil {
			optimize = *options.Optimize
		}
		if options.EmitMainWrapper != nil {
			emitMainWrapper = *options.EmitMainWrapper
		}
	}

	return &NibJvmCompiler{
		className:       className,
		buildConfig:     buildConfig,
		optimize:        optimize,
		emitMainWrapper: emitMainWrapper,
	}
}

func (c *NibJvmCompiler) CompileSource(source string) (*PackageResult, error) {
	ast, err := nibparser.ParseNib(source)
	if err != nil {
		return nil, wrapStage("parse", err)
	}

	typeResult := nibtypechecker.CheckNib(ast)
	if !typeResult.OK {
		lines := make([]string, 0, len(typeResult.Errors))
		for _, diagnostic := range typeResult.Errors {
			lines = append(lines, fmt.Sprintf("Line %d, Col %d: %s", diagnostic.Line, diagnostic.Column, diagnostic.Message))
		}
		return nil, &PackageError{
			Stage:   "type-check",
			Message: strings.Join(lines, "\n"),
		}
	}

	rawIR := nibircompiler.CompileNib(typeResult.TypedAST, c.buildConfig).Program
	optimizer := iroptimizer.NoOp()
	if c.optimize {
		optimizer = iroptimizer.DefaultPasses()
	}
	optimization := optimizer.Optimize(rawIR)

	artifact, err := irtojvmclassfile.LowerIRToJvmClassFile(
		optimization.Program,
		irtojvmclassfile.JvmBackendConfig{
			ClassName:       c.className,
			EmitMainWrapper: c.emitMainWrapper,
		},
	)
	if err != nil {
		return nil, wrapStage("lower-jvm", err)
	}

	parsedClass, err := jvmclassfile.ParseClassFile(artifact.ClassBytes)
	if err != nil {
		return nil, wrapStage("validate-class", err)
	}

	return &PackageResult{
		Source:       source,
		ClassName:    c.className,
		AST:          ast,
		TypedAST:     typeResult.TypedAST,
		RawIR:        rawIR,
		Optimization: optimization,
		OptimizedIR:  optimization.Program,
		Artifact:     artifact,
		ParsedClass:  parsedClass,
		ClassBytes:   artifact.ClassBytes,
	}, nil
}

func (c *NibJvmCompiler) WriteClassFile(source string, outputDir string) (*PackageResult, error) {
	result, err := c.CompileSource(source)
	if err != nil {
		return nil, err
	}
	path, err := irtojvmclassfile.WriteClassFile(result.Artifact, outputDir)
	if err != nil {
		return nil, wrapStage("write", err)
	}
	result.ClassFilePath = path
	return result, nil
}

func CompileSource(source string) (*PackageResult, error) {
	return NewNibJvmCompiler(nil).CompileSource(source)
}

func PackSource(source string) (*PackageResult, error) {
	return CompileSource(source)
}

func WriteClassFile(source string, outputDir string) (*PackageResult, error) {
	return NewNibJvmCompiler(nil).WriteClassFile(source, outputDir)
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
