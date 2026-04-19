module NibIRCompiler
    ( description
    , BuildConfig(..)
    , CompileResult(..)
    , releaseConfig
    , compileNib
    ) where

import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import CompilerIR hiding (description)
import Lexer.Token
import NibTypeChecker hiding (description)
import Parser.AST

description :: String
description = "Haskell Nib to compiler IR frontend"

data BuildConfig = BuildConfig
    { buildConfigOptimize :: Bool
    }
    deriving (Eq, Show)

data CompileResult = CompileResult
    { compileResultProgram :: IrProgram
    }
    deriving (Eq, Show)

releaseConfig :: BuildConfig
releaseConfig = BuildConfig {buildConfigOptimize = True}

compileNib :: TypedAst -> BuildConfig -> CompileResult
compileNib typedAst _ =
    CompileResult (compileProgramFromAst (typedAstRoot typedAst))

compileProgramFromAst :: ASTNode -> IrProgram
compileProgramFromAst root =
    let initial =
            appendMany
                (emptyProgram "_start")
                [ instruction Label [LabelRef "_start"] 0
                , if hasMain then instruction Call [LabelRef "_fn_main"] 1 else instruction Comment [LabelRef "no-main"] 1
                , instruction Halt [] 2
                ]
        (_, finalProgram) = foldl compileFunction (3, initial) (functionNodes root)
     in finalProgram
  where
    hasMain = any ((== Just "main") . firstName) (functionNodes root)

compileFunction :: (Int, IrProgram) -> ASTNode -> (Int, IrProgram)
compileFunction (nextId, program) fn =
    let functionName = maybe "anonymous" id (firstName fn)
        withLabel = appendInstruction program (instruction Label [LabelRef ("_fn_" ++ functionName)] nextId)
        env = Map.fromList [(name, register) | (name, register) <- zip (paramNames fn) [2 ..]]
        (afterBlockId, afterBlockProgram) =
            case firstChildRule "block" fn of
                Nothing -> (nextId + 1, withLabel)
                Just block -> compileBlock (nextId + 1) withLabel env (2 + length (paramNames fn)) block
     in (afterBlockId + 1, appendInstruction afterBlockProgram (instruction Ret [] afterBlockId))

compileBlock :: Int -> IrProgram -> Map String Int -> Int -> ASTNode -> (Int, IrProgram)
compileBlock nextId program env nextRegister block =
    foldl step (nextId, program, env, nextRegister) (childRules block) |> (\(ident, currentProgram, _, _) -> (ident, currentProgram))
  where
    step (ident, currentProgram, currentEnv, registerCursor) stmt =
        let (nextIdent, nextProgram, nextEnv, nextCursor) = compileStmt ident currentProgram currentEnv registerCursor stmt
         in (nextIdent, nextProgram, nextEnv, nextCursor)

compileStmt :: Int -> IrProgram -> Map String Int -> Int -> ASTNode -> (Int, IrProgram, Map String Int, Int)
compileStmt ident program env nextRegister stmt =
    let actualStmt = unwrapStmt stmt
     in case actualStmt of
        RuleNode "let_stmt" _ ->
            let name = maybe ("tmp" ++ show nextRegister) id (firstName actualStmt)
                register = Map.findWithDefault nextRegister name env
                env' = Map.insert name register env
                (ident', program') =
                    case firstChildRule "expr" actualStmt of
                        Nothing -> (ident, program)
                        Just expr -> emitExpr ident program env' register expr
             in (ident', program', env', max nextRegister (register + 1))
        RuleNode "assign_stmt" _ ->
            case firstName actualStmt >>= (`Map.lookup` env) of
                Nothing -> (ident, program, env, nextRegister)
                Just register ->
                    case firstChildRule "expr" actualStmt of
                        Nothing -> (ident, program, env, nextRegister)
                        Just expr ->
                            let (ident', program') = emitExpr ident program env register expr
                             in (ident', program', env, nextRegister)
        RuleNode "return_stmt" _ ->
            case firstChildRule "expr" actualStmt of
                Nothing -> (ident, program, env, nextRegister)
                Just expr ->
                    let (ident', program') = emitExpr ident program env 1 expr
                     in (ident', program', env, nextRegister)
        RuleNode "expr_stmt" _ ->
            case firstChildRule "expr" actualStmt of
                Nothing -> (ident, program, env, nextRegister)
                Just expr ->
                    let (ident', program') = emitExpr ident program env 1 expr
                     in (ident', program', env, nextRegister)
        _ -> (ident, program, env, nextRegister)

emitExpr :: Int -> IrProgram -> Map String Int -> Int -> ASTNode -> (Int, IrProgram)
emitExpr ident program env dest node =
    case node of
        TokenNode token -> emitToken ident program env dest token
        RuleNode "call_expr" _ -> emitCall ident program env dest node
        RuleNode "add_expr" _ ->
            case childRules node of
                [] -> (ident, program)
                firstOperand : restOperands ->
                    let (afterLeftId, afterLeftProgram) = emitExpr ident program env dest firstOperand
                     in foldl (emitAdd dest env) (afterLeftId, afterLeftProgram) restOperands
        RuleNode _ children ->
            case [child | child <- children, isCall child] of
                callNode : _ -> emitExpr ident program env dest callNode
                [] ->
                    case childRules node of
                        [onlyChild] -> emitExpr ident program env dest onlyChild
                        firstChild : _ -> emitExpr ident program env dest firstChild
                        [] ->
                            case childTokens node of
                                token : _ -> emitToken ident program env dest token
                                [] -> (ident, program)
        _ -> (ident, program)

emitAdd :: Int -> Map String Int -> (Int, IrProgram) -> ASTNode -> (Int, IrProgram)
emitAdd dest env (ident, program) rhs =
    case literalValue rhs of
        Just value ->
            let addInst = instruction AddImm [Register dest, Register dest, Immediate value] ident
                maskInst = instruction AndImm [Register dest, Register dest, Immediate 15] (ident + 1)
             in (ident + 2, appendMany program [addInst, maskInst])
        Nothing ->
            let scratch = 32
                (afterRhsId, afterRhsProgram) = emitExpr ident program env scratch rhs
                addInst = instruction Add [Register dest, Register dest, Register scratch] afterRhsId
                maskInst = instruction AndImm [Register dest, Register dest, Immediate 15] (afterRhsId + 1)
             in (afterRhsId + 2, appendMany afterRhsProgram [addInst, maskInst])

emitCall :: Int -> IrProgram -> Map String Int -> Int -> ASTNode -> (Int, IrProgram)
emitCall ident program env dest node =
    let name = maybe "anonymous" id (firstName node)
        (afterArgsId, afterArgsProgram) =
            foldl
                (\(currentId, currentProgram) (arg, register) -> emitExpr currentId currentProgram env register arg)
                (ident, program)
                (zip (callArguments node) [2 ..])
        callInst = instruction Call [LabelRef ("_fn_" ++ name)] afterArgsId
        copyInst = instruction AddImm [Register dest, Register 1, Immediate 0] (afterArgsId + 1)
        instructions = if dest == 1 then [callInst] else [callInst, copyInst]
     in (afterArgsId + length instructions, appendMany afterArgsProgram instructions)

emitToken :: Int -> IrProgram -> Map String Int -> Int -> Token -> (Int, IrProgram)
emitToken ident program env dest token =
    case literalTokenValue token of
        Just value -> (ident + 1, appendInstruction program (instruction LoadImm [Register dest, Immediate value] ident))
        Nothing ->
            case Map.lookup (tokenValue token) env of
                Nothing -> (ident, program)
                Just source -> (ident + 1, appendInstruction program (instruction AddImm [Register dest, Register source, Immediate 0] ident))

literalValue :: ASTNode -> Maybe Integer
literalValue (TokenNode token) = literalTokenValue token
literalValue (RuleNode _ children) = firstJust (map literalValue children)
literalValue _ = Nothing

literalTokenValue :: Token -> Maybe Integer
literalTokenValue token =
    case canonicalTokenName token of
        "INT_LIT" -> Just (read (tokenValue token))
        "HEX_LIT" -> Just (read ("0" ++ drop 1 (tokenValue token)))
        _ ->
            case tokenValue token of
                "true" -> Just 1
                "false" -> Just 0
                _ -> Nothing

paramNames :: ASTNode -> [String]
paramNames node =
    case firstChildRule "param_list" node of
        Nothing -> []
        Just paramList -> [name | param <- childRules paramList, Just name <- [firstName param]]

callArguments :: ASTNode -> [ASTNode]
callArguments node =
    case firstChildRule "arg_list" node of
        Nothing -> []
        Just argList -> [expr | expr@(RuleNode "expr" _) <- childRules argList]

unwrapStmt :: ASTNode -> ASTNode
unwrapStmt (RuleNode "stmt" children) =
    case childRules (RuleNode "stmt" children) of
        firstChild : _ -> firstChild
        [] -> RuleNode "stmt" children
unwrapStmt node = node

firstChildRule :: String -> ASTNode -> Maybe ASTNode
firstChildRule name node =
    case [child | child@(RuleNode ruleName _) <- childRules node, ruleName == name] of
        firstChild : _ -> Just firstChild
        [] -> Nothing

childRules :: ASTNode -> [ASTNode]
childRules (RuleNode _ children) = [child | child@(RuleNode _ _) <- children]
childRules _ = []

childTokens :: ASTNode -> [Token]
childTokens (RuleNode _ children) = [token | TokenNode token <- children]
childTokens (TokenNode token) = [token]
childTokens _ = []

isCall :: ASTNode -> Bool
isCall (RuleNode "call_expr" _) = True
isCall _ = False

appendMany :: IrProgram -> [IrInstruction] -> IrProgram
appendMany = foldl appendInstruction

firstJust :: [Maybe a] -> Maybe a
firstJust values =
    case values of
        [] -> Nothing
        Just value : _ -> Just value
        Nothing : rest -> firstJust rest

(|>) :: a -> (a -> b) -> b
value |> f = f value
