import CEngram
import Foundation

@objc final class MosaicHost: NSObject, MosaicHostBridgeObject {
    private var session: OpaquePointer?

    override init() {
        self.session = eg_session_new_demo()
        super.init()
    }

    deinit {
        if let session {
            eg_session_free(session)
        }
    }

    func applyProps() -> NSDictionary? {
        guard let session else {
            return [:]
        }
        let deckId = currentDeckId()
        let json = deckId.withCString { deckPointer in
            takeCString(eg_engram_app_props(session, deckPointer, currentTimeMillis()))
        }
        return hostResponseDictionary(from: json)
    }

    func handleEvent(_ envelope: NSDictionary, name: NSString) -> NSDictionary? {
        guard let session else {
            return [:]
        }
        let eventJson = encodeJson(envelope)
        let deckId = currentDeckId()
        let json = eventJson.withCString { eventPointer in
            deckId.withCString { deckPointer in
                takeCString(eg_handle_engram_app_event(
                    session,
                    eventPointer,
                    deckPointer,
                    currentTimeMillis()))
            }
        }
        return hostResponseDictionary(from: json)
    }

    private func hostResponseDictionary(from json: String) -> NSDictionary? {
        guard let root = decodeJsonObject(json) else {
            return [:]
        }
        if let ok = root["ok"] as? Bool, !ok {
            if let error = root["error"] {
                print("Engram host error: \(error)")
            }
            return ["error": root["error"] ?? "unknown error"] as NSDictionary
        }
        var response: [String: Any] = [
            "props": root["props"] as? [String: Any] ?? [:],
        ]
        if let intent = root["hostIntent"] as? [String: Any],
           let type = intent["type"] {
            print("Engram host intent: \(type)")
            response["hostIntent"] = intent
        }
        return response as NSDictionary
    }

    private func encodeJson(_ object: NSDictionary) -> String {
        guard JSONSerialization.isValidJSONObject(object),
              let data = try? JSONSerialization.data(withJSONObject: object, options: []),
              let json = String(data: data, encoding: .utf8) else {
            return "{}"
        }
        return json
    }

    private func decodeJsonObject(_ json: String) -> [String: Any]? {
        guard let data = json.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data, options: []),
              let root = object as? [String: Any] else {
            return nil
        }
        return root
    }

    private func takeCString(_ pointer: UnsafeMutablePointer<CChar>?) -> String {
        guard let pointer else {
            return "{\"ok\":false,\"error\":\"Engram native host returned null\"}"
        }
        defer {
            eg_string_free(pointer)
        }
        return String(cString: pointer)
    }

    private func currentDeckId() -> String {
        ProcessInfo.processInfo.environment["ENGRAM_DECK_ID"] ?? ""
    }

    private func currentTimeMillis() -> UInt64 {
        UInt64(Date().timeIntervalSince1970 * 1000)
    }
}
