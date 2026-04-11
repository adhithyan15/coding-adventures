import Foundation

public typealias Bit = Int

public enum BlockRAMError: Error, Equatable {
    case invalidArgument(String)
    case outOfRange(String)
    case writeCollision(address: Int)
}
