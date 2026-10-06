{-# LANGUAGE OverloadedStrings #-}

-- | Process-free graph and changed-path selection. All repository inputs are
-- supplied as inert values; this module never consults the host.
module GraphDiffCore
    ( Edge(..)
    , GraphInput(..)
    , Package(..)
    , DiffInput(..)
    , BoundaryInput(..)
    , BoundaryRule(..)
    , RepositoryBoundary(..)
    , GraphResult(..)
    , DiffResult(..)
    , evaluateGraph
    , evaluateDiffSelection
    ) where

import Control.Monad (foldM, unless, when)
import Data.Aeson (FromJSON(..), ToJSON(..), object, withObject, (.:), (.:?), (.!=), (.=))
import qualified Data.Aeson as Aeson
import Data.Aeson.Types (Parser)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Lazy as BL
import Data.Bits (shiftR)
import Data.Char (isAsciiLower, isDigit, ord)
import Data.List (isPrefixOf, sort)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import qualified Data.Sequence as Seq
import qualified Data.Set as Set
import Data.Set (Set)
import qualified Data.Text.Encoding as TE
import Data.Word (Word8)
import Sha256 (sha256FinalizeHex, sha256Init, sha256Update)
import qualified TrackedArtifactUnicode17 as Unicode

data Edge = Edge String String deriving (Eq, Ord, Show)

instance FromJSON Edge where
    parseJSON value = do
        items <- parseJSON value :: Parser [String]
        case items of
            [a,b] -> pure (Edge a b)
            _ -> fail "edge must be a pair"

instance ToJSON Edge where
    toJSON (Edge a b) = toJSON [a,b]

data GraphInput = GraphInput [String] [Edge]

instance FromJSON GraphInput where
    parseJSON = withObject "graph input" $ \root -> do
        options <- root .: "options"
        GraphInput <$> options .: "packages" <*> options .: "edges"

data Package = Package
    { packageName :: String
    , packageRoot :: String
    , packageMode :: String
    , packageGlobs :: [String]
    }

instance FromJSON Package where
    parseJSON = withObject "package" $ \o -> Package
        <$> o .: "name" <*> o .: "rel_path"
        <*> o .: "source_mode" <*> o .:? "source_globs" .!= []

data RepositoryBoundary = RepositoryBoundary Int String [BoundaryRule]

data BoundaryInput = BoundaryInput String String (Maybe String)

data BoundaryRule = BoundaryRule
    { ruleId :: String
    , ruleOrigin :: String
    , ruleExact :: [String]
    , ruleDescendants :: [String]
    , ruleExcluded :: [String]
    , ruleInputs :: [BoundaryInput]
    , ruleReason :: String
    , ruleOwner :: String
    }

instance FromJSON RepositoryBoundary where
    parseJSON = withObject "repository boundary" $ \o -> RepositoryBoundary
        <$> o .: "schema_version"
        <*> o .: "language_source_input_registry_sha256"
        <*> o .: "boundaries"

instance ToJSON RepositoryBoundary where
    toJSON (RepositoryBoundary version registry rules) = object
        [ "schema_version" .= version
        , "language_source_input_registry_sha256" .= registry
        , "boundaries" .= rules
        ]

instance FromJSON BoundaryInput where
    parseJSON = withObject "boundary input" $ \o -> BoundaryInput
        <$> o .: "path" <*> o .: "role" <*> o .:? "generated_component"

instance ToJSON BoundaryInput where
    toJSON (BoundaryInput path role generated) = object
        (["path" .= path, "role" .= role] ++
            maybe [] (\component -> ["generated_component" .= component]) generated)

instance FromJSON BoundaryRule where
    parseJSON = withObject "boundary rule" $ \o -> do
        applies <- o .: "applies_to"
        BoundaryRule <$> o .: "id" <*> o .: "input_origin"
            <*> applies .: "exact_roots"
            <*> applies .: "descendant_roots"
            <*> applies .: "excluded_roots"
            <*> o .: "inputs" <*> o .: "reason" <*> o .: "owner"

instance ToJSON BoundaryRule where
    toJSON rule = object
        [ "id" .= ruleId rule
        , "input_origin" .= ruleOrigin rule
        , "applies_to" .= object
            [ "exact_roots" .= ruleExact rule
            , "descendant_roots" .= ruleDescendants rule
            , "excluded_roots" .= ruleExcluded rule
            ]
        , "inputs" .= ruleInputs rule
        , "reason" .= ruleReason rule
        , "owner" .= ruleOwner rule
        ]

data DiffInput = DiffInput
    { diffPackages :: [Package]
    , diffEdges :: [Edge]
    , diffForced :: [String]
    , diffPolicy :: String
    , diffChangedPaths :: [String]
    , diffBoundarySha256 :: Maybe String
    , diffBoundary :: Maybe RepositoryBoundary
    }

instance FromJSON DiffInput where
    parseJSON = withObject "diff input" $ \root -> do
        options <- root .: "options"
        DiffInput <$> options .: "packages" <*> options .: "edges"
            <*> options .: "forced_packages" <*> options .: "unknown_path_policy"
            <*> root .: "changed_paths" <*> options .:? "boundary_sha256"
            <*> pure Nothing

data GraphResult = GraphResult [Edge] [[String]] deriving (Eq, Show)

instance ToJSON GraphResult where
    toJSON (GraphResult edges levels) = object
        ["edges" .= edges, "levels" .= levels]

data DiffResult = DiffResult [String] [String] [String] deriving (Eq, Show)

instance ToJSON DiffResult where
    toJSON (DiffResult changed affected prerequisites) = object
        [ "changed_packages" .= changed
        , "affected_packages" .= affected
        , "prerequisite_packages" .= prerequisites
        ]

data Graph = Graph
    { graphNames :: Set String
    , graphEdges :: [Edge]
    , graphDependents :: Map String [String]
    , graphPrerequisites :: Map String [String]
    }

evaluateGraph :: GraphInput -> Either String GraphResult
evaluateGraph (GraphInput names edges) = do
    graph <- validateGraph names edges
    levels <- maybe (Left "GRAPH_CYCLE") Right (graphLevels graph)
    pure (GraphResult (graphEdges graph) levels)

evaluateDiffSelection :: DiffInput -> Either String DiffResult
evaluateDiffSelection input = do
    let packages = diffPackages input
    graph <- validateGraph (map packageName packages) (diffEdges input)
    unless (graphLevels graph /= Nothing) (Left "DIFF_EDGE_CYCLE")
    validateDiff input
    boundaryConsumers <- boundaryIndex input
    preflightMatchWork packages (diffChangedPaths input)
    let (selected, unknown) = foldl selectPath (Set.fromList (diffForced input), False)
            (diffChangedPaths input)
        selectPath (acc, priorUnknown) path =
            let registered = Map.findWithDefault Set.empty path boundaryConsumers
                matching = [packageName pkg | pkg <- packages, inside path (packageRoot pkg)]
                selectedPackages = [packageName pkg | pkg <- packages
                    , inside path (packageRoot pkg)
                    , let relative = relativePath path (packageRoot pkg)
                    , packageMode pkg == "package_prefix" || buildFront relative ||
                        any (`matchPath` relative) (packageGlobs pkg)]
            in (Set.unions [acc, registered, Set.fromList selectedPackages],
                priorUnknown || (Set.null registered && null matching))
    when (unknown && diffPolicy input == "error") (Left "DIFF_UNKNOWN_PATH")
    let changed = if unknown then graphNames graph else selected
        affected = closure changed (graphDependents graph)
        prerequisites = closure affected (graphPrerequisites graph) `Set.difference` affected
    pure (DiffResult (Set.toAscList changed) (Set.toAscList affected)
        (Set.toAscList prerequisites))

preflightMatchWork :: [Package] -> [String] -> Either String ()
preflightMatchWork packages paths = do
    _ <- foldM chargePackage 0 packages
    pure ()
  where
    chargePackage total pkg
        | packageMode pkg /= "strict_globs" = Right total
        | otherwise =
            let factor = sum [toInteger (length glob) + 1 | glob <- packageGlobs pkg]
            in foldM (chargePath pkg factor) total paths
    chargePath pkg factor total path
        | not (inside path (packageRoot pkg)) = Right total
        | buildFront relative = Right total
        | otherwise =
            let next = total + factor * (toInteger (length relative) + 1)
            in if next > 50000000 then Left "DIFF_MATCH_LIMIT_EXCEEDED"
                else Right next
      where
        relative = relativePath path (packageRoot pkg)

validateGraph :: [String] -> [Edge] -> Either String Graph
validateGraph names edges = do
    when (length names > 4096) (Left "GRAPH_PACKAGE_LIMIT_EXCEEDED")
    when (length edges > 16384) (Left "GRAPH_EDGE_LIMIT_EXCEEDED")
    unless (all validPackageName names) (Left "GRAPH_PACKAGE_INVALID")
    let nameSet = Set.fromList names
    when (Set.size nameSet /= length names) (Left "GRAPH_PACKAGE_DUPLICATE")
    let endpoints (Edge a b) = Set.member a nameSet && Set.member b nameSet
    unless (all endpoints edges) (Left "GRAPH_EDGE_UNKNOWN")
    when (any (\(Edge a b) -> a == b) edges) (Left "GRAPH_EDGE_SELF")
    when (Set.size (Set.fromList edges) /= length edges) (Left "GRAPH_EDGE_DUPLICATE")
    let emptyAdj = Map.fromSet (const []) nameSet
        dependents = foldl (\m (Edge a b) -> Map.adjust (b:) a m) emptyAdj edges
        prerequisites = foldl (\m (Edge a b) -> Map.adjust (a:) b m) emptyAdj edges
    pure (Graph nameSet (sort edges) (Map.map sort dependents) (Map.map sort prerequisites))

graphLevels :: Graph -> Maybe [[String]]
graphLevels graph = go initialDegrees initialReady 0 []
  where
    initialDegrees = Map.map length (graphPrerequisites graph)
    initialReady = [n | (n,d) <- Map.toAscList initialDegrees, d == 0]
    go degrees ready visited levels
        | null ready = if visited == Set.size (graphNames graph)
            then Just (reverse levels) else Nothing
        | otherwise =
            let (nextDegrees, nextReady) = foldl visit (degrees, []) ready
                visit (ds, newlyReady) name = foldl decrement (ds, newlyReady)
                    (Map.findWithDefault [] name (graphDependents graph))
                decrement (ds, newlyReady) name =
                    let degree = Map.findWithDefault 0 name ds - 1
                    in (Map.insert name degree ds,
                        if degree == 0 then name:newlyReady else newlyReady)
            in go nextDegrees (sort nextReady) (visited + length ready) (ready:levels)

closure :: Set String -> Map String [String] -> Set String
closure seeds adjacency = go seeds (Set.toList seeds)
  where
    go seen [] = seen
    go seen (name:rest) =
        let new = Set.fromList (Map.findWithDefault [] name adjacency) `Set.difference` seen
        in go (Set.union seen new) (Set.toList new ++ rest)

validateDiff :: DiffInput -> Either String ()
validateDiff input = do
    let packages = diffPackages input
        roots = map packageRoot packages
        identities = map (Unicode.casefold . Unicode.nfc) roots
        overlaps a b = a == b || (a ++ "/") `isPrefixOf` b || (b ++ "/") `isPrefixOf` a
        rootPairs = [(a,b) | (i,a) <- zip [0::Int ..] identities,
            (j,b) <- zip [0::Int ..] identities, i < j]
    unless (all validPath roots && not (any (uncurry overlaps) rootPairs))
        (Left "DIFF_PATH_INVALID")
    unless (all (\p -> packageMode p `elem` ["package_prefix", "strict_globs"]) packages)
        (Left "DIFF_SOURCE_MODE_INVALID")
    unless (all validGlobs packages) (Left "DIFF_GLOB_INVALID")
    unless (diffPolicy input `elem` ["all", "error"]) (Left "DIFF_POLICY_INVALID")
    unless (length (diffForced input) <= 4096 && unique (diffForced input))
        (Left "DIFF_FORCED_PACKAGE_INVALID")
    unless (length (diffChangedPaths input) <= 4096 &&
        unique (diffChangedPaths input) && all validPath (diffChangedPaths input))
        (Left "DIFF_PATH_INVALID")
    unless (all (`elem` map packageName packages) (diffForced input))
        (Left "DIFF_FORCED_PACKAGE_UNKNOWN")
  where
    validGlobs pkg = length (packageGlobs pkg) <= 256 && unique (packageGlobs pkg)
        && all (\glob -> validGlob glob && validGlobPattern glob) (packageGlobs pkg)
        && (packageMode pkg == "strict_globs" || null (packageGlobs pkg))

unique :: Ord a => [a] -> Bool
unique values = Set.size (Set.fromList values) == length values

validPackageName :: String -> Bool
validPackageName value = length value <= 240 && case split '/' value of
    (_:_:_) -> all validSegment (split '/' value)
    _ -> False
  where
    validSegment [] = False
    validSegment (first:rest) = lowerOrDigit first && all validTail rest
    lowerOrDigit c = isAsciiLower c || isDigit c && ord c < 128
    validTail c = lowerOrDigit c || c `elem` ['.','_','-']

validPath :: String -> Bool
validPath value = validPathShape value
    && all (`notElem` ['<','>',':','"','|','?','*']) value
    && all (not . reservedSegment) (split '/' value)

validGlob :: String -> Bool
validGlob value = validPathShape value
    && all (`notElem` ['<','>',':','"','|','?']) value
    && all (\segment -> any (`elem` ['*','[',']','{','}']) segment ||
        not (reservedSegment segment)) (split '/' value)

validPathShape :: String -> Bool
validPathShape value = not (null value) && length value <= 512 && head value /= '/'
    && '\\' `notElem` value && all (\c -> ord c >= 32 && not (surrogate c)) value
    && Unicode.nfc value == value
    && all segmentOK (split '/' value)
  where
    segmentOK segment = not (null segment) && segment `notElem` [".",".."]
        && last segment `notElem` [' ','.']
    surrogate c = ord c >= 0xd800 && ord c <= 0xdfff

reservedSegment :: String -> Bool
reservedSegment segment = Unicode.fullUppercase (takeWhile (/= '.') segment)
    `Set.member` reservedNames
  where
    reservedNames = Set.fromList
        (["CON","PRN","AUX","NUL","CONIN$","CONOUT$","CLOCK$"] ++
         [prefix ++ show number | prefix <- ["COM","LPT"], number <- [1::Int .. 9]] ++
         [prefix ++ suffix | prefix <- ["COM","LPT"], suffix <- ["¹","²","³"]])

validGlobPattern :: String -> Bool
validGlobPattern = go
  where
    go [] = True
    go ('[':rest) = case characterClassMembers rest of
        Nothing -> go rest -- an unmatched opening bracket is literal
        Just (members,remaining) -> not (ambiguousPair members)
            && validMembers members && go remaining
    go (_:rest) = go rest
    validMembers [] = True
    validMembers (a:'-':b:rest) = a <= b && validMembers rest
    validMembers (_:rest) = validMembers rest
    ambiguousPair (a:b:rest) =
        (a == b && a `elem` ['-','&','~','|']) || ambiguousPair (b:rest)
    ambiguousPair _ = False

characterClassMembers :: String -> Maybe (String,String)
characterClassMembers input =
    let body = case input of
            ('!':rest) -> rest
            _ -> input
        search = case body of
            (']':rest) -> let (more,remaining) = break (== ']') rest
                          in (']':more,remaining)
            _ -> break (== ']') body
    in case search of
        (members,_:remaining) -> Just (members,remaining)
        _ -> Nothing

split :: Char -> String -> [String]
split delimiter value = case break (== delimiter) value of
    (first, []) -> [first]
    (first, _:rest) -> first : split delimiter rest

inside :: String -> String -> Bool
inside path root = path == root || (root ++ "/") `isPrefixOf` path

relativePath :: String -> String -> String
relativePath path root = if path == root then "" else drop (length root + 1) path

buildFront :: String -> Bool
buildFront path = last (split '/' path) `elem`
    ["BUILD", "BUILD_windows", "BUILD_mac", "BUILD_linux", "BUILD_mac_and_linux"]

boundaryIndex :: DiffInput -> Either String (Map String (Set String))
boundaryIndex input = case (diffBoundarySha256 input, diffBoundary input) of
    (Nothing, Nothing) -> Right Map.empty
    (Just "", Nothing) -> Right Map.empty
    (Just wanted, Just boundary@(RepositoryBoundary _ _ rules)) -> do
        unless (length wanted == 64 && all (`elem` ['0'..'9'] ++ ['a'..'f']) wanted &&
            wanted == boundaryDigest boundary) (Left "DIFF_BOUNDARY_DIGEST_MISMATCH")
        pure (foldl addPackage Map.empty (diffPackages input))
      where
        addPackage index pkg = foldl (addRule pkg) index rules
        addRule pkg index rule
            | packageRoot pkg `elem` ruleExact rule ||
                (packageRoot pkg `notElem` ruleExcluded rule &&
                    any (\root -> (root ++ "/") `isPrefixOf` packageRoot pkg)
                        (ruleDescendants rule)) =
                foldl (\m (BoundaryInput path _ _) -> Map.insertWith Set.union path
                    (Set.singleton (packageName pkg)) m) index (ruleInputs rule)
            | otherwise = index
    _ -> Left "DIFF_BOUNDARY_DIGEST_MISMATCH"

boundaryDigest :: RepositoryBoundary -> String
boundaryDigest boundary =
    let encoded = BL.toStrict (Aeson.encode boundary)
        domain = TE.encodeUtf8 "coding-adventures/build-tool-repository-source-input-boundary/v1\0"
        count = toInteger (BS.length encoded)
        lengthBytes = BS.pack [fromInteger (count `shiftR` (8 * i)) :: Word8 | i <- [7,6..0]]
    in sha256FinalizeHex (sha256Update (sha256Update
        (sha256Update sha256Init domain) lengthBytes) encoded)

-- Memo tables keep both path and segment matching bounded by input lengths.
matchPath :: String -> String -> Bool
matchPath patternValue path = fst (go Map.empty 0 0)
  where
    patterns = Seq.fromList (split '/' patternValue)
    values = Seq.fromList (if null path then [] else split '/' path)
    go memo i j = case Map.lookup (i,j) memo of
        Just answer -> (answer,memo)
        Nothing ->
            let (answer, nextMemo) =
                    if i == Seq.length patterns then (j == Seq.length values,memo)
                    else if Seq.index patterns i == "**" then
                        let (zero,m1) = go memo (i+1) j
                        in if zero then (True,m1) else if j < Seq.length values
                            then go m1 i (j+1) else (False,m1)
                    else if j < Seq.length values &&
                        matchSegment (Seq.index patterns i) (Seq.index values j)
                        then go memo (i+1) (j+1) else (False,memo)
            in (answer, Map.insert (i,j) answer nextMemo)

matchSegment :: String -> String -> Bool
matchSegment patternValue value = fst (go Map.empty 0 0)
  where
    patterns = Seq.fromList patternValue
    values = Seq.fromList value
    classes = Map.fromList [(i, characterClass (drop i patternValue))
        | (i,c) <- zip [0::Int ..] patternValue, c == '[']
    go memo i j = case Map.lookup (i,j) memo of
        Just answer -> (answer,memo)
        Nothing ->
            let (answer,m1) = case Seq.lookup i patterns of
                    Nothing -> (j == Seq.length values,memo)
                    Just '*' ->
                        let (zero,m2) = go memo (i+1) j
                        in if zero then (True,m2) else if j < Seq.length values
                            then go m2 i (j+1) else (False,m2)
                    Just '?' -> if j < Seq.length values then go memo (i+1) (j+1)
                        else (False,memo)
                    Just '[' -> case Map.findWithDefault Nothing i classes of
                        Just (matches,consumed) | j < Seq.length values &&
                            matches (Seq.index values j) ->
                            go memo (i+consumed) (j+1)
                        Just _ -> (False,memo)
                        Nothing -> literal '['
                    Just c -> literal c
                  where
                    literal c = if j < Seq.length values && Seq.index values j == c
                        then go memo (i+1) (j+1) else (False,memo)
            in (answer, Map.insert (i,j) answer m1)

characterClass :: String -> Maybe (Char -> Bool, Int)
characterClass ('[':rest) =
    let (negated,body) = case rest of
            ('!':remaining) -> (True,remaining)
            _ -> (False,rest)
        (members,closing) = break (== ']') body
        (members',closing') = if null members && not (null closing)
            then let (extra,tailValue) = break (== ']') (tail closing)
                 in (']':extra,tailValue)
            else (members,closing)
    in case closing' of
        [] -> Nothing
        _ -> Just (\c -> let found = member c members'
                          in if negated then not found else found,
            2 + length members' + if negated then 1 else 0)
  where
    member c (a:'-':b:more) = (a <= c && c <= b) || member c more
    member c (a:more) = c == a || member c more
    member _ [] = False
characterClass _ = Nothing
