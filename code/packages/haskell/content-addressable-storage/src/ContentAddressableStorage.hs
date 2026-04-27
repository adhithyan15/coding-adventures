module ContentAddressableStorage
    ( Key
    , keyToHex
    , hexToKey
    , sha1Key
    , BlobStore(..)
    , CasError(..)
    , ContentAddressableStore
    , newContentAddressableStore
    , MemoryStore
    , newMemoryStore
    , LocalDiskStore
    , newLocalDiskStore
    , putContent
    , getContent
    , existsContent
    , findByPrefix
    ) where

import Control.Exception (IOException, try)
import Data.Bits
    ( rotateL
    , shiftL
    , shiftR
    , xor
    , (.&.)
    , (.|.)
    )
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BSC
import qualified Data.IORef as IORef
import qualified Data.List as List
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.Maybe (mapMaybe)
import Data.Word (Word32, Word64, Word8)
import Numeric (showHex)
import System.Directory
    ( createDirectoryIfMissing
    , doesDirectoryExist
    , doesFileExist
    , listDirectory
    )
import System.FilePath ((</>))

newtype Key = Key BS.ByteString
    deriving (Eq, Ord)

instance Show Key where
    show = keyToHex

keyToHex :: Key -> String
keyToHex (Key bytes) =
    concatMap toHexPair (BS.unpack bytes)
  where
    toHexPair byte =
        let hexDigits = showHex byte ""
         in replicate (2 - length hexDigits) '0' ++ hexDigits

hexToKey :: String -> Either String Key
hexToKey hexValue
    | length hexValue /= 40 = Left ("expected 40 hex chars, got " ++ show (length hexValue))
    | otherwise = Key . BS.pack <$> go hexValue
  where
    go [] = Right []
    go (left : right : rest) = do
        hi <- fromHexDigit left
        lo <- fromHexDigit right
        let byte = fromIntegral (hi * 16 + lo)
        (byte :) <$> go rest
    go _ = Left "expected an even number of hex digits"

fromHexDigit :: Char -> Either String Int
fromHexDigit charValue
    | charValue >= '0' && charValue <= '9' = Right (fromEnum charValue - fromEnum '0')
    | charValue >= 'a' && charValue <= 'f' = Right (10 + fromEnum charValue - fromEnum 'a')
    | charValue >= 'A' && charValue <= 'F' = Right (10 + fromEnum charValue - fromEnum 'A')
    | otherwise = Left ("invalid hex character: " ++ [charValue])

sha1Key :: BS.ByteString -> Key
sha1Key input =
    Key . BS.pack . concatMap word32ToBytes $ finalState
  where
    finalState = foldl processChunk initialState (chunkify 64 (padSha1 input))
    initialState =
        [ 0x67452301
        , 0xEFCDAB89
        , 0x98BADCFE
        , 0x10325476
        , 0xC3D2E1F0
        ]

data CasError
    = CasStoreError String
    | CasNotFound Key
    | CasCorrupted Key
    | CasAmbiguousPrefix String
    | CasPrefixNotFound String
    | CasInvalidPrefix String
    deriving (Eq, Show)

class BlobStore store where
    putBlob :: store -> Key -> BS.ByteString -> IO (Either String ())
    getBlob :: store -> Key -> IO (Either String (Maybe BS.ByteString))
    existsBlob :: store -> Key -> IO (Either String Bool)
    keysWithPrefix :: store -> String -> IO (Either String [Key])

newtype ContentAddressableStore store = ContentAddressableStore store

newContentAddressableStore :: store -> ContentAddressableStore store
newContentAddressableStore = ContentAddressableStore

data MemoryStore = MemoryStore (IORef.IORef (Map Key BS.ByteString))

newMemoryStore :: IO MemoryStore
newMemoryStore =
    MemoryStore <$> IORef.newIORef Map.empty

instance BlobStore MemoryStore where
    putBlob (MemoryStore ref) key bytes = do
        IORef.modifyIORef' ref (Map.insert key bytes)
        pure (Right ())

    getBlob (MemoryStore ref) key = do
        store <- IORef.readIORef ref
        pure (Right (Map.lookup key store))

    existsBlob (MemoryStore ref) key = do
        store <- IORef.readIORef ref
        pure (Right (Map.member key store))

    keysWithPrefix (MemoryStore ref) prefix = do
        store <- IORef.readIORef ref
        pure (Right [key | key <- Map.keys store, prefix `List.isPrefixOf` keyToHex key])

newtype LocalDiskStore = LocalDiskStore FilePath

newLocalDiskStore :: FilePath -> IO LocalDiskStore
newLocalDiskStore root = do
    createDirectoryIfMissing True root
    pure (LocalDiskStore root)

instance BlobStore LocalDiskStore where
    putBlob (LocalDiskStore root) key bytes =
        wrapIoError $ do
            let path = pathForKey root key
            createDirectoryIfMissing True (keyDirectory root key)
            BS.writeFile path bytes

    getBlob (LocalDiskStore root) key =
        wrapIoError $ do
            let path = pathForKey root key
            present <- doesFileExist path
            if present
                then Just <$> BS.readFile path
                else pure Nothing

    existsBlob (LocalDiskStore root) key =
        wrapIoError (doesFileExist (pathForKey root key))

    keysWithPrefix (LocalDiskStore root) prefix =
        wrapIoError (discoverMatchingKeys root prefix)

putContent :: BlobStore store => ContentAddressableStore store -> BS.ByteString -> IO (Either CasError Key)
putContent (ContentAddressableStore store) bytes = do
    let key = sha1Key bytes
    result <- putBlob store key bytes
    pure $
        case result of
            Left err -> Left (CasStoreError err)
            Right () -> Right key

getContent :: BlobStore store => ContentAddressableStore store -> Key -> IO (Either CasError BS.ByteString)
getContent (ContentAddressableStore store) key = do
    result <- getBlob store key
    pure $
        case result of
            Left err -> Left (CasStoreError err)
            Right Nothing -> Left (CasNotFound key)
            Right (Just bytes) ->
                if sha1Key bytes == key
                    then Right bytes
                    else Left (CasCorrupted key)

existsContent :: BlobStore store => ContentAddressableStore store -> Key -> IO (Either CasError Bool)
existsContent (ContentAddressableStore store) key = do
    result <- existsBlob store key
    pure $
        case result of
            Left err -> Left (CasStoreError err)
            Right present -> Right present

findByPrefix :: BlobStore store => ContentAddressableStore store -> String -> IO (Either CasError Key)
findByPrefix (ContentAddressableStore store) prefix =
    case validatePrefix prefix of
        Left err -> pure (Left err)
        Right normalizedPrefix -> do
            result <- keysWithPrefix store normalizedPrefix
            pure $
                case result of
                    Left err -> Left (CasStoreError err)
                    Right [] -> Left (CasPrefixNotFound normalizedPrefix)
                    Right [key] -> Right key
                    Right _ -> Left (CasAmbiguousPrefix normalizedPrefix)

validatePrefix :: String -> Either CasError String
validatePrefix prefix
    | null prefix = Left (CasInvalidPrefix prefix)
    | any (not . isHexChar) prefix = Left (CasInvalidPrefix prefix)
    | otherwise = Right prefix

isHexChar :: Char -> Bool
isHexChar charValue =
    (charValue >= '0' && charValue <= '9')
        || (charValue >= 'a' && charValue <= 'f')
        || (charValue >= 'A' && charValue <= 'F')

pathForKey :: FilePath -> Key -> FilePath
pathForKey root key =
    keyDirectory root key </> drop 2 (keyToHex key)

keyDirectory :: FilePath -> Key -> FilePath
keyDirectory root key =
    root </> take 2 (keyToHex key)

discoverMatchingKeys :: FilePath -> String -> IO [Key]
discoverMatchingKeys root prefix = do
    rootExists <- doesDirectoryExist root
    if not rootExists
        then pure []
        else do
            directoryNames <- candidateDirectories root prefix
            fmap concat $
                mapM (discoverKeysInDirectory root prefix) directoryNames

candidateDirectories :: FilePath -> String -> IO [FilePath]
candidateDirectories root prefix
    | length prefix < 2 = do
        entries <- listDirectory root
        pure [entry | entry <- entries, prefix `List.isPrefixOf` entry]
    | otherwise = do
        let dirName = take 2 prefix
        exists <- doesDirectoryExist (root </> dirName)
        pure [dirName | exists]

discoverKeysInDirectory :: FilePath -> String -> FilePath -> IO [Key]
discoverKeysInDirectory root prefix dirName = do
    fileNames <- listDirectory (root </> dirName)
    pure $
        mapMaybe
            (\fileName ->
                let candidate = dirName ++ fileName
                 in if length candidate == 40
                        && prefix `List.isPrefixOf` candidate
                        then either (const Nothing) Just (hexToKey candidate)
                        else Nothing
            )
            fileNames

wrapIoError :: IO a -> IO (Either String a)
wrapIoError action = do
    result <- try action
    pure $
        case result of
            Left err -> Left (show (err :: IOException))
            Right value -> Right value

padSha1 :: BS.ByteString -> BS.ByteString
padSha1 input =
    input <> BS.pack [0x80] <> BS.replicate paddingLength 0 <> BS.pack (word64ToBytes messageLengthBits)
  where
    messageLengthBits =
        fromIntegral (BS.length input) * 8 :: Word64
    remainder =
        (BS.length input + 1 + 8) `mod` 64
    paddingLength =
        if remainder == 0 then 0 else 64 - remainder

chunkify :: Int -> BS.ByteString -> [BS.ByteString]
chunkify chunkSize bytes
    | BS.null bytes = []
    | otherwise =
        let (current, rest) = BS.splitAt chunkSize bytes
         in current : chunkify chunkSize rest

processChunk :: [Word32] -> BS.ByteString -> [Word32]
processChunk [h0, h1, h2, h3, h4] chunk =
    [ h0 + aFinal
    , h1 + bFinal
    , h2 + cFinal
    , h3 + dFinal
    , h4 + eFinal
    ]
  where
    schedule = expandSchedule (map bytesToWord32 (chunkify 4 chunk))
    (aFinal, bFinal, cFinal, dFinal, eFinal) =
        foldl step (h0, h1, h2, h3, h4) (zip [0 :: Int .. 79] schedule)

    step (a, b, c, d, e) (index, wordValue) =
        let (f, k)
                | index <= 19 = ((b .&. c) .|. (complement32 b .&. d), 0x5A827999)
                | index <= 39 = (b `xor` c `xor` d, 0x6ED9EBA1)
                | index <= 59 = ((b .&. c) .|. (b .&. d) .|. (c .&. d), 0x8F1BBCDC)
                | otherwise = (b `xor` c `xor` d, 0xCA62C1D6)
            temp = rotateL a 5 + f + e + k + wordValue
         in (temp, a, rotateL b 30, c, d)
processChunk state _ = state

expandSchedule :: [Word32] -> [Word32]
expandSchedule initialWords =
    initialWords ++ map buildWord [16 .. 79]
  where
    buildWord index =
        rotateL
            ( schedule !! (index - 3)
                `xor` schedule !! (index - 8)
                `xor` schedule !! (index - 14)
                `xor` schedule !! (index - 16)
            )
            1
    schedule = expandSchedule initialWords

complement32 :: Word32 -> Word32
complement32 value =
    value `xor` 0xFFFFFFFF

bytesToWord32 :: BS.ByteString -> Word32
bytesToWord32 bytes =
    foldl (\acc byte -> shiftL acc 8 .|. fromIntegral byte) 0 (BS.unpack bytes)

word32ToBytes :: Word32 -> [Word8]
word32ToBytes value =
    [ fromIntegral (shiftR value 24 .&. 0xFF)
    , fromIntegral (shiftR value 16 .&. 0xFF)
    , fromIntegral (shiftR value 8 .&. 0xFF)
    , fromIntegral (value .&. 0xFF)
    ]

word64ToBytes :: Word64 -> [Word8]
word64ToBytes value =
    [ fromIntegral (shiftR value 56 .&. 0xFF)
    , fromIntegral (shiftR value 48 .&. 0xFF)
    , fromIntegral (shiftR value 40 .&. 0xFF)
    , fromIntegral (shiftR value 32 .&. 0xFF)
    , fromIntegral (shiftR value 24 .&. 0xFF)
    , fromIntegral (shiftR value 16 .&. 0xFF)
    , fromIntegral (shiftR value 8 .&. 0xFF)
    , fromIntegral (value .&. 0xFF)
    ]
