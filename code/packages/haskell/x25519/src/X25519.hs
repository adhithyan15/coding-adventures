module X25519
    ( description
    , x25519
    , x25519Base
    , generateKeypair
    ) where

import Control.Exception (IOException, bracket, catch)
import qualified Data.ByteString as BS
import Data.Word (Word8)
import System.Directory (findExecutable, getTemporaryDirectory, removeFile)
import System.Exit (ExitCode(..))
import System.IO (hClose, openBinaryTempFile)
import System.IO.Error (catchIOError)
import System.Process (readProcessWithExitCode)

description :: String
description = "X25519 key agreement (RFC 7748) backed by the local OpenSSL toolchain"

x25519 :: [Word8] -> [Word8] -> IO (Either String [Word8])
x25519 privateKey publicKey
    | length privateKey /= 32 =
        pure (Left ("X25519 private key must be 32 bytes, got " ++ show (length privateKey)))
    | length publicKey /= 32 =
        pure (Left ("X25519 public key must be 32 bytes, got " ++ show (length publicKey)))
    | otherwise =
        withTempBinaryFile "x25519-private-" (x25519PrivatePrefix <> privateKey) $ \privatePath ->
            withTempBinaryFile "x25519-public-" (x25519PublicPrefix <> publicKey) $ \publicPath ->
                withEmptyTempFile "x25519-secret-" $ \secretPath -> do
                    runResult <-
                        runOpenSSL
                            [ "pkeyutl"
                            , "-derive"
                            , "-inkey", privatePath
                            , "-keyform", "DER"
                            , "-peerkey", publicPath
                            , "-peerform", "DER"
                            , "-out", secretPath
                            ]
                    case runResult of
                        Left err -> pure (Left err)
                        Right () -> do
                            secret <- BS.unpack <$> BS.readFile secretPath
                            pure $
                                if length secret == 32
                                    then Right secret
                                    else Left ("OpenSSL returned " ++ show (length secret) ++ " bytes for X25519 shared secret")

x25519Base :: [Word8] -> IO (Either String [Word8])
x25519Base privateKey
    | length privateKey /= 32 =
        pure (Left ("X25519 private key must be 32 bytes, got " ++ show (length privateKey)))
    | otherwise =
        withTempBinaryFile "x25519-private-" (x25519PrivatePrefix <> privateKey) $ \privatePath ->
            withEmptyTempFile "x25519-public-der-" $ \publicPath -> do
                runResult <-
                    runOpenSSL
                        [ "pkey"
                        , "-inform", "DER"
                        , "-in", privatePath
                        , "-outform", "DER"
                        , "-pubout"
                        , "-out", publicPath
                        ]
                case runResult of
                    Left err -> pure (Left err)
                    Right () -> do
                        publicDer <- BS.unpack <$> BS.readFile publicPath
                        pure (extractPrefixed x25519PublicPrefix publicDer "X25519 public key")

generateKeypair :: [Word8] -> IO (Either String [Word8])
generateKeypair = x25519Base

x25519PrivatePrefix :: [Word8]
x25519PrivatePrefix = hexToBytes "302e020100300506032b656e04220420"

x25519PublicPrefix :: [Word8]
x25519PublicPrefix = hexToBytes "302a300506032b656e032100"

extractPrefixed :: [Word8] -> [Word8] -> String -> Either String [Word8]
extractPrefixed prefixValue encoded label
    | take (length prefixValue) encoded /= prefixValue =
        Left ("Unexpected OpenSSL DER output for " ++ label)
    | otherwise =
        Right (drop (length prefixValue) encoded)

runOpenSSL :: [String] -> IO (Either String ())
runOpenSSL arguments = do
    maybePath <- findExecutable "openssl"
    case maybePath of
        Nothing -> pure (Left "openssl executable was not found on PATH")
        Just executablePath -> do
            result <-
                catch
                    (do
                        (exitCode, _, stderrOutput) <- readProcessWithExitCode executablePath arguments ""
                        pure (Right (exitCode, stderrOutput)))
                    (\err -> pure (Left (show (err :: IOException))))
            pure $
                case result of
                    Left err -> Left err
                    Right (ExitSuccess, _) -> Right ()
                    Right (ExitFailure _, stderrOutput) ->
                        Left ("OpenSSL failed: " ++ trim stderrOutput)

withTempBinaryFile :: String -> [Word8] -> (FilePath -> IO a) -> IO a
withTempBinaryFile template contents =
    bracket create cleanup
  where
    create = do
        tempDirectory <- getTemporaryDirectory
        (path, handle) <- openBinaryTempFile tempDirectory template
        BS.hPut handle (BS.pack contents)
        hClose handle
        pure path
    cleanup path = ignoreIo (removeFile path)

withEmptyTempFile :: String -> (FilePath -> IO a) -> IO a
withEmptyTempFile template =
    bracket create cleanup
  where
    create = do
        tempDirectory <- getTemporaryDirectory
        (path, handle) <- openBinaryTempFile tempDirectory template
        hClose handle
        pure path
    cleanup path = ignoreIo (removeFile path)

ignoreIo :: IO () -> IO ()
ignoreIo action =
    catchIOError action (\_ -> pure ())

trim :: String -> String
trim =
    reverse . dropWhile (`elem` ['\n', '\r', ' ']) . reverse

hexToBytes :: String -> [Word8]
hexToBytes [] = []
hexToBytes (a : b : rest) = fromIntegral (hexDigit a * 16 + hexDigit b) : hexToBytes rest
hexToBytes _ = []

hexDigit :: Char -> Int
hexDigit character
    | character >= '0' && character <= '9' = fromEnum character - fromEnum '0'
    | character >= 'a' && character <= 'f' = 10 + fromEnum character - fromEnum 'a'
    | character >= 'A' && character <= 'F' = 10 + fromEnum character - fromEnum 'A'
    | otherwise = 0
