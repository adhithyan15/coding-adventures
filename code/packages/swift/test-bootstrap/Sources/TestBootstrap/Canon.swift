// ============================================================================
// Canon.swift -- total, bounded rendering of witnesses
// ============================================================================
//
// A verdict message names the value that broke the property. That value is
// arbitrary user data, so rendering it must be TOTAL and BOUNDED:
//
//   * at most `maxOutput` (1 MiB) of text;
//   * at most `maxNodes` (10,000) values visited;
//   * nesting deeper than `maxDepth` (16) renders as `...`;
//   * control characters (below 0x20, and 0x7f) are escaped as `\u00XX`, so a
//     hostile string cannot forge log lines or terminal escapes.
//
// THE `description` HAZARD
// ------------------------
// Swift's `String(describing:)` calls the value's own `description`, which is
// user code: it can recurse forever, and a Swift stack overflow is a TRAP that
// no `catch` can stop. So this renderer never calls `description` on a value
// that has structure. It walks the value with `Mirror` (reflection) up to the
// caps above. It uses `description` only for the built-in scalars, `BigInt`,
// and payload-free enums whose type does not customise it. Everything else
// renders by structure, or, as a last resort, as its type name.

public enum Canon {
    public static let maxDepth = 16
    public static let maxNodes = 10_000
    public static let maxOutput = 1 << 20
    /// Returned instead of a rendering when none can be produced.
    public static let unrenderable = "<unrenderable>"

    /// Renders any value. Never throws, never recurses past `maxDepth`.
    public static func render(_ value: Any) -> String {
        var r = Renderer()
        r.render(value, depth: 0)
        return r.finish()
    }

    private struct Renderer {
        var out = ""
        var nodes = 0
        var truncated = false
        /// This renderer's own output cap. The top level gets `maxOutput`; a
        /// sub-renderer (a dictionary key or value, a set element) gets at most
        /// `subLimit` and never more than the budget remaining, so the TOTAL can
        /// never exceed `maxOutput` (F2).
        var limit = Canon.maxOutput

        /// The longest rendering of one dictionary key / value / set element.
        static let subLimit = 4096

        mutating func emit(_ s: String) {
            if truncated { return }
            if out.utf8.count >= limit { truncated = true; return }
            out += s
        }

        /// Renders `child` on its own, sharing the node counter and capped by
        /// both `subLimit` and `room`. Returns the text.
        mutating func renderSub(_ child: Any, depth: Int, room: Int) -> String {
            var sub = Renderer()
            sub.nodes = nodes
            sub.limit = Swift.max(0, Swift.min(Renderer.subLimit, room))
            sub.render(child, depth: depth)
            nodes = sub.nodes
            return sub.finish()
        }

        /// Renders each child to its own text (bounded), stopping at the node or
        /// output cap; the caller sorts, so the result is independent of the
        /// container's iteration order.
        mutating func renderEntries(_ children: Mirror.Children, depth: Int, pairs: Bool) -> (entries: [(String, String)], cut: Bool) {
            var entries: [(String, String)] = []
            var cut = false
            var used = 0
            let room = Swift.max(0, limit - out.utf8.count)
            for child in children {
                // One node per child BEFORE any shape test, so a mirror whose
                // children are not pairs (or never end) still terminates.
                nodes += 1
                if nodes > Canon.maxNodes || used >= room { cut = true; break }
                if pairs {
                    let pair = Mirror(reflecting: child.value).children.map { $0.value }
                    if pair.count != 2 { continue }
                    let k = renderSub(pair[0], depth: depth, room: room - used)
                    let v = renderSub(pair[1], depth: depth, room: room - used - k.utf8.count)
                    used += k.utf8.count + v.utf8.count + 2
                    entries.append((k, v))
                } else {
                    let k = renderSub(child.value, depth: depth, room: room - used)
                    used += k.utf8.count + 1
                    entries.append((k, ""))
                }
            }
            // Ordered by UTF-8 bytes, not by Swift's canonical-equivalence `<`/`==`, so
            // NFC and NFD spellings of the same text keep a stable, distinct order.
            entries.sort { a, b in
                let ka = Array(a.0.utf8), kb = Array(b.0.utf8)
                if ka != kb { return ka.lexicographicallyPrecedes(kb) }
                return Array(a.1.utf8).lexicographicallyPrecedes(Array(b.1.utf8))
            }
            return (entries, cut)
        }

        mutating func finish() -> String {
            if truncated { out += "..." }
            return out
        }

        mutating func render(_ v: Any, depth: Int) {
            nodes += 1
            if nodes > Canon.maxNodes { truncated = true; return }
            if truncated { return }

            // Scalars first: cheap, and never structural.
            switch v {
            case let x as Bool: emit(x ? "true" : "false"); return
            case let x as Int: emit(String(x)); return
            case let x as Int8: emit(String(x)); return
            case let x as Int16: emit(String(x)); return
            case let x as Int32: emit(String(x)); return
            case let x as Int64: emit(String(x)); return
            case let x as UInt: emit(String(x)); return
            case let x as UInt8: emit(String(x)); return
            case let x as UInt16: emit(String(x)); return
            case let x as UInt32: emit(String(x)); return
            case let x as UInt64: emit(String(x)); return
            case let x as Double: emit(String(x)); return
            case let x as Float: emit(String(x)); return
            case let x as BigInt: emit(x.description); return
            case let x as String: quote(x); return
            case let x as Substring: quote(String(x)); return
            case let x as Character: quote(String(x)); return
            case let x as [UInt8]: hex(x); return
            default: break
            }

            let m = Mirror(reflecting: v)
            switch m.displayStyle {
            case .optional:
                if let child = m.children.first { render(child.value, depth: depth) } else { emit("nil") }
            case .collection:
                if depth >= Canon.maxDepth { emit("..."); return }
                emit("[")
                var first = true
                for child in m.children {
                    if !first { emit(",") }
                    first = false
                    if nodes >= Canon.maxNodes { emit("..."); truncated = true; break }
                    render(child.value, depth: depth + 1)
                }
                emit("]")
            case .set:
                // Hash order is unspecified (and per-process random), so sort the
                // rendered elements: the same set always renders the same text
                // (F5), which keeps the CI `repro-fnv` digest deterministic.
                if depth >= Canon.maxDepth { emit("..."); return }
                let (items, cut) = renderEntries(m.children, depth: depth + 1, pairs: false)
                emit("[")
                for (i, e) in items.enumerated() {
                    if i > 0 { emit(",") }
                    emit(e.0)
                }
                if cut { emit(",...") }
                emit("]")
            case .dictionary:
                if depth >= Canon.maxDepth { emit("..."); return }
                let (entries, cut) = renderEntries(m.children, depth: depth + 1, pairs: true)
                emit("{")
                for (i, e) in entries.enumerated() {
                    if i > 0 { emit(",") }
                    emit(e.0); emit(":"); emit(e.1)
                }
                if cut { emit(",...") }
                emit("}")
            case .struct, .class, .tuple:
                if depth >= Canon.maxDepth { emit("..."); return }
                emit("{")
                var first = true
                for child in m.children {
                    if !first { emit(",") }
                    first = false
                    if nodes >= Canon.maxNodes { emit("..."); truncated = true; break }
                    render(child.value, depth: depth + 1)
                }
                emit("}")
            case .enum:
                if m.children.isEmpty {
                    // A payload-free enum case. Its name comes from
                    // `String(describing:)`, which is safe ONLY when the type
                    // has not customised `description`.
                    if v is CustomStringConvertible || v is CustomDebugStringConvertible {
                        quote(String(reflecting: type(of: v)))
                    } else {
                        emit(String(describing: v))
                    }
                } else {
                    if depth >= Canon.maxDepth { emit("..."); return }
                    for child in m.children {
                        emit(child.label ?? "case")
                        emit("(")
                        render(child.value, depth: depth + 1)
                        emit(")")
                    }
                }
            default:
                quote(String(reflecting: type(of: v)))
            }
        }

        mutating func hex(_ bytes: [UInt8]) {
            emit("0x")
            let digits = Array("0123456789abcdef")
            for b in bytes {
                if truncated { return }
                emit(String([digits[Int(b >> 4)], digits[Int(b & 15)]]))
            }
        }

        mutating func quote(_ s: String) {
            emit("\"")
            for u in s.unicodeScalars {
                if truncated { return }
                switch u {
                case "\"": emit("\\\"")
                case "\\": emit("\\\\")
                case "\n": emit("\\n")
                case "\r": emit("\\r")
                case "\t": emit("\\t")
                default:
                    // Control characters, C1 controls, line/paragraph separators and
                    // bidirectional overrides are escaped so a witness cannot forge
                    // log lines or reorder displayed text (F5).
                    if u.value < 0x20 || (u.value >= 0x7f && u.value <= 0x9f)
                        || u.value == 0x2028 || u.value == 0x2029 || (u.value >= 0x202A && u.value <= 0x202E) {
                        let h = String(u.value, radix: 16)
                        emit("\\u" + String(repeating: "0", count: 4 - h.count) + h)
                    } else {
                        emit(String(u))
                    }
                }
            }
            emit("\"")
        }
    }
}

/// Clips text to at most `limit` characters, marking the cut.
func clip(_ s: String, _ limit: Int) -> String {
    s.count <= limit ? s : String(s.prefix(limit)) + "..."
}
