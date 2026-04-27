package irtowasmvalidator

import (
	ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
	irtowasmcompiler "github.com/adhithyan15/coding-adventures/code/packages/go/ir-to-wasm-compiler"
)

const Version = "0.1.0"

type ValidationError struct {
	Rule    string
	Message string
}

type WasmIRValidator struct{}

func NewWasmIRValidator() *WasmIRValidator {
	return &WasmIRValidator{}
}

func (v *WasmIRValidator) Validate(program *ir.IrProgram, functionSignatures ...irtowasmcompiler.FunctionSignature) []ValidationError {
	_, err := irtowasmcompiler.NewIrToWasmCompiler().Compile(program, functionSignatures...)
	if err != nil {
		return []ValidationError{{
			Rule:    "lowering",
			Message: err.Error(),
		}}
	}
	return nil
}

func Validate(program *ir.IrProgram, functionSignatures ...irtowasmcompiler.FunctionSignature) []ValidationError {
	return NewWasmIRValidator().Validate(program, functionSignatures...)
}
