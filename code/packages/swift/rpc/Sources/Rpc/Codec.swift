import Foundation

public protocol RpcCodec {
    associatedtype Value: Sendable

    func encode(_ message: RpcMessage<Value>) throws -> Data
    func decode(_ bytes: Data) throws -> RpcMessage<Value>
}
