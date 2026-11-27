{-# LANGUAGE OverloadedStrings #-}
-- | In-memory inode tree, independent descriptor offsets, kernel events, and shell.
module UnixHistory (Inode(..), Kernel, newKernel, resolve, openFile, readFileAt, closeFile, listDirectory, runCommand) where
import Data.Aeson (Value(..), object, (.=))
import qualified Data.Aeson.Key as K
import qualified Data.Aeson.KeyMap as KM
import qualified Data.ByteString as BS
import qualified Data.Map.Strict as M
import Data.Maybe (fromMaybe)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import qualified Data.Vector as V

data Inode = Inode { inodeNumber :: !Int, inodeData :: !(Maybe BS.ByteString), inodeEntries :: ![(String, Int)] } deriving (Eq, Show)
data Descriptor = Descriptor !Int !Int deriving (Eq, Show)
data Kernel = Kernel { image :: !(M.Map Int Inode), slots :: !(M.Map Int Descriptor), capacity :: !Int }
newKernel :: Kernel
newKernel = Kernel (M.fromList [(1,Inode 1 Nothing [("hello",2),("empty",3),("etc",4)]),(2,Inode 2 (Just "Hello, Unix!\n") []),(3,Inode 3 (Just BS.empty) []),(4,Inode 4 Nothing [("notes",5)]),(5,Inode 5 (Just "A directory maps a name to an inode.\nAn open descriptor tracks a byte offset.\n") [])]) M.empty 4

resolve :: Kernel -> String -> Either String Inode
resolve _ path | null path || '\0' `elem` path = Left "EINVAL"
resolve k path = do
  (node,_) <- foldl step (Right (root,[])) (filter (not . null) (split path))
  if last path == '/' && inodeData node /= Nothing then Left "ENOTDIR" else Right node
 where
  root = fromMaybe (error "root inode invariant") (M.lookup 1 (image k))
  step (Left e) _ = Left e
  step (Right (node,parents)) name | inodeData node /= Nothing = Left "ENOTDIR"
    | name == "." = Right (node,parents)
    | name == ".." = case parents of p:ps -> Right (p,ps); [] -> Right (root,[])
    | otherwise = case lookup name (inodeEntries node) >>= (`M.lookup` image k) of Nothing -> Left "ENOENT"; Just child -> Right (child,node:parents)
  split []=[]
  split xs=let (a,b)=break (=='/') xs in a:case b of []->[]; _:rest->split rest

openFile :: Kernel -> String -> (Kernel,Value)
openFile k path=case resolve k path of
 Left e -> (k,event ["call".=("open"::String),"path".=path,"error".=e])
 Right node | inodeData node==Nothing -> (k,event ["call".=("open"::String),"path".=path,"error".=("EISDIR"::String)])
 Right node -> case findFree [0..capacity k-1] of Nothing -> (k,event ["call".=("open"::String),"path".=path,"error".=("EMFILE"::String)]); Just fd -> (k{slots=M.insert fd (Descriptor (inodeNumber node) 0) (slots k)},event ["call".=("open"::String),"path".=path,"inode".=inodeNumber node,"fd".=fd,"offset".=(0::Int)])
 where findFree []=Nothing; findFree (x:xs)=if M.member x (slots k) then findFree xs else Just x
readFileAt :: Kernel -> Int -> Int -> (Kernel,Value,BS.ByteString)
readFileAt k fd count=case M.lookup fd (slots k) of
 Nothing -> (k,event ["call".=("read"::String),"fd".=fd,"error".=("EBADF"::String)],BS.empty)
 Just _ | count<0 -> (k,event ["call".=("read"::String),"fd".=fd,"error".=("EINVAL"::String)],BS.empty)
 Just (Descriptor ino off) -> let bytes=fromMaybe BS.empty (inodeData=<<M.lookup ino (image k)); chunk=BS.take count (BS.drop off bytes); next=off+BS.length chunk; k'=k{slots=M.insert fd (Descriptor ino next) (slots k)} in (k',event ["call".=("read"::String),"fd".=fd,"inode".=ino,"offset".=off,"bytes".=BS.length chunk,"nextOffset".=next],chunk)
closeFile :: Kernel -> Int -> (Kernel,Value)
closeFile k fd=if M.member fd (slots k) then (k{slots=M.delete fd (slots k)},event ["call".=("close"::String),"fd".=fd]) else (k,event ["call".=("close"::String),"fd".=fd,"error".=("EBADF"::String)])
listDirectory :: Kernel -> String -> Value
listDirectory k path=case resolve k path of Left e->event ["call".=("list"::String),"path".=path,"error".=e]; Right node->case inodeData node of Just _->event ["call".=("list"::String),"path".=path,"error".=("ENOTDIR"::String)]; Nothing->event ["call".=("list"::String),"path".=path,"inode".=inodeNumber node,"names".=map fst (inodeEntries node)]
runCommand :: String -> Value
runCommand cmd =
  let (out, evs) = case asciiWords cmd of
        ["cat", p] -> cat p
        ["ls"] -> ls "/"
        ["ls", p] -> ls p
        [] -> ("", [])
        _ -> ("usage: cat PATH or ls [PATH]\n", [])
      errors = concat ["error: " ++ e ++ "\n" | v <- evs, Just e <- [fieldString "error" v]]
  in event ["output" .= (out ++ errors), "events" .= evs]
 where
  ls p = let e = listDirectory newKernel p in (names e, [e])
  cat p = let (k, e) = openFile newKernel p in case fieldInt "fd" e of
    Nothing -> ("", [e])
    Just fd -> let (k', chunks, readEvents) = drain k fd [] []
                   (_, closed) = closeFile k' fd
               in (T.unpack (TE.decodeUtf8 (BS.concat (reverse chunks))), e : readEvents ++ [closed])
  drain k fd chunks evs = let (k', e, b) = readFileAt k fd 8
                          in if BS.null b then (k', chunks, evs ++ [e]) else drain k' fd (b:chunks) (evs ++ [e])
  names v = case KM.lookup "names" (obj v) of
    Just (Array a) -> concat [T.unpack t ++ "\n" | String t <- V.toList a]
    _ -> ""

event :: [(K.Key, Value)] -> Value
event = object
obj :: Value -> KM.KeyMap Value
obj (Object x) = x
obj _ = KM.empty
fieldString :: String -> Value -> Maybe String
fieldString key v = case KM.lookup (K.fromString key) (obj v) of Just (String s) -> Just (T.unpack s); _ -> Nothing
fieldInt :: String -> Value -> Maybe Int
fieldInt key v = case KM.lookup (K.fromString key) (obj v) of Just (Number n) -> Just (round n); _ -> Nothing

-- Only the six ASCII whitespace bytes delimit shell words in the portable model.
asciiWords :: String -> [String]
asciiWords input = case dropWhile asciiSpace input of
  [] -> []
  remaining -> let (word, suffix) = span (not . asciiSpace) remaining
               in word : asciiWords suffix
 where
  asciiSpace :: Char -> Bool
  asciiSpace c = c `elem` (" \t\n\r\f\v" :: String)
