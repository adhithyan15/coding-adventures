module CodingAdventures.CanonicalCbor.Constants
  ( maxNestingDepth
  , maxEncodedBytes
  , rootDepth
  , initialMapDepth
  , minBytesPerRawByte
  , minBytesPerArrayItem
  , minBytesPerMapEntry
  , minBytesPerKey
  , minBytesPerValue
  , majorUnsigned
  , majorNegative
  , majorBytes
  , majorText
  , majorArray
  , majorMap
  , majorTag
  , majorSimple
  , aiDirectMax
  , aiOneByte
  , aiTwoBytes
  , aiFourBytes
  , aiEightBytes
  , aiFloat16
  , aiFloat32
  , aiFloat64
  , aiReserved28
  , aiReserved29
  , aiReserved30
  , aiIndefinite
  , simpleFalse
  , simpleTrue
  , simpleNull
  , majorTypeShift
  , majorTypeMask
  , additionalInfoMask
  , wireByteFalse
  , wireByteTrue
  , wireByteNull
  , wireByteEmptyMap
  , wireByteEmptyArray
  , maxUint8
  , maxUint16
  , maxUint32
  , maxUint64
  , shiftByte1
  , shiftByte2
  , shiftByte3
  , shiftByte4
  , shiftByte5
  , shiftByte6
  , shiftByte7
  , argBytes2
  , argBytes4
  , argBytes8
  , headerLenDirect
  , headerLen1Byte
  , headerLen2Bytes
  , headerLen4Bytes
  , headerLen8Bytes
  ) where

import Data.Word (Word8, Word64)

maxNestingDepth :: Int
maxNestingDepth = 128

maxEncodedBytes :: Word64
maxEncodedBytes = 1048576

rootDepth :: Int
rootDepth = 0

initialMapDepth :: Int
initialMapDepth = 1

minBytesPerRawByte :: Word64
minBytesPerRawByte = 1

minBytesPerArrayItem :: Word64
minBytesPerArrayItem = 1

minBytesPerMapEntry :: Word64
minBytesPerMapEntry = 2

minBytesPerKey :: Word64
minBytesPerKey = 1

minBytesPerValue :: Word64
minBytesPerValue = 1

majorUnsigned :: Word8
majorUnsigned = 0

majorNegative :: Word8
majorNegative = 1

majorBytes :: Word8
majorBytes = 2

majorText :: Word8
majorText = 3

majorArray :: Word8
majorArray = 4

majorMap :: Word8
majorMap = 5

majorTag :: Word8
majorTag = 6

majorSimple :: Word8
majorSimple = 7

aiDirectMax :: Word8
aiDirectMax = 23

aiOneByte :: Word8
aiOneByte = 24

aiTwoBytes :: Word8
aiTwoBytes = 25

aiFourBytes :: Word8
aiFourBytes = 26

aiEightBytes :: Word8
aiEightBytes = 27

aiFloat16 :: Word8
aiFloat16 = 25

aiFloat32 :: Word8
aiFloat32 = 26

aiFloat64 :: Word8
aiFloat64 = 27

aiReserved28 :: Word8
aiReserved28 = 28

aiReserved29 :: Word8
aiReserved29 = 29

aiReserved30 :: Word8
aiReserved30 = 30

aiIndefinite :: Word8
aiIndefinite = 31

simpleFalse :: Word8
simpleFalse = 20

simpleTrue :: Word8
simpleTrue = 21

simpleNull :: Word8
simpleNull = 22

majorTypeShift :: Int
majorTypeShift = 5

majorTypeMask :: Word8
majorTypeMask = 0xE0

additionalInfoMask :: Word8
additionalInfoMask = 0x1F

wireByteFalse :: Word8
wireByteFalse = 0xF4

wireByteTrue :: Word8
wireByteTrue = 0xF5

wireByteNull :: Word8
wireByteNull = 0xF6

wireByteEmptyMap :: Word8
wireByteEmptyMap = 0xA0

wireByteEmptyArray :: Word8
wireByteEmptyArray = 0x80

maxUint8 :: Word64
maxUint8 = 0xFF

maxUint16 :: Word64
maxUint16 = 0xFFFF

maxUint32 :: Word64
maxUint32 = 0xFFFFFFFF

maxUint64 :: Word64
maxUint64 = 0xFFFFFFFFFFFFFFFF

shiftByte1 :: Int
shiftByte1 = 8

shiftByte2 :: Int
shiftByte2 = 16

shiftByte3 :: Int
shiftByte3 = 24

shiftByte4 :: Int
shiftByte4 = 32

shiftByte5 :: Int
shiftByte5 = 40

shiftByte6 :: Int
shiftByte6 = 48

shiftByte7 :: Int
shiftByte7 = 56

argBytes2 :: Word64
argBytes2 = 2

argBytes4 :: Word64
argBytes4 = 4

argBytes8 :: Word64
argBytes8 = 8

headerLenDirect :: Word64
headerLenDirect = 1

headerLen1Byte :: Word64
headerLen1Byte = 2

headerLen2Bytes :: Word64
headerLen2Bytes = 3

headerLen4Bytes :: Word64
headerLen4Bytes = 5

headerLen8Bytes :: Word64
headerLen8Bytes = 9
