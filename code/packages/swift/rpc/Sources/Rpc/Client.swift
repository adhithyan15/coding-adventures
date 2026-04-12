import Foundation

public final class RpcClient<Codec: RpcCodec, Framer: RpcFramer> {
    public typealias Value = Codec.Value
    public typealias NotificationHandler = (Value?) -> Void

    private let codec: Codec
    private let framer: Framer
    private var nextRequestID: Int
    private var notificationHandlers: [String: NotificationHandler] = [:]

    public init(codec: Codec, framer: Framer, nextRequestID: Int = 1) {
        self.codec = codec
        self.framer = framer
        self.nextRequestID = nextRequestID
    }

    @discardableResult
    public func onNotification(_ method: String, handler: @escaping NotificationHandler) -> Self {
        notificationHandlers[method] = handler
        return self
    }

    public func request(_ method: String, params: Value? = nil) throws -> Value {
        let id: RpcId = .integer(nextRequestID)
        nextRequestID += 1

        try send(.request(RpcRequest(id: id, method: method, params: params)), requestID: id, context: "request")

        while true {
            let frame: Data?
            do {
                frame = try framer.readFrame()
            } catch let error as RpcErrorResponse<Value> {
                throw error
            } catch {
                throw RpcErrorResponse<Value>.internalError(id: id, message: "Framing error: \(String(describing: error))")
            }

            guard let frame else {
                throw RpcErrorResponse<Value>.internalError(id: id, message: "Connection closed before response")
            }

            let message: RpcMessage<Value>
            do {
                message = try codec.decode(frame)
            } catch let error as RpcErrorResponse<Value> {
                throw error
            } catch {
                throw RpcErrorResponse<Value>.internalError(id: id, message: "Codec error: \(String(describing: error))")
            }

            switch message {
            case .notification(let notification):
                handleNotification(notification)

            case .response(let response):
                if response.id == id {
                    return response.result
                }

            case .errorResponse(let error):
                if error.id == nil || error.id == id {
                    throw error
                }

            case .request:
                // Bidirectional requests are outside this package's minimal
                // client contract, so they are ignored.
                continue
            }
        }
    }

    @discardableResult
    public func notify(_ method: String, params: Value? = nil) throws -> Self {
        try send(.notification(RpcNotification(method: method, params: params)), requestID: nil, context: "notification")
        return self
    }

    private func send(_ message: RpcMessage<Value>, requestID: RpcId?, context: String) throws {
        do {
            let bytes = try codec.encode(message)
            try framer.writeFrame(bytes)
        } catch let error as RpcErrorResponse<Value> {
            throw error
        } catch {
            throw RpcErrorResponse<Value>.internalError(id: requestID, message: "Failed to send \(context): \(String(describing: error))")
        }
    }

    private func handleNotification(_ notification: RpcNotification<Value>) {
        guard let handler = notificationHandlers[notification.method] else {
            return
        }

        handler(notification.params)
    }
}
