import CEngram
import Foundation
#if os(macOS)
import AppKit
#endif

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
        return handleHostIntent(response)
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

    private func handleHostIntent(_ response: NSDictionary?) -> NSDictionary? {
        guard let responseMap = response as? [String: Any],
              let hostIntent = responseMap["hostIntent"] as? [String: Any],
              let type = hostIntent["type"] as? String else {
            return response
        }
        switch type {
        case "importAnki":
            return importAnkiPackage(response: responseMap, hostIntent: hostIntent)
        case "exportAnki":
            return exportAnkiPackage(response: responseMap, hostIntent: hostIntent)
        default:
            return response
        }
    }

    private func importAnkiPackage(
        response: [String: Any],
        hostIntent: [String: Any]
    ) -> NSDictionary {
        guard let session else {
            return hostResultResponse(response, hostIntent: hostIntent, status: "unavailable")
        }

        #if os(macOS)
        guard let url = pickAnkiImportURL(hostIntent: hostIntent) else {
            return hostResultResponse(response, hostIntent: hostIntent, status: "cancelled")
        }

        do {
            let data = try Data(contentsOf: url)
            let json = data.withUnsafeBytes { rawBuffer -> String in
                guard let bytes = rawBuffer.bindMemory(to: UInt8.self).baseAddress else {
                    return "{\"ok\":false,\"error\":\"Anki package was empty\"}"
                }
                return takeCString(eg_merge_anki_apkg(session, bytes, data.count))
            }
            guard let imported = decodeJsonObject(json),
                  (imported["ok"] as? Bool) != false else {
                print("Engram Anki import failed: \(decodeJsonObject(json)?["error"] ?? "unknown error")")
                return hostResultResponse(
                    response,
                    hostIntent: hostIntent,
                    status: "import-error",
                    path: url.path)
            }

            persistSnapshot()
            var refreshed = applyProps() as? [String: Any] ?? [:]
            let hostResult: [String: Any] = [
                "status": "imported",
                "path": url.path,
            ]
            refreshed["hostIntent"] = hostIntent
            refreshed["hostResult"] = hostResult
            return withHostStatusProps(refreshed, hostResult: hostResult)
        } catch {
            print("Engram could not import Anki package: \(error)")
            return hostResultResponse(
                response,
                hostIntent: hostIntent,
                status: "read-error",
                path: url.path)
        }
        #else
        return hostResultResponse(response, hostIntent: hostIntent, status: "unsupported")
        #endif
    }

    private func exportAnkiPackage(
        response: [String: Any],
        hostIntent: [String: Any]
    ) -> NSDictionary {
        guard let session else {
            return hostResultResponse(response, hostIntent: hostIntent, status: "unavailable")
        }

        #if os(macOS)
        guard let url = pickAnkiExportURL(hostIntent: hostIntent) else {
            return hostResultResponse(response, hostIntent: hostIntent, status: "cancelled")
        }

        let json = takeCString(eg_export_anki_apkg(session))
        guard let root = decodeJsonObject(json),
              (root["ok"] as? Bool) != false else {
            print("Engram Anki export failed: \(decodeJsonObject(json)?["error"] ?? "unknown error")")
            return hostResultResponse(
                response,
                hostIntent: hostIntent,
                status: "export-error",
                path: url.path)
        }

        let data = jsonByteArray(root, property: "apkg")
        guard !data.isEmpty else {
            return hostResultResponse(
                response,
                hostIntent: hostIntent,
                status: "export-error",
                path: url.path)
        }

        do {
            try data.write(to: url, options: [.atomic])
            return hostResultResponse(
                response,
                hostIntent: hostIntent,
                status: "exported",
                path: url.path)
        } catch {
            print("Engram could not export Anki package: \(error)")
            return hostResultResponse(
                response,
                hostIntent: hostIntent,
                status: "write-error",
                path: url.path)
        }
        #else
        return hostResultResponse(response, hostIntent: hostIntent, status: "unsupported")
        #endif
    }

    private func hostResultResponse(
        _ response: [String: Any],
        hostIntent: [String: Any],
        status: String,
        path: String? = nil
    ) -> NSDictionary {
        var out = response
        out["hostIntent"] = hostIntent
        var hostResult: [String: Any] = ["status": status]
        if let path, !path.isEmpty {
            hostResult["path"] = path
        }
        out["hostResult"] = hostResult
        return withHostStatusProps(out, hostResult: hostResult)
    }

    private func withHostStatusProps(
        _ response: [String: Any],
        hostResult: [String: Any]
    ) -> NSDictionary {
        let statusProps = hostStatusProps(hostResult)
        if statusProps.isEmpty {
            return response as NSDictionary
        }
        var out = response
        var props = out["props"] as? [String: Any] ?? [:]
        for (key, value) in statusProps {
            props[key] = value
        }
        out["props"] = props
        return out as NSDictionary
    }

    private func hostStatusProps(_ hostResult: [String: Any]) -> [String: Any] {
        guard let status = hostResult["status"] as? String, !status.isEmpty else {
            return [:]
        }
        return [
            "host-status-visible": true,
            "host-status-kind": status,
            "host-status-label": hostStatusLabel(status),
            "host-status-message": hostStatusMessage(hostResult, status: status),
        ]
    }

    private func hostStatusLabel(_ status: String) -> String {
        switch status {
        case "imported":
            return "Import complete"
        case "exported":
            return "Export complete"
        case "cancelled":
            return "Import cancelled"
        case "read-error", "import-error":
            return "Import failed"
        case "export-error", "write-error":
            return "Export failed"
        case "unavailable", "unsupported":
            return "Host unavailable"
        default:
            return "Host status"
        }
    }

    private func hostStatusMessage(_ hostResult: [String: Any], status: String) -> String {
        let file = hostResultFile(hostResult)
        switch status {
        case "imported":
            return file.isEmpty ? "Anki package imported." : "Imported \(file)."
        case "exported":
            return file.isEmpty ? "Anki package exported." : "Saved \(file)."
        case "cancelled":
            return "No Anki package was selected."
        case "read-error":
            return file.isEmpty ? "Could not read the selected file." : "Could not read \(file)."
        case "import-error":
            return file.isEmpty ? "Could not import the selected package." : "Could not import \(file)."
        case "export-error":
            return "Could not export Anki package."
        case "write-error":
            return file.isEmpty ? "Could not save the Anki package." : "Could not save \(file)."
        case "unavailable":
            return "Engram native host is unavailable."
        case "unsupported":
            return "This host does not support native Anki file dialogs yet."
        default:
            return file.isEmpty ? status : file
        }
    }

    private func hostResultFile(_ hostResult: [String: Any]) -> String {
        guard let path = hostResult["path"] as? String, !path.isEmpty else {
            return ""
        }
        return URL(fileURLWithPath: path).lastPathComponent
    }

    private func hostIntentExtensions(
        _ hostIntent: [String: Any],
        property: String,
        fallback: [String]
    ) -> [String] {
        guard let raw = hostIntent[property] as? [Any] else {
            return fallback
        }
        let extensions = raw.compactMap { value -> String? in
            var extensionValue = String(describing: value)
                .trimmingCharacters(in: .whitespacesAndNewlines)
            if extensionValue.isEmpty {
                return nil
            }
            if !extensionValue.hasPrefix(".") {
                extensionValue = ".\(extensionValue)"
            }
            return extensionValue
        }
        return extensions.isEmpty ? fallback : extensions
    }

    private func suggestedAnkiFileName(_ hostIntent: [String: Any]) -> String {
        var name = (hostIntent["deckId"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if name.isEmpty {
            name = "engram-collection"
        }
        let invalidCharacters = CharacterSet(charactersIn: "/\\:*?\"<>|")
        name = name.components(separatedBy: invalidCharacters).joined(separator: "-")
        if !name.lowercased().hasSuffix(".apkg") {
            name += ".apkg"
        }
        return name
    }

    private func fileTypes(from extensions: [String]) -> [String] {
        extensions.map { extensionValue in
            if extensionValue.hasPrefix(".") {
                return String(extensionValue.dropFirst())
            }
            return extensionValue
        }
    }

    private func jsonByteArray(_ root: [String: Any], property: String) -> Data {
        guard let values = root[property] as? [Any] else {
            return Data()
        }
        let bytes = values.compactMap { value -> UInt8? in
            if let number = value as? NSNumber {
                return number.uint8Value
            }
            if let int = value as? Int {
                return UInt8(truncatingIfNeeded: int)
            }
            return nil
        }
        return Data(bytes)
    }

    #if os(macOS)
    private func pickAnkiImportURL(hostIntent: [String: Any]) -> URL? {
        let extensions = hostIntentExtensions(
            hostIntent,
            property: "accept",
            fallback: [".apkg", ".colpkg"])
        let panel = NSOpenPanel()
        panel.title = "Import Anki package"
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        panel.allowedFileTypes = fileTypes(from: extensions)
        return panel.runModal() == .OK ? panel.url : nil
    }

    private func pickAnkiExportURL(hostIntent: [String: Any]) -> URL? {
        let extensions = hostIntentExtensions(
            hostIntent,
            property: "extensions",
            fallback: [".apkg"])
        let panel = NSSavePanel()
        panel.title = "Export Anki package"
        panel.nameFieldStringValue = suggestedAnkiFileName(hostIntent)
        panel.canCreateDirectories = true
        panel.allowedFileTypes = fileTypes(from: extensions)
        guard let url = panel.runModal() == .OK ? panel.url : nil else {
            return nil
        }
        if url.pathExtension.isEmpty {
            return url.appendingPathExtension("apkg")
        }
        return url
    }
    #endif

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
