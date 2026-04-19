module IROptimizer
    ( description
    , OptimizationResult(..)
    , optimizeProgram
    ) where

import CompilerIR hiding (description)

description :: String
description = "Haskell no-op optimization stage for compiler IR"

data OptimizationResult = OptimizationResult
    { optimizationProgram :: IrProgram
    , optimizationChanged :: Bool
    }
    deriving (Eq, Show)

optimizeProgram :: IrProgram -> OptimizationResult
optimizeProgram program =
    OptimizationResult
        { optimizationProgram = program
        , optimizationChanged = False
        }
