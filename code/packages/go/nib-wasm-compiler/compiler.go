package nibwasmcompiler

import (
	"errors"
	"os"
	"path/filepath"
	"strings"

	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	iroptimizer "github.com/adhithyan15/coding-adventures/code/packages/go/ir-optimizer"
	irtowasmcompiler "github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-wasm-compiler"
	irtowasmvalidator "github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-wasm-validator"
	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	nibircompiler "github.com/adhithyan15/coding-adventures/code/packages/go/nib-ir-compiler"
	nibparser "github.com/adhithyan15/coding-adventures/code/packages/go/nib-parser"
	nibtypechecker "github.com/adhithyan15/coding-adventures/code/packages/go/nib-type-checker"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
	wasmmoduleencoder "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-module-encoder"
	wasmtypes "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-types"
	wasmvalidator "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-validator"
)

// NibWasmCompilerOptions configures the orchestration package.
type NibWasmCompilerOptions struct {
	BuildConfig *nibircompiler.BuildConfig
	OptimizeIR  *bool
}

// PackageResult keeps every stage visible so tests and downstream tools can
// inspect the same pipeline artifacts without re-running the compiler.
type PackageResult struct {
	Source          string
	AST             *parser.ASTNode
	TypedAST        *nibtypechecker.TypedAST
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

type NibWasmCompiler struct {
	buildConfig *nibircompiler.BuildConfig
	optimizer   *iroptimizer.IrOptimizer
}

func NewNibWasmCompiler(options *NibWasmCompilerOptions) *NibWasmCompiler {
	optimize := true
	var config *nibircompiler.BuildConfig
	if options != nil {
		config = options.BuildConfig
		if options.OptimizeIR != nil {
			optimize = *options.OptimizeIR
		}
	}

	optimizer := iroptimizer.DefaultPasses()
	if !optimize {
		optimizer = iroptimizer.NoOp()
	}

	return &NibWasmCompiler{
		buildConfig: config,
		optimizer:   optimizer,
	}
}

func (c *NibWasmCompiler) CompileSource(source string) (*PackageResult, error) {
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

	config := nibircompiler.ReleaseConfig()
	if c.buildConfig != nil {
		config = *c.buildConfig
	}
	irResult := nibircompiler.CompileNib(typeResult.TypedAST, config)
	rawIR := irResult.Program
	optimization := c.optimizer.Optimize(rawIR)
	optimizedIR := optimization.Program
	signatures := extractSignatures(typeResult.TypedAST)

	if validationErrors := irtowasmvalidator.Validate(optimizedIR, signatures...); len(validationErrors) > 0 {
		messages := make([]string, len(validationErrors))
		for index, current := range validationErrors {
			messages[index] = current.Message
		}
		return nil, wrapStage("lowering-validate", errors.New(strings.Join(messages, "; ")))
	}

	module, err := irtowasmcompiler.NewIrToWasmCompiler().Compile(optimizedIR, signatures...)
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
		AST:             ast,
		TypedAST:        typeResult.TypedAST,
		RawIR:           rawIR,
		Optimization:    optimization,
		OptimizedIR:     optimizedIR,
		Module:          module,
		ValidatedModule: validatedModule,
		Binary:          binary,
	}, nil
}

func (c *NibWasmCompiler) WriteWasmFile(source, outputPath string) (*PackageResult, error) {
	result, err := c.CompileSource(source)
	if err != nil {
		return nil, err
	}
	if dir := filepath.Dir(outputPath); dir != "." && dir != "" {
		if err := os.MkdirAll(dir, 0o755); err != nil {
			return nil, wrapStage("write", err)
		}
	}
	if err := os.WriteFile(outputPath, result.Binary, 0o644); err != nil {
		return nil, wrapStage("write", err)
	}
	result.WasmPath = outputPath
	return result, nil
}

func CompileSource(source string) (*PackageResult, error) {
	return NewNibWasmCompiler(nil).CompileSource(source)
}

func PackSource(source string) (*PackageResult, error) {
	return CompileSource(source)
}

func WriteWasmFile(source, outputPath string) (*PackageResult, error) {
	return NewNibWasmCompiler(nil).WriteWasmFile(source, outputPath)
}

func extractSignatures(typedAST *nibtypechecker.TypedAST) []irtowasmcompiler.FunctionSignature {
	signatures := []irtowasmcompiler.FunctionSignature{{
		Label:      "_start",
		ParamCount: 0,
		ExportName: "_start",
	}}
	if typedAST == nil || typedAST.Root == nil {
		return signatures
	}

	for _, topDecl := range childNodes(typedAST.Root) {
		decl := unwrapTopDecl(topDecl)
		if decl == nil || decl.RuleName != "fn_decl" {
			continue
		}
		name := firstName(decl)
		if name == "" {
			continue
		}
		signatures = append(signatures, irtowasmcompiler.FunctionSignature{
			Label:      "_fn_" + name,
			ParamCount: countParams(decl),
			ExportName: name,
		})
	}
	return signatures
}

func childNodes(node *parser.ASTNode) []*parser.ASTNode {
	if node == nil {
		return nil
	}
	out := []*parser.ASTNode{}
	for _, child := range node.Children {
		if childNode, ok := child.(*parser.ASTNode); ok {
			out = append(out, childNode)
		}
	}
	return out
}

func unwrapTopDecl(node *parser.ASTNode) *parser.ASTNode {
	if node == nil {
		return nil
	}
	for _, child := range node.Children {
		if childNode, ok := child.(*parser.ASTNode); ok {
			return childNode
		}
	}
	return nil
}

func firstName(node *parser.ASTNode) string {
	if node == nil {
		return ""
	}
	for _, child := range node.Children {
		switch value := child.(type) {
		case lexer.Token:
			if tokenTypeName(value) == "NAME" {
				return value.Value
			}
		case *parser.ASTNode:
			if name := firstName(value); name != "" {
				return name
			}
		}
	}
	return ""
}

func countParams(fnDecl *parser.ASTNode) int {
	for _, child := range childNodes(fnDecl) {
		if child.RuleName != "param_list" {
			continue
		}
		count := 0
		for _, param := range childNodes(child) {
			if param.RuleName == "param" {
				count++
			}
		}
		return count
	}
	return 0
}

func tokenTypeName(token lexer.Token) string {
	if token.TypeName != "" {
		return token.TypeName
	}
	return token.Type.String()
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
