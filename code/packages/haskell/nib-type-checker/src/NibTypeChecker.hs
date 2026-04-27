module NibTypeChecker
    ( description
    , NibType(..)
    , TypeDiagnostic(..)
    , TypedAst(..)
    , TypeCheckResult(..)
    , checkSource
    , checkAst
    , functionNodes
    , firstName
    , countParams
    ) where

import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Lexer.Token
import NibParser hiding (description)
import Parser.AST

description :: String
description = "Haskell Nib semantic checker for the convergence subset"

data NibType
    = U4
    | U8
    | Bcd
    | Bool
    | Void
    deriving (Eq, Ord, Show)

data TypeDiagnostic = TypeDiagnostic
    { diagnosticMessage :: String
    , diagnosticLine :: Int
    , diagnosticColumn :: Int
    }
    deriving (Eq, Show)

data TypedAst = TypedAst
    { typedAstRoot :: ASTNode
    }
    deriving (Eq, Show)

data TypeCheckResult = TypeCheckResult
    { typeCheckOk :: Bool
    , typeCheckErrors :: [TypeDiagnostic]
    , typeCheckTypedAst :: TypedAst
    }
    deriving (Eq, Show)

type Env = Map String NibType
type FunctionEnv = Map String ([NibType], NibType)

checkSource :: String -> TypeCheckResult
checkSource source =
    case tokenizeAndParseNib source of
        Left err ->
            TypeCheckResult
                { typeCheckOk = False
                , typeCheckErrors = [TypeDiagnostic (show err) 1 1]
                , typeCheckTypedAst = TypedAst (RuleNode "program" [])
                }
        Right ast -> checkAst ast

checkAst :: ASTNode -> TypeCheckResult
checkAst ast =
    let functions = Map.fromList [(name, (paramTypes fn, returnType fn)) | fn <- functionNodes ast, Just name <- [firstName fn]]
        errors = concatMap (checkFunction functions) (functionNodes ast)
     in TypeCheckResult
            { typeCheckOk = null errors
            , typeCheckErrors = errors
            , typeCheckTypedAst = TypedAst ast
            }

checkFunction :: FunctionEnv -> ASTNode -> [TypeDiagnostic]
checkFunction functions fn =
    case firstChildRule "block" fn of
        Nothing -> []
        Just block -> checkBlock functions (paramEnv fn) (returnType fn) block

checkBlock :: FunctionEnv -> Env -> NibType -> ASTNode -> [TypeDiagnostic]
checkBlock functions env expectedReturn block =
    snd (foldl step (env, []) (childRules block))
  where
    step (currentEnv, errors) stmt =
        let (nextEnv, moreErrors) = checkStmt functions currentEnv expectedReturn stmt
         in (nextEnv, errors ++ moreErrors)

checkStmt :: FunctionEnv -> Env -> NibType -> ASTNode -> (Env, [TypeDiagnostic])
checkStmt functions env expectedReturn stmt =
    let actualStmt = unwrapStmt stmt
     in case actualStmt of
        RuleNode "let_stmt" _ ->
            let name = maybe "<unknown>" id (firstName actualStmt)
                declared = maybe U4 id (firstChildRule "type" actualStmt >>= parseType)
                actual = firstChildRule "expr" actualStmt >>= inferExpr functions env
                errors =
                    case actual of
                        Just valueType | valueType /= declared -> [diagnostic actualStmt ("let `" ++ name ++ "` expects " ++ show declared ++ ", got " ++ show valueType)]
                        _ -> []
             in (Map.insert name declared env, errors)
        RuleNode "assign_stmt" _ ->
            let name = maybe "<unknown>" id (firstName actualStmt)
                actual = firstChildRule "expr" actualStmt >>= inferExpr functions env
                errors =
                    case Map.lookup name env of
                        Nothing -> [diagnostic actualStmt ("unknown variable `" ++ name ++ "`")]
                        Just expected ->
                            case actual of
                                Just valueType | valueType /= expected -> [diagnostic actualStmt ("assignment to `" ++ name ++ "` expects " ++ show expected ++ ", got " ++ show valueType)]
                                _ -> []
             in (env, errors)
        RuleNode "return_stmt" _ ->
            let actual = firstChildRule "expr" actualStmt >>= inferExpr functions env
                errors =
                    case actual of
                        Just valueType | expectedReturn /= Void && valueType /= expectedReturn -> [diagnostic actualStmt ("return expects " ++ show expectedReturn ++ ", got " ++ show valueType)]
                        _ -> []
             in (env, errors)
        _ -> (env, [])

inferExpr :: FunctionEnv -> Env -> ASTNode -> Maybe NibType
inferExpr functions env node =
    case node of
        TokenNode token -> inferToken functions env token
        RuleNode "call_expr" _ -> inferCall functions env node
        RuleNode "add_expr" _ ->
            let operands = childRules node
                types = mapMaybeLocal (inferExpr functions env) operands
             in case types of
                    [] -> Nothing
                    firstType : rest
                        | all (== firstType) rest && isNumeric firstType -> Just firstType
                        | null rest -> Just firstType
                        | otherwise -> Nothing
        RuleNode _ children ->
            case [child | child <- children, isCall child] of
                callNode : _ -> inferExpr functions env callNode
                [] ->
                    case childRules node of
                        [onlyChild] -> inferExpr functions env onlyChild
                        firstChild : _ -> inferExpr functions env firstChild
                        [] ->
                            case childTokens node of
                                token : _ -> inferToken functions env token
                                [] -> Nothing
        _ -> Nothing

inferCall :: FunctionEnv -> Env -> ASTNode -> Maybe NibType
inferCall functions env node = do
    name <- firstName node
    (params, resultType) <- Map.lookup name functions
    let args = callArguments node
        argTypes = mapMaybeLocal (inferExpr functions env) args
    if length args == length params && argTypes == params
        then Just resultType
        else Nothing

inferToken :: FunctionEnv -> Env -> Token -> Maybe NibType
inferToken _ env token =
    case canonicalTokenName token of
        "INT_LIT" -> Just (if read (tokenValue token) <= (15 :: Integer) then U4 else U8)
        "HEX_LIT" -> Just U4
        "NAME" -> Map.lookup (tokenValue token) env
        _ ->
            case tokenValue token of
                "true" -> Just Bool
                "false" -> Just Bool
                _ -> Nothing

functionNodes :: ASTNode -> [ASTNode]
functionNodes node =
    case node of
        RuleNode "fn_decl" _ -> [node]
        RuleNode _ children -> concatMap functionNodes children
        _ -> []

firstName :: ASTNode -> Maybe String
firstName node =
    case node of
        TokenNode token
            | canonicalTokenName token == "NAME" -> Just (tokenValue token)
            | otherwise -> Nothing
        RuleNode _ children -> firstJust (map firstName children)
        _ -> Nothing

countParams :: ASTNode -> Int
countParams node =
    case firstChildRule "param_list" node of
        Nothing -> 0
        Just paramList -> length [param | param@(RuleNode "param" _) <- childRules paramList]

paramTypes :: ASTNode -> [NibType]
paramTypes node =
    case firstChildRule "param_list" node of
        Nothing -> []
        Just paramList -> [valueType | param <- childRules paramList, Just valueType <- [firstChildRule "type" param >>= parseType]]

paramEnv :: ASTNode -> Env
paramEnv node =
    case firstChildRule "param_list" node of
        Nothing -> Map.empty
        Just paramList ->
            Map.fromList
                [ (name, valueType)
                | param <- childRules paramList
                , Just name <- [firstName param]
                , Just valueType <- [firstChildRule "type" param >>= parseType]
                ]

returnType :: ASTNode -> NibType
returnType node =
    maybe Void id (firstChildRule "type" node >>= parseType)

parseType :: ASTNode -> Maybe NibType
parseType node =
    case childTokens node of
        token : _ ->
            case tokenValue token of
                "u4" -> Just U4
                "u8" -> Just U8
                "bcd" -> Just Bcd
                "bool" -> Just Bool
                _ -> Nothing
        [] -> Nothing

unwrapStmt :: ASTNode -> ASTNode
unwrapStmt (RuleNode "stmt" children) =
    case childRules (RuleNode "stmt" children) of
        firstChild : _ -> firstChild
        [] -> RuleNode "stmt" children
unwrapStmt node = node

callArguments :: ASTNode -> [ASTNode]
callArguments node =
    case firstChildRule "arg_list" node of
        Nothing -> []
        Just argList -> [expr | expr@(RuleNode "expr" _) <- childRules argList]

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

isNumeric :: NibType -> Bool
isNumeric valueType = valueType `elem` [U4, U8, Bcd]

diagnostic :: ASTNode -> String -> TypeDiagnostic
diagnostic node message =
    TypeDiagnostic message (nodeLine node) (nodeColumn node)

nodeLine :: ASTNode -> Int
nodeLine (TokenNode token) = tokenLine token
nodeLine (RuleNode _ children) =
    case children of
        firstChild : _ -> nodeLine firstChild
        [] -> 1
nodeLine _ = 1

nodeColumn :: ASTNode -> Int
nodeColumn (TokenNode token) = tokenColumn token
nodeColumn (RuleNode _ children) =
    case children of
        firstChild : _ -> nodeColumn firstChild
        [] -> 1
nodeColumn _ = 1

firstJust :: [Maybe a] -> Maybe a
firstJust values =
    case values of
        [] -> Nothing
        Just value : _ -> Just value
        Nothing : rest -> firstJust rest

mapMaybeLocal :: (a -> Maybe b) -> [a] -> [b]
mapMaybeLocal f values =
    case values of
        [] -> []
        value : rest ->
            case f value of
                Just result -> result : mapMaybeLocal f rest
                Nothing -> mapMaybeLocal f rest
