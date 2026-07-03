import CEngram
import Foundation

@objc final class MosaicHost: NSObject, MosaicHostBridgeObject {
    private var session: OpaquePointer?
    private let snapshotFileName = "mosaic-snapshot.v1.json"

    override init() {
        self.session = eg_session_new_demo()
        super.init()
        hydrateSession()
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
        let response = hostResponseDictionary(from: json)
        if response?["error"] == nil {
            persistSnapshot()
        }
        return response
    }

    private func hydrateSession() {
        guard let session else {
            return
        }
        let url = snapshotURL()
        if let data = try? Data(contentsOf: url),
           let snapshot = String(data: data, encoding: .utf8) {
            let json = snapshot.withCString { snapshotPointer in
                takeCString(eg_load_snapshot(session, snapshotPointer))
            }
            if let root = decodeJsonObject(json),
               (root["ok"] as? Bool) != false {
                return
            }
            print("Engram persisted snapshot was invalid; using demo state")
        }
        persistSnapshot()
    }

    private func persistSnapshot() {
        guard let session else {
            return
        }
        let json = takeCString(eg_snapshot(session))
        guard let root = decodeJsonObject(json),
              (root["ok"] as? Bool) != false,
              let state = root["state"],
              JSONSerialization.isValidJSONObject(state),
              let data = try? JSONSerialization.data(withJSONObject: state, options: []) else {
            return
        }

        let url = snapshotURL()
        do {
            try FileManager.default.createDirectory(
                at: url.deletingLastPathComponent(),
                withIntermediateDirectories: true)
            try data.write(to: url, options: [.atomic])
        } catch {
            print("Engram could not persist snapshot: \(error)")
        }
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

    private func snapshotURL() -> URL {
        if let configured = ProcessInfo.processInfo.environment["ENGRAM_SNAPSHOT_PATH"],
           !configured.isEmpty {
            return URL(fileURLWithPath: configured)
        }
        return FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".engram", isDirectory: true)
            .appendingPathComponent(snapshotFileName)
    }

    private func currentTimeMillis() -> UInt64 {
        UInt64(Date().timeIntervalSince1970 * 1000)
    }
}
