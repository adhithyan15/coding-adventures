package brainfuckjvmcompiler

import (
	"os"

	brainfuck "github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck"
	brainfuckircompiler "github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck-ir-compiler"
	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	iroptimizer "github.com/adhithyan15/coding-adventures/code/packages/go/ir-optimizer"
	irtojvmclassfile "github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-jvm-class-file"
	jvmclassfile "github.com/adhithyan15/coding-adventures/code/packages/go/jvm-class-file"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

type BrainfuckJvmCompilerOptions struct {
	Filename        string
	ClassName       string
	BuildConfig     *brainfuckircompiler.BuildConfig
	Optimize        *bool
	EmitMainWrapper *bool
}

type PackageResult struct {
	Source        string
	Filename      string
	ClassName     string
	AST           *parser.ASTNode
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

type BrainfuckJvmCompiler struct {
	filename        string
	className       string
	buildConfig     brainfuckircompiler.BuildConfig
	optimize        bool
	emitMainWrapper bool
}

func NewBrainfuckJvmCompiler(options *BrainfuckJvmCompilerOptions) *BrainfuckJvmCompiler {
	filename := "program.bf"
	className := "BrainfuckProgram"
	buildConfig := brainfuckircompiler.ReleaseConfig()
	optimize := true
	emitMainWrapper := true

	if options != nil {
		if options.Filename != "" {
			filename = options.Filename
		}
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

	return &BrainfuckJvmCompiler{
		filename:        filename,
		className:       className,
		buildConfig:     buildConfig,
		optimize:        optimize,
		emitMainWrapper: emitMainWrapper,
	}
}

func (c *BrainfuckJvmCompiler) CompileSource(source string) (*PackageResult, error) {
	ast, err := brainfuck.ParseBrainfuck(source)
	if err != nil {
		return nil, wrapStage("parse", err)
	}

	irResult, err := brainfuckircompiler.Compile(ast, c.filename, c.buildConfig)
	if err != nil {
		return nil, wrapStage("ir-compile", err)
	}

	optimizer := iroptimizer.NoOp()
	if c.optimize {
		optimizer = iroptimizer.DefaultPasses()
	}
	optimization := optimizer.Optimize(irResult.Program)

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
		Filename:     c.filename,
		ClassName:    c.className,
		AST:          ast,
		RawIR:        irResult.Program,
		Optimization: optimization,
		OptimizedIR:  optimization.Program,
		Artifact:     artifact,
		ParsedClass:  parsedClass,
		ClassBytes:   artifact.ClassBytes,
	}, nil
}

func (c *BrainfuckJvmCompiler) WriteClassFile(source string, outputDir string) (*PackageResult, error) {
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
	return NewBrainfuckJvmCompiler(nil).CompileSource(source)
}

func PackSource(source string) (*PackageResult, error) {
	return CompileSource(source)
}

func WriteClassFile(source string, outputDir string) (*PackageResult, error) {
	return NewBrainfuckJvmCompiler(nil).WriteClassFile(source, outputDir)
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

func writeFile(path string, data []byte) error {
	return os.WriteFile(path, data, 0o644)
}
