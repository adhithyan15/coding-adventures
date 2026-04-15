import Foundation

public final class RpcServer<Codec: RpcCodec, Framer: RpcFramer> {
    public typealias Value = Codec.Value
    public typealias RequestHandler = (RpcId, Value?) throws -> Value
    public typealias NotificationHandler = (Value?) throws -> Void

    private let codec: Codec
    private let framer: Framer
    private var requestHandlers: [String: RequestHandler] = [:]
    private var notificationHandlers: [String: NotificationHandler] = [:]

    public init(codec: Codec, framer: Framer) {
        self.codec = codec
        self.framer = framer
    }

    @discardableResult
    public func onRequest(_ method: String, handler: @escaping RequestHandler) -> Self {
        requestHandlers[method] = handler
        return self
    }

    @discardableResult
    public func onNotification(_ method: String, handler: @escaping NotificationHandler) -> Self {
        notificationHandlers[method] = handler
        return self
    }

    public func serve() {
        while true {
            let frame: Data?
            do {
                frame = try framer.readFrame()
            } catch let error as RpcErrorResponse<Value> {
                writeError(error)
                continue
            } catch {
                writeError(.internalError(message: "Framing error: \(String(describing: error))"))
                continue
            }

            guard let frame else {
                break
            }

            let message: RpcMessage<Value>
            do {
                message = try codec.decode(frame)
            } catch let error as RpcErrorResponse<Value> {
                writeError(error)
                continue
            } catch {
                writeError(.parseError(message: "Codec error: \(String(describing: error))"))
                continue
            }

            dispatch(message)
        }
    }

    private func dispatch(_ message: RpcMessage<Value>) {
        switch message {
        case .request(let request):
            handleRequest(request)
        case .notification(let notification):
            handleNotification(notification)
        case .response, .errorResponse:
            // Server mode ignores responses.
            break
        }
    }

    private func handleRequest(_ request: RpcRequest<Value>) {
        guard let handler = requestHandlers[request.method] else {
            writeError(.methodNotFound(id: request.id, message: "Method not found: \(request.method)"))
            return
        }

        do {
            let result = try handler(request.id, request.params)
            write(.response(RpcResponse(id: request.id, result: result)))
        } catch let error as RpcErrorResponse<Value> {
            writeError(error.withID(request.id))
        } catch {
            writeError(.internalError(id: request.id, message: "Internal error: \(String(describing: error))"))
        }
    }

    private func handleNotification(_ notification: RpcNotification<Value>) {
        guard let handler = notificationHandlers[notification.method] else {
            return
        }

        do {
            try handler(notification.params)
        } catch {
            // Notifications are fire-and-forget. Errors are swallowed.
        }
    }

    private func write(_ message: RpcMessage<Value>) {
        guard let bytes = try? codec.encode(message) else {
            return
        }

        try? framer.writeFrame(bytes)
    }

    private func writeError(_ error: RpcErrorResponse<Value>) {
        write(.errorResponse(error))
    }
}
