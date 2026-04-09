import Foundation

public typealias Bit = Int

public enum FPGAError: Error, Equatable {
    case invalidArgument(String)
    case outOfRange(String)
    case notFound(String)
}
