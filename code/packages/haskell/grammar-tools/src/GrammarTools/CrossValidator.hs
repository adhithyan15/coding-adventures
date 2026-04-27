module GrammarTools.CrossValidator
    ( crossValidate
    ) where

import Data.List (sort)
import qualified Data.Set as Set
import Data.Set (Set)
import GrammarTools.ParserGrammar
import GrammarTools.TokenGrammar

crossValidate :: TokenGrammar -> ParserGrammar -> [String]
crossValidate tokenGrammar parserGrammar =
    missingTokenIssues ++ unusedTokenIssues
  where
    definedTokens =
        Set.unions
            [ tokenNames tokenGrammar
            , Set.fromList ["NEWLINE", "EOF"]
            , if tokenGrammarMode tokenGrammar == Just "indentation"
                then Set.fromList ["INDENT", "DEDENT"]
                else Set.empty
            ]
    referencedTokens = tokenReferences parserGrammar
    missingTokenIssues =
        [ "Error: Grammar references token '"
            ++ referenceName
            ++ "' which is not defined in the tokens file"
        | referenceName <- sort (Set.toList referencedTokens)
        , referenceName `Set.notMember` definedTokens
        ]
    unusedTokenIssues =
        [ "Warning: Token '"
            ++ tokenDefinitionName definition
            ++ "' (line "
            ++ show (tokenDefinitionLineNumber definition)
            ++ ") is defined but never used in the grammar"
        | definition <- tokenGrammarDefinitions tokenGrammar
        , not (definitionUsed definition referencedTokens)
        ]

definitionUsed :: TokenDefinition -> Set String -> Bool
definitionUsed definition referencedTokens =
    tokenDefinitionName definition `Set.member` referencedTokens
        || maybe False (`Set.member` referencedTokens) (tokenDefinitionAlias definition)
