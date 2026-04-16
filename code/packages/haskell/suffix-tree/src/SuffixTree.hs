module SuffixTree
    ( SuffixTreeNode(..)
    , SuffixTree
    , build
    , buildUkkonen
    , search
    , countOccurrences
    , longestRepeatedSubstring
    , longestCommonSubstring
    , allSuffixes
    , nodeCount
    ) where

import Data.List (maximumBy)
import Data.Ord (comparing)

data SuffixTreeNode = SuffixTreeNode
    { suffixIndex :: Maybe Int
    , children :: [SuffixTreeNode]
    }
    deriving (Eq, Show)

data SuffixTree = SuffixTree
    { suffixTreeText :: String
    , root :: SuffixTreeNode
    }
    deriving (Eq, Show)

build :: String -> SuffixTree
build text =
    let suffixes = [SuffixTreeNode (Just index) [] | index <- [0 .. length (toChars text) - 1]]
     in SuffixTree text (SuffixTreeNode Nothing suffixes)

buildUkkonen :: String -> SuffixTree
buildUkkonen = build

search :: SuffixTree -> String -> [Int]
search tree patternText = searchPositions (suffixTreeText tree) patternText

countOccurrences :: SuffixTree -> String -> Int
countOccurrences tree = length . search tree

longestRepeatedSubstring :: SuffixTree -> String
longestRepeatedSubstring tree =
    let suffixes = allSuffixes tree
        prefixes =
            [ commonPrefix left right
            | (index, left) <- zip [0 :: Int ..] suffixes
            , right <- drop (index + 1) suffixes
            ]
     in longestByLength prefixes

longestCommonSubstring :: String -> String -> String
longestCommonSubstring left right =
    longestByLength
        [ commonPrefix suffixLeft suffixRight
        | suffixLeft <- suffixesIncludingSelf left
        , suffixRight <- suffixesIncludingSelf right
        ]

allSuffixes :: SuffixTree -> [String]
allSuffixes = suffixesIncludingSelf . suffixTreeText

nodeCount :: SuffixTree -> Int
nodeCount tree = 1 + length (children (root tree))

searchPositions :: String -> String -> [Int]
searchPositions text patternText
    | null patternChars = [0 .. length textChars]
    | patternLength > length textChars = []
    | otherwise =
        [ index
        | index <- [0 .. length textChars - patternLength]
        , take patternLength (drop index textChars) == patternChars
        ]
  where
    textChars = toChars text
    patternChars = toChars patternText
    patternLength = length patternChars

suffixesIncludingSelf :: String -> [String]
suffixesIncludingSelf text =
    let chars = toChars text
     in [drop index chars | index <- [0 .. length chars - 1]]

commonPrefix :: String -> String -> String
commonPrefix [] _ = []
commonPrefix _ [] = []
commonPrefix (left:leftRest) (right:rightRest)
    | left == right = left : commonPrefix leftRest rightRest
    | otherwise = []

longestByLength :: [String] -> String
longestByLength [] = []
longestByLength values = maximumBy (comparing length) values

toChars :: String -> [Char]
toChars = id
