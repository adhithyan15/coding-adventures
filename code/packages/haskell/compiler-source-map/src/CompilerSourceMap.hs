module CompilerSourceMap
    ( SourcePosition(..)
    , formatSourcePosition
    , SourceToAstEntry(..)
    , SourceToAst(..)
    , emptySourceToAst
    , addSourceToAst
    , lookupSourceByAstNodeId
    , lookupAstNodeIdBySourcePosition
    , AstToIrEntry(..)
    , AstToIr(..)
    , emptyAstToIr
    , addAstToIr
    , lookupIrIdsByAstNodeId
    , lookupAstNodeIdByIrId
    , IrToIrEntry(..)
    , IrToIr(..)
    , emptyIrToIr
    , addIrMapping
    , addIrDeletion
    , lookupNewIrIdsByOriginalId
    , lookupOriginalIrIdByNewId
    , IrToMachineCodeEntry(..)
    , IrToMachineCode(..)
    , emptyIrToMachineCode
    , addIrToMachineCode
    , lookupMachineCodeByIrId
    , lookupIrIdByMachineCodeOffset
    , SourceMapChain(..)
    , emptySourceMapChain
    , addOptimizerPass
    , setIrToMachineCode
    , sourceToMachineCode
    , machineCodeToSource
    ) where

import Data.List (find)
import qualified Data.Set as Set
import Data.Set (Set)

data SourcePosition = SourcePosition
    { sourcePositionFile :: FilePath
    , sourcePositionLine :: Int
    , sourcePositionColumn :: Int
    , sourcePositionLength :: Int
    }
    deriving (Eq, Ord, Show)

formatSourcePosition :: SourcePosition -> String
formatSourcePosition pos =
    sourcePositionFile pos
        ++ ":"
        ++ show (sourcePositionLine pos)
        ++ ":"
        ++ show (sourcePositionColumn pos)
        ++ " (len="
        ++ show (sourcePositionLength pos)
        ++ ")"

data SourceToAstEntry = SourceToAstEntry
    { sourceToAstPosition :: SourcePosition
    , sourceToAstNodeId :: Int
    }
    deriving (Eq, Show)

newtype SourceToAst = SourceToAst
    { sourceToAstEntries :: [SourceToAstEntry]
    }
    deriving (Eq, Show)

emptySourceToAst :: SourceToAst
emptySourceToAst = SourceToAst []

addSourceToAst :: SourcePosition -> Int -> SourceToAst -> SourceToAst
addSourceToAst pos astNodeId (SourceToAst entries) =
    SourceToAst (entries ++ [SourceToAstEntry pos astNodeId])

lookupSourceByAstNodeId :: Int -> SourceToAst -> Maybe SourcePosition
lookupSourceByAstNodeId astNodeId (SourceToAst entries) =
    sourceToAstPosition <$> find ((== astNodeId) . sourceToAstNodeId) entries

lookupAstNodeIdBySourcePosition :: SourcePosition -> SourceToAst -> Maybe Int
lookupAstNodeIdBySourcePosition pos (SourceToAst entries) =
    sourceToAstNodeId <$> find (\entry -> matchesSourcePosition pos (sourceToAstPosition entry)) entries

data AstToIrEntry = AstToIrEntry
    { astToIrNodeId :: Int
    , astToIrIrIds :: [Int]
    }
    deriving (Eq, Show)

newtype AstToIr = AstToIr
    { astToIrEntries :: [AstToIrEntry]
    }
    deriving (Eq, Show)

emptyAstToIr :: AstToIr
emptyAstToIr = AstToIr []

addAstToIr :: Int -> [Int] -> AstToIr -> AstToIr
addAstToIr astNodeId irIds (AstToIr entries) =
    AstToIr (entries ++ [AstToIrEntry astNodeId irIds])

lookupIrIdsByAstNodeId :: Int -> AstToIr -> Maybe [Int]
lookupIrIdsByAstNodeId astNodeId (AstToIr entries) =
    astToIrIrIds <$> find ((== astNodeId) . astToIrNodeId) entries

lookupAstNodeIdByIrId :: Int -> AstToIr -> Maybe Int
lookupAstNodeIdByIrId irId (AstToIr entries) =
    astToIrNodeId <$> find (elem irId . astToIrIrIds) entries

data IrToIrEntry = IrToIrEntry
    { irToIrOriginalId :: Int
    , irToIrNewIds :: [Int]
    }
    deriving (Eq, Show)

data IrToIr = IrToIr
    { irToIrEntries :: [IrToIrEntry]
    , irToIrDeleted :: Set Int
    , irToIrPassName :: String
    }
    deriving (Eq, Show)

emptyIrToIr :: String -> IrToIr
emptyIrToIr passName =
    IrToIr
        { irToIrEntries = []
        , irToIrDeleted = Set.empty
        , irToIrPassName = passName
        }

addIrMapping :: Int -> [Int] -> IrToIr -> IrToIr
addIrMapping originalId newIds segment =
    segment{irToIrEntries = irToIrEntries segment ++ [IrToIrEntry originalId newIds]}

addIrDeletion :: Int -> IrToIr -> IrToIr
addIrDeletion originalId segment =
    segment
        { irToIrEntries = irToIrEntries segment ++ [IrToIrEntry originalId []]
        , irToIrDeleted = Set.insert originalId (irToIrDeleted segment)
        }

lookupNewIrIdsByOriginalId :: Int -> IrToIr -> Maybe [Int]
lookupNewIrIdsByOriginalId originalId segment
    | Set.member originalId (irToIrDeleted segment) = Nothing
    | otherwise =
        irToIrNewIds <$> find ((== originalId) . irToIrOriginalId) (irToIrEntries segment)

lookupOriginalIrIdByNewId :: Int -> IrToIr -> Maybe Int
lookupOriginalIrIdByNewId newId segment =
    irToIrOriginalId <$> find (elem newId . irToIrNewIds) (irToIrEntries segment)

data IrToMachineCodeEntry = IrToMachineCodeEntry
    { irToMachineCodeIrId :: Int
    , irToMachineCodeOffset :: Int
    , irToMachineCodeLength :: Int
    }
    deriving (Eq, Show)

newtype IrToMachineCode = IrToMachineCode
    { irToMachineCodeEntries :: [IrToMachineCodeEntry]
    }
    deriving (Eq, Show)

emptyIrToMachineCode :: IrToMachineCode
emptyIrToMachineCode = IrToMachineCode []

addIrToMachineCode :: Int -> Int -> Int -> IrToMachineCode -> IrToMachineCode
addIrToMachineCode irId offset len (IrToMachineCode entries) =
    IrToMachineCode (entries ++ [IrToMachineCodeEntry irId offset len])

lookupMachineCodeByIrId :: Int -> IrToMachineCode -> Maybe (Int, Int)
lookupMachineCodeByIrId irId (IrToMachineCode entries) =
    fmap
        (\entry -> (irToMachineCodeOffset entry, irToMachineCodeLength entry))
        (find ((== irId) . irToMachineCodeIrId) entries)

lookupIrIdByMachineCodeOffset :: Int -> IrToMachineCode -> Maybe Int
lookupIrIdByMachineCodeOffset offset (IrToMachineCode entries) =
    irToMachineCodeIrId
        <$> find
            (\entry ->
                offset >= irToMachineCodeOffset entry
                    && offset < irToMachineCodeOffset entry + irToMachineCodeLength entry
            )
            entries

data SourceMapChain = SourceMapChain
    { sourceMapChainSourceToAst :: SourceToAst
    , sourceMapChainAstToIr :: AstToIr
    , sourceMapChainIrToIr :: [IrToIr]
    , sourceMapChainIrToMachineCode :: Maybe IrToMachineCode
    }
    deriving (Eq, Show)

emptySourceMapChain :: SourceMapChain
emptySourceMapChain =
    SourceMapChain
        { sourceMapChainSourceToAst = emptySourceToAst
        , sourceMapChainAstToIr = emptyAstToIr
        , sourceMapChainIrToIr = []
        , sourceMapChainIrToMachineCode = Nothing
        }

addOptimizerPass :: IrToIr -> SourceMapChain -> SourceMapChain
addOptimizerPass segment chain =
    chain{sourceMapChainIrToIr = sourceMapChainIrToIr chain ++ [segment]}

setIrToMachineCode :: IrToMachineCode -> SourceMapChain -> SourceMapChain
setIrToMachineCode segment chain =
    chain{sourceMapChainIrToMachineCode = Just segment}

sourceToMachineCode :: SourceMapChain -> SourcePosition -> Maybe [IrToMachineCodeEntry]
sourceToMachineCode chain pos = do
    astNodeId <- lookupAstNodeIdBySourcePosition pos (sourceMapChainSourceToAst chain)
    initialIrIds <- lookupIrIdsByAstNodeId astNodeId (sourceMapChainAstToIr chain)
    let finalIrIds = foldl applyOptimizerPass initialIrIds (sourceMapChainIrToIr chain)
    if null finalIrIds
        then Nothing
        else do
            machineCode <- sourceMapChainIrToMachineCode chain
            let results =
                    [ IrToMachineCodeEntry irId offset len
                    | irId <- finalIrIds
                    , Just (offset, len) <- [lookupMachineCodeByIrId irId machineCode]
                    ]
            if null results then Nothing else Just results

machineCodeToSource :: SourceMapChain -> Int -> Maybe SourcePosition
machineCodeToSource chain offset = do
    machineCode <- sourceMapChainIrToMachineCode chain
    finalIrId <- lookupIrIdByMachineCodeOffset offset machineCode
    originalIrId <- foldr walkPass (Just finalIrId) (sourceMapChainIrToIr chain)
    astNodeId <- lookupAstNodeIdByIrId originalIrId (sourceMapChainAstToIr chain)
    lookupSourceByAstNodeId astNodeId (sourceMapChainSourceToAst chain)

applyOptimizerPass :: [Int] -> IrToIr -> [Int]
applyOptimizerPass currentIds segment =
    concatMap step currentIds
  where
    step irId
        | Set.member irId (irToIrDeleted segment) = []
        | otherwise =
            case lookupNewIrIdsByOriginalId irId segment of
                Just newIds -> newIds
                Nothing -> []

walkPass :: IrToIr -> Maybe Int -> Maybe Int
walkPass _ Nothing = Nothing
walkPass segment (Just irId) = lookupOriginalIrIdByNewId irId segment

matchesSourcePosition :: SourcePosition -> SourcePosition -> Bool
matchesSourcePosition left right =
    sourcePositionFile left == sourcePositionFile right
        && sourcePositionLine left == sourcePositionLine right
        && sourcePositionColumn left == sourcePositionColumn right
