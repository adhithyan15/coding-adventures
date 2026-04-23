public enum RpcId: Sendable, Hashable, ExpressibleByIntegerLiteral, ExpressibleByStringLiteral, CustomStringConvertible {
    case integer(Int)
    case string(String)

    public init(_ value: Int) {
        self = .integer(value)
    }

    public init(_ value: String) {
        self = .string(value)
    }

    public init(integerLiteral value: Int) {
        self = .integer(value)
    }

    public init(stringLiteral value: String) {
        self = .string(value)
    }

    public init(extendedGraphemeClusterLiteral value: String) {
        self = .string(value)
    }

    public init(unicodeScalarLiteral value: String) {
        self = .string(value)
    }

    public var description: String {
        switch self {
        case .integer(let value):
            return String(value)
        case .string(let value):
            return value
        }
    }
}

public struct RpcRequest<Value: Sendable>: Sendable {
    public let id: RpcId
    public let method: String
    public let params: Value?

    public init(id: RpcId, method: String, params: Value? = nil) {
        self.id = id
        self.method = method
        self.params = params
    }
}

public struct RpcResponse<Value: Sendable>: Sendable {
    public let id: RpcId
    public let result: Value

    public init(id: RpcId, result: Value) {
        self.id = id
        self.result = result
    }
}

public struct RpcErrorResponse<Value: Sendable>: Error, Sendable {
    public let id: RpcId?
    public let code: Int
    public let message: String
    public let data: Value?

    public init(id: RpcId? = nil, code: Int, message: String, data: Value? = nil) {
        self.id = id
        self.code = code
        self.message = message
        self.data = data
    }
}

public struct RpcNotification<Value: Sendable>: Sendable {
    public let method: String
    public let params: Value?

    public init(method: String, params: Value? = nil) {
        self.method = method
        self.params = params
    }
}

public enum RpcMessage<Value: Sendable>: Sendable {
    case request(RpcRequest<Value>)
    case response(RpcResponse<Value>)
    case errorResponse(RpcErrorResponse<Value>)
    case notification(RpcNotification<Value>)
}

public extension RpcErrorResponse {
    static func parseError(id: RpcId? = nil, message: String = "Parse error", data: Value? = nil) -> Self {
        Self(id: id, code: RpcErrorCodes.parseError, message: message, data: data)
    }

    static func invalidRequest(id: RpcId? = nil, message: String = "Invalid request", data: Value? = nil) -> Self {
        Self(id: id, code: RpcErrorCodes.invalidRequest, message: message, data: data)
    }

    static func methodNotFound(id: RpcId? = nil, message: String = "Method not found", data: Value? = nil) -> Self {
        Self(id: id, code: RpcErrorCodes.methodNotFound, message: message, data: data)
    }

    static func invalidParams(id: RpcId? = nil, message: String = "Invalid params", data: Value? = nil) -> Self {
        Self(id: id, code: RpcErrorCodes.invalidParams, message: message, data: data)
    }

    static func internalError(id: RpcId? = nil, message: String = "Internal error", data: Value? = nil) -> Self {
        Self(id: id, code: RpcErrorCodes.internalError, message: message, data: data)
    }

    func withID(_ id: RpcId) -> Self {
        Self(id: id, code: code, message: message, data: data)
    }
}

extension RpcMessage: Equatable where Value: Equatable {}

extension RpcRequest: Equatable where Value: Equatable {}
extension RpcResponse: Equatable where Value: Equatable {}
extension RpcErrorResponse: Equatable where Value: Equatable {}
extension RpcNotification: Equatable where Value: Equatable {}

extension RpcErrorResponse: CustomStringConvertible {
    public var description: String {
        let idDescription = id.map(String.init(describing:)) ?? "nil"
        let dataDescription = data.map(String.init(describing:)) ?? "nil"
        return "RpcErrorResponse(id: \(idDescription), code: \(code), message: \(message), data: \(dataDescription))"
    }
}
