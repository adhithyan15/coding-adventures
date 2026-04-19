module IRToWasmValidator
    ( description
    , ValidationError(..)
    , validateProgram
    ) where

import CompilerIR hiding (description)

description :: String
description = "Haskell validator for the IR-to-Wasm lowering subset"

data ValidationError = ValidationError
    { validationErrorMessage :: String
    , validationErrorInstructionId :: Int
    }
    deriving (Eq, Show)

validateProgram :: IrProgram -> [ValidationError]
validateProgram program =
    concatMap validateInstruction (irInstructions program)

validateInstruction :: IrInstruction -> [ValidationError]
validateInstruction inst =
    case (irOpcode inst, irOperands inst) of
        (Comment, [LabelRef _]) -> []
        (Label, [LabelRef _]) -> []
        (LoadImm, [Register _, Immediate _]) -> []
        (LoadAddr, [Register _, LabelRef _]) -> []
        (LoadByte, [Register _, Register _, Register _]) -> []
        (StoreByte, [Register _, Register _, Register _]) -> []
        (Add, [Register _, Register _, Register _]) -> []
        (AddImm, [Register _, Register _, Immediate _]) -> []
        (Sub, [Register _, Register _, Register _]) -> []
        (And, [Register _, Register _, Register _]) -> []
        (AndImm, [Register _, Register _, Immediate _]) -> []
        (Jump, [LabelRef _]) -> []
        (BranchZ, [Register _, LabelRef _]) -> []
        (BranchNz, [Register _, LabelRef _]) -> []
        (Call, [LabelRef _]) -> []
        (Ret, []) -> []
        (Syscall, [Immediate _]) -> []
        (Halt, []) -> []
        (Nop, []) -> []
        _ -> [ValidationError "instruction has operands unsupported by the Haskell Wasm lowerer" (irInstructionId inst)]
