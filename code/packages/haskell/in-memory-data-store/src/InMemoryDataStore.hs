module InMemoryDataStore
    ( description
    , DataStoreManager
    , newDataStoreManager
    , startBackgroundWorkers
    , stopBackgroundWorkers
    , executeManager
    , executeRespValue
    , executeRespBytes
    , encodeResponses
    , module InMemoryDataStoreEngine
    , module InMemoryDataStoreProtocol
    ) where

import qualified Control.Concurrent as Concurrent
import qualified Control.Concurrent.MVar as MVar
import qualified Data.ByteString as BS
import InMemoryDataStoreEngine hiding (description)
import InMemoryDataStoreProtocol hiding (description)
import RespProtocol (RespDecodeError(..), RespEncodeError, RespValue, decodeAll, encode)

description :: String
description = "Haskell in-memory data store composition package without TCP"

data DataStoreManager = DataStoreManager
    { managerEngine :: DataStoreEngine
    , managerStopFlag :: MVar.MVar Bool
    , managerWorker :: MVar.MVar (Maybe Concurrent.ThreadId)
    }

newDataStoreManager :: Maybe FilePath -> IO DataStoreManager
newDataStoreManager maybeAofPath = do
    engine <- newDataStoreEngine maybeAofPath
    stopFlag <- MVar.newMVar False
    workerVar <- MVar.newMVar Nothing
    pure
        DataStoreManager
            { managerEngine = engine
            , managerStopFlag = stopFlag
            , managerWorker = workerVar
            }

startBackgroundWorkers :: DataStoreManager -> IO ()
startBackgroundWorkers manager =
    MVar.modifyMVar_
        (managerWorker manager)
        (\maybeWorker ->
            case maybeWorker of
                Just workerId -> pure (Just workerId)
                Nothing -> do
                    MVar.swapMVar (managerStopFlag manager) False
                    workerId <- Concurrent.forkIO (expiryLoop manager)
                    pure (Just workerId))

stopBackgroundWorkers :: DataStoreManager -> IO ()
stopBackgroundWorkers manager = do
    MVar.swapMVar (managerStopFlag manager) True
    MVar.modifyMVar_
        (managerWorker manager)
        (\maybeWorker ->
            case maybeWorker of
                Nothing -> pure Nothing
                Just workerId -> do
                    Concurrent.killThread workerId
                    pure Nothing)

executeManager :: DataStoreManager -> Int -> CommandFrame -> IO (Int, EngineResponse)
executeManager manager = executeWithDb (managerEngine manager)

executeRespValue :: DataStoreManager -> Int -> RespValue -> IO (Int, EngineResponse)
executeRespValue manager selectedDb respValue =
    case commandFrameFromResp respValue of
        Nothing ->
            pure
                ( selectedDb
                , EngineError "ERR protocol error: expected array of bulk strings"
                )
        Just commandFrameValue ->
            executeManager manager selectedDb commandFrameValue

executeRespBytes :: DataStoreManager -> Int -> BS.ByteString -> IO (Int, [EngineResponse])
executeRespBytes manager selectedDb bytesValue =
    case decodeAll bytesValue of
        Left decodeError ->
            pure (selectedDb, [EngineError ("ERR " ++ respDecodeErrorMessage decodeError)])
        Right (respValues, _) ->
            foldl
                executeOne
                (pure (selectedDb, []))
                respValues
  where
    executeOne ioState respValue = do
        (currentDb, responses) <- ioState
        (nextDb, response) <- executeRespValue manager currentDb respValue
        pure (nextDb, responses ++ [response])

encodeResponses :: [EngineResponse] -> Either RespEncodeError BS.ByteString
encodeResponses responses =
    BS.concat <$> traverse (encode . engineResponseToResp) responses

expiryLoop :: DataStoreManager -> IO ()
expiryLoop manager = do
    shouldStop <- MVar.readMVar (managerStopFlag manager)
    if shouldStop
        then pure ()
        else do
            Concurrent.threadDelay 100000
            activeExpireAll (managerEngine manager)
            expiryLoop manager
