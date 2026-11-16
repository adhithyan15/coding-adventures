// ============================================================================
// Constants.swift -- Canonical CBOR Wire Constants & Limits
// ============================================================================

public enum Constants {
  // Protocol limits
  public static let maxNestingDepth: Int = 128
  public static let maxEncodedBytes: Int = 1_048_576
  public static let rootDepth: Int = 0
  public static let initialMapDepth: Int = 1

  // Allocation & buffer capacity hints
  public static let defaultBufferCapacity: Int = 64
  public static let keyBufferCapacity: Int = 32
  public static let collectionInitialCapacity: Int = 1024

  // Allocation & progress bounds
  public static let minBytesPerRawByte: UInt64 = 1
  public static let minBytesPerArrayItem: UInt64 = 1
  public static let minBytesPerMapEntry: UInt64 = 2
  public static let minBytesPerKey: UInt64 = 1
  public static let minBytesPerValue: UInt64 = 1

  // Major types (bits 7..5)
  public static let majorUnsigned: UInt8 = 0
  public static let majorNegative: UInt8 = 1
  public static let majorBytes: UInt8 = 2
  public static let majorText: UInt8 = 3
  public static let majorArray: UInt8 = 4
  public static let majorMap: UInt8 = 5
  public static let majorTag: UInt8 = 6
  public static let majorSimple: UInt8 = 7

  // Additional information codes
  public static let aiDirectMax: UInt8 = 23
  public static let aiOneByte: UInt8 = 24
  public static let aiTwoBytes: UInt8 = 25
  public static let aiFourBytes: UInt8 = 26
  public static let aiEightBytes: UInt8 = 27
  public static let aiFloat16: UInt8 = 25
  public static let aiFloat32: UInt8 = 26
  public static let aiFloat64: UInt8 = 27
  public static let aiReserved28: UInt8 = 28
  public static let aiReserved29: UInt8 = 29
  public static let aiReserved30: UInt8 = 30
  public static let aiIndefinite: UInt8 = 31

  // Simple values
  public static let simpleFalse: UInt8 = 20
  public static let simpleTrue: UInt8 = 21
  public static let simpleNull: UInt8 = 22

  // Bit manipulation
  public static let majorTypeShift: Int = 5
  public static let majorTypeMask: UInt8 = 0xE0
  public static let additionalInfoMask: UInt8 = 0x1F
  public static let byteMask: UInt64 = 0xFF

  // Fixed wire bytes
  public static let wireByteFalse: UInt8 = 0xF4
  public static let wireByteTrue: UInt8 = 0xF5
  public static let wireByteNull: UInt8 = 0xF6
  public static let wireByteEmptyMap: UInt8 = 0xA0
  public static let wireByteEmptyArray: UInt8 = 0x80

  // Integer boundary constants
  public static let maxUint8: UInt64 = 0xFF
  public static let maxUint16: UInt64 = 0xFFFF
  public static let maxUint32: UInt64 = 0xFFFF_FFFF
  public static let maxUint64: UInt64 = 0xFFFF_FFFF_FFFF_FFFF

  // Bit shifts
  public static let shift8: Int = 8
  public static let shift16: Int = 16
  public static let shift24: Int = 24
  public static let shift32: Int = 32
  public static let shift40: Int = 40
  public static let shift48: Int = 48
  public static let shift56: Int = 56

  // Argument byte lengths
  public static let argBytes2: Int = 2
  public static let argBytes4: Int = 4
  public static let argBytes8: Int = 8

  // Header total lengths
  public static let headerLenDirect: Int = 1
  public static let headerLen1Byte: Int = 2
  public static let headerLen2Bytes: Int = 3
  public static let headerLen4Bytes: Int = 5
  public static let headerLen8Bytes: Int = 9
}
