import Foundation

public protocol RpcFramer: AnyObject {
    func readFrame() throws -> Data?
    func writeFrame(_ bytes: Data) throws
}
