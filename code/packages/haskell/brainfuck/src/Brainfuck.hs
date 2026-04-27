module Brainfuck
    ( description
    , BrainfuckOp(..)
    , BrainfuckError(..)
    , tokenize
    , parseSource
    ) where

description :: String
description = "Haskell Brainfuck frontend for compiler convergence"

data BrainfuckOp
    = MoveRight
    | MoveLeft
    | Increment
    | Decrement
    | Output
    | Input
    | Loop [BrainfuckOp]
    deriving (Eq, Show)

data BrainfuckError = BrainfuckError
    { brainfuckErrorMessage :: String
    , brainfuckErrorOffset :: Int
    }
    deriving (Eq, Show)

tokenize :: String -> String
tokenize = filter (`elem` "><+-.,[]")

parseSource :: String -> Either BrainfuckError [BrainfuckOp]
parseSource source =
    case parseMany 0 (tokenize source) of
        Left err -> Left err
        Right (ops, rest, offset) ->
            case rest of
                [] -> Right ops
                ']' : _ ->
                    Left
                        BrainfuckError
                            { brainfuckErrorMessage = "unmatched closing bracket"
                            , brainfuckErrorOffset = offset
                            }
                _ -> Right ops

parseMany :: Int -> String -> Either BrainfuckError ([BrainfuckOp], String, Int)
parseMany offset input =
    go offset input []
  where
    go current rest acc =
        case rest of
            [] -> Right (acc, [], current)
            ']' : _ -> Right (acc, rest, current)
            '[' : more ->
                case parseMany (current + 1) more of
                    Left err -> Left err
                    Right (body, afterBody, afterOffset) ->
                        case afterBody of
                            ']' : finalRest -> go (afterOffset + 1) finalRest (acc ++ [Loop body])
                            _ ->
                                Left
                                    BrainfuckError
                                        { brainfuckErrorMessage = "unmatched opening bracket"
                                        , brainfuckErrorOffset = current
                                        }
            ch : more -> go (current + 1) more (acc ++ [toOp ch])

toOp :: Char -> BrainfuckOp
toOp ch =
    case ch of
        '>' -> MoveRight
        '<' -> MoveLeft
        '+' -> Increment
        '-' -> Decrement
        '.' -> Output
        ',' -> Input
        _ -> error "toOp called with non-Brainfuck token"
