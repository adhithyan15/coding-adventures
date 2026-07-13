module HuffmanTree
    ( WeightPair (..)
    , HuffmanTree
    , build
    , codeTable
    , codeFor
    , canonicalCodeTable
    , decodeAll
    , weight
    , depth
    , symbolCount
    , leaves
    , isValid
    ) where

import Data.Bits (shiftL)
import Data.Char (intToDigit)
import Data.List (find, sortOn)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Set (Set)
import qualified Data.Set as Set
import qualified Heap
import Numeric (showIntAtBase)

-- | A symbol and its positive occurrence frequency.
data WeightPair = WeightPair
    { symbol :: Int
    , frequency :: Int
    }
    deriving (Eq, Show)

-- | A symbol-bearing leaf or a weighted internal node.
data Node
    = Leaf Int Int
    | Internal Int Node Node Int
    deriving (Eq, Show)

-- | The tree root and the number of leaves supplied at construction time.
data HuffmanTree = HuffmanTree Node Int
    deriving (Eq, Show)

-- The heap compares queue entries only by their deterministic priority key.
-- Node payloads are deliberately excluded from comparison.
data QueueItem = QueueItem (Int, Int, Int, Int) Node
    deriving (Show)

instance Eq QueueItem where
    QueueItem keyA _ == QueueItem keyB _ = keyA == keyB

instance Ord QueueItem where
    compare (QueueItem keyA _) (QueueItem keyB _) = compare keyA keyB

-- | Construct a deterministic Huffman tree from symbol-frequency pairs.
--
-- Equal-weight nodes use the cross-language tie-breaking contract:
-- leaves precede internal nodes, lower symbols precede higher symbols, and
-- internal nodes retain creation order.
build :: [WeightPair] -> Either String HuffmanTree
build [] = Left "weights must not be empty"
build pairs =
    case find ((<= 0) . frequency) pairs of
        Just pair ->
            Left
                ( "frequency must be positive; got symbol="
                    ++ show (symbol pair)
                    ++ ", freq="
                    ++ show (frequency pair)
                )
        Nothing -> combine initialQueue 0
  where
    initialQueue =
        Heap.fromList
            [ let node = Leaf (symbol pair) (frequency pair)
               in QueueItem (priority node) node
            | pair <- pairs
            ]
    symbolTotal = length pairs

    combine queue order
        | Heap.size queue == 1 = do
            (root, _) <- takeMinimum queue
            Right (HuffmanTree root symbolTotal)
        | otherwise = do
            (leftNode, afterLeft) <- takeMinimum queue
            (rightNode, afterRight) <- takeMinimum afterLeft
            let combinedWeight = nodeWeight leftNode + nodeWeight rightNode
                parent = Internal combinedWeight leftNode rightNode order
                nextQueue = Heap.push (QueueItem (priority parent) parent) afterRight
            combine nextQueue (order + 1)

-- | Return the ordinary tree-walk code for every symbol.
codeTable :: HuffmanTree -> Map Int String
codeTable (HuffmanTree root _) = Map.fromList (walkCodes root "")

-- | Find one symbol's ordinary tree-walk code.
codeFor :: HuffmanTree -> Int -> Maybe String
codeFor (HuffmanTree root _) target = findCode root target ""

-- | Return canonical Huffman codes sorted by code length and symbol value.
canonicalCodeTable :: HuffmanTree -> Map Int String
canonicalCodeTable (HuffmanTree root _) =
    case sortedLengths of
        [] -> Map.empty
        [(leafSymbol, _)] -> Map.singleton leafSymbol "0"
        (_, firstLength) : _ ->
            Map.fromList (assignCodes 0 firstLength sortedLengths)
  where
    lengths = Map.fromList (collectLengths root 0)
    sortedLengths = sortOn (\(leafSymbol, codeLength) -> (codeLength, leafSymbol)) (Map.toList lengths)

-- | Decode exactly the requested number of symbols from an ordinary code stream.
decodeAll :: HuffmanTree -> String -> Int -> Either String [Int]
decodeAll _ _ count | count <= 0 = Right []
decodeAll (HuffmanTree root _) bits count = go root bits []
  where
    singleLeaf = isLeaf root

    go _ _ decoded | length decoded == count = Right (reverse decoded)
    go node remaining decoded =
        case node of
            Leaf leafSymbol _ ->
                let rest =
                        if singleLeaf
                            then case remaining of
                                [] -> []
                                _ : more -> more
                            else remaining
                 in go root rest (leafSymbol : decoded)
            Internal _ leftNode rightNode _ ->
                case remaining of
                    [] ->
                        Left
                            ( "Bit stream exhausted after "
                                ++ show (length decoded)
                                ++ " symbols; expected "
                                ++ show count
                            )
                    bit : more ->
                        go (if bit == '0' then leftNode else rightNode) more decoded

-- | The sum of all input frequencies.
weight :: HuffmanTree -> Int
weight (HuffmanTree root _) = nodeWeight root

-- | The maximum number of edges from root to leaf.
depth :: HuffmanTree -> Int
depth (HuffmanTree root _) = maximumDepth root 0

-- | The number of symbol-frequency pairs supplied to 'build'.
symbolCount :: HuffmanTree -> Int
symbolCount (HuffmanTree _ count) = count

-- | Return leaves in left-to-right order with ordinary tree-walk codes.
leaves :: HuffmanTree -> [(Int, String)]
leaves (HuffmanTree root _) = walkCodes root ""

-- | Check weight and symbol-uniqueness invariants.
isValid :: HuffmanTree -> Bool
isValid (HuffmanTree root _) = fst (checkNode root Set.empty)

priority :: Node -> (Int, Int, Int, Int)
priority (Leaf leafSymbol leafWeight) = (leafWeight, 0, leafSymbol, -1)
priority (Internal internalWeight _ _ order) = (internalWeight, 1, -1, order)

nodeWeight :: Node -> Int
nodeWeight (Leaf _ leafWeight) = leafWeight
nodeWeight (Internal internalWeight _ _ _) = internalWeight

takeMinimum :: Heap.MinHeap QueueItem -> Either String (Node, Heap.MinHeap QueueItem)
takeMinimum queue =
    case Heap.minView queue of
        Nothing -> Left "internal error: Huffman priority queue is empty"
        Just (QueueItem _ node, rest) -> Right (node, rest)

walkCodes :: Node -> String -> [(Int, String)]
walkCodes (Leaf leafSymbol _) prefix = [(leafSymbol, nonEmptyCode prefix)]
walkCodes (Internal _ leftNode rightNode _) prefix =
    walkCodes leftNode (prefix ++ "0") ++ walkCodes rightNode (prefix ++ "1")

findCode :: Node -> Int -> String -> Maybe String
findCode (Leaf leafSymbol _) target prefix
    | leafSymbol == target = Just (nonEmptyCode prefix)
    | otherwise = Nothing
findCode (Internal _ leftNode rightNode _) target prefix =
    case findCode leftNode target (prefix ++ "0") of
        Just result -> Just result
        Nothing -> findCode rightNode target (prefix ++ "1")

nonEmptyCode :: String -> String
nonEmptyCode "" = "0"
nonEmptyCode code = code

collectLengths :: Node -> Int -> [(Int, Int)]
collectLengths (Leaf leafSymbol _) currentDepth =
    [(leafSymbol, max 1 currentDepth)]
collectLengths (Internal _ leftNode rightNode _) currentDepth =
    collectLengths leftNode (currentDepth + 1)
        ++ collectLengths rightNode (currentDepth + 1)

assignCodes :: Integer -> Int -> [(Int, Int)] -> [(Int, String)]
assignCodes _ _ [] = []
assignCodes current previousLength ((leafSymbol, codeLength) : rest) =
    let shifted =
            if codeLength > previousLength
                then current `shiftL` (codeLength - previousLength)
                else current
        code = leftPad codeLength (toBinary shifted)
     in (leafSymbol, code) : assignCodes (shifted + 1) codeLength rest

toBinary :: Integer -> String
toBinary 0 = "0"
toBinary value = showIntAtBase 2 intToDigit value ""

leftPad :: Int -> String -> String
leftPad width value = replicate (max 0 (width - length value)) '0' ++ value

maximumDepth :: Node -> Int -> Int
maximumDepth (Leaf _ _) currentDepth = currentDepth
maximumDepth (Internal _ leftNode rightNode _) currentDepth =
    max
        (maximumDepth leftNode (currentDepth + 1))
        (maximumDepth rightNode (currentDepth + 1))

isLeaf :: Node -> Bool
isLeaf Leaf {} = True
isLeaf Internal {} = False

checkNode :: Node -> Set Int -> (Bool, Set Int)
checkNode (Leaf leafSymbol _) seen
    | Set.member leafSymbol seen = (False, seen)
    | otherwise = (True, Set.insert leafSymbol seen)
checkNode (Internal internalWeight leftNode rightNode _) seen
    | internalWeight /= nodeWeight leftNode + nodeWeight rightNode = (False, seen)
    | otherwise =
        let (leftValid, afterLeft) = checkNode leftNode seen
            (rightValid, afterRight) = checkNode rightNode afterLeft
         in (leftValid && rightValid, afterRight)
