// ============================================================================
// FuzzCorpus.swift -- evaluating the fuzz-v1 conformance corpus
// (code/specs/fixtures/fuzz-v1/cases.json, VER06 section 8, VER06l section 8)
// ============================================================================
//
// The runner takes the parsed JSON, strictly decodes it, evaluates every case
// with this package's own public engine functions and returns human-readable
// problems. No problems and the right count means the lane conforms. It does no
// file I/O (the package is capability-free); the tests read the committed fixture.
//
// A runner that cannot report a problem is worse than none, so this one is strict:
//   * a case has exactly the fields of its op, in canonical encodings (ints are
//     decimal strings, bytes are even-length lowercase hex);
//   * an unknown op or a malformed value is a PROBLEM, never a silent skip;
//   * every op appears, a failure id is accepted and one rejected, and the case
//     count equals `fuzzv1CaseCount`, so deleting cases is itself a problem;
//   * fixture-controlled sizes are capped (CWE-400): a hostile `steps = 10^9` is
//     a reported problem, not a hang;
//   * every problem about a case starts with that case's ESCAPED id.

public enum FuzzCorpus {
    /// How many cases fuzz-v1 holds; a different count is a problem.
    public static let fuzzv1CaseCount = 25
    private static let maxCases = 10_000
    /// Bounds the aggregate `mutate_sequence` work of one corpus: the bytes
    /// ACTUALLY copied, summed over cases. Each case is already capped (10,000
    /// steps, 10,000 bytes) but 10,000 such cases would still be 10^12 byte-steps.
    static let maxAggregateSequenceWork = 1 << 24

    public struct Report {
        public let evaluated: Int
        public let problems: [String]
    }

    /// Strictly decodes a fuzz-v1 document into its case list. The document must
    /// be exactly `{"schema_version": 1, "cases": [...]}`. Duplicate keys,
    /// trailing data, nesting deeper than 64 and documents over 8 MiB are refused.
    public static func parse(_ data: [UInt8]) throws -> [Json] {
        let root = try StrictJson.parse(data)
        guard case .object(let pairs) = root, pairs.count == 2 else {
            throw JsonError("fuzz corpus: want exactly {\"schema_version\": 1, \"cases\": [...]}")
        }
        guard case .number(let v)? = root["schema_version"], v == "1" else {
            throw JsonError("fuzz corpus: schema_version must be 1")
        }
        guard case .array(let cases)? = root["cases"] else {
            throw JsonError("fuzz corpus: \"cases\" must be an array")
        }
        return cases
    }

    /// The closed field set of each op (besides id and op).
    static let shapes: [String: [String]] = [
        "mutate_bit_flip": ["input", "bit_offset", "count", "output"],
        "mutate_byte_flip": ["input", "byte_offset", "count", "output"],
        "mutate_arithmetic": ["input", "byte_offset", "delta", "width", "endian", "output"],
        "mutate_boundary": ["input", "byte_offset", "width", "val", "endian", "output"],
        "mutate_block_delete": ["input", "offset", "length", "output"],
        "mutate_block_insert": ["input", "offset", "insert_bytes", "output"],
        "mutate_block_duplicate": ["input", "src_offset", "length", "dst_offset", "output"],
        "mutate_splice": ["target", "donor", "target_split", "donor_split", "output"],
        "mutate_sequence": ["input", "seed", "steps", "output"],
        "corpus_admit": ["existing_edges", "trace_edges", "admitted", "new_edges"],
        "crash_dedup": ["crash_a_kind", "crash_a_frames", "crash_b_kind", "crash_b_frames", "is_duplicate"],
        "failure_id_valid": ["input", "valid"],
        "private_reporting": ["target", "failure_id", "raw_payload", "sig_hash", "tap_line", "contains_raw_payload"],
        "corpus_reject": ["text"],
    ]

    /// A case id made safe to print: printable ASCII kept, every other byte shown
    /// as `?`, at most `limit` bytes (`...` marks a cut). A hostile id can then
    /// neither forge a problem line nor smuggle terminal controls.
    static func escapeId(_ s: String, limit: Int = 100) -> String {
        var out = [UInt8]()
        var cut = false
        for (i, b) in s.utf8.enumerated() {
            if i >= limit { cut = true; break }
            out.append(b >= 0x20 && b <= 0x7E ? b : 0x3F)
        }
        return String(decoding: out, as: UTF8.self) + (cut ? "..." : "")
    }

    public static func evaluate(_ cases: [Json]) -> Report { evaluate(cases, expected: fuzzv1CaseCount) }

    /// Evaluates every case and returns the problems (none means the engine
    /// conforms). A negative `expected` skips the case-count check.
    public static func evaluate(_ casesIn: [Json], expected: Int) -> Report {
        let hooks = FuzzHooks.current
        var problems: [String] = []
        var evaluated = 0
        var cases = casesIn
        if expected >= 0 && cases.count != expected {
            problems.append("fuzz-v1: expected \(expected) cases, got \(cases.count)")
        }
        if cases.count > maxCases {
            problems.append("fuzz-v1: \(cases.count) cases exceeds the cap of \(maxCases)")
            cases = Array(cases.prefix(maxCases))
        }

        // Ids are compared byte-exact; a non-string id is a reported problem.
        var seenIds = Set<[UInt8]>()
        for raw in cases {
            guard case .string(let id)? = raw["id"], !id.isEmpty, seenIds.insert(Array(id.utf8)).inserted else {
                problems.append("duplicate or missing case id")
                break
            }
        }

        var workLeft = maxAggregateSequenceWork
        var opsSeen = Set<String>()
        var acceptedIdSeen = false
        var rejectedIdSeen = false

        for raw in cases {
            guard case .object(let pairs) = raw else {
                problems.append("case is not an object: \(Values.show(raw))")
                continue
            }
            guard case .string(let rawId)? = raw["id"], !rawId.isEmpty else {
                problems.append("case missing a string id")
                continue
            }
            let cid = escapeId(rawId)
            let op: String = { if case .string(let s)? = raw["op"] { return s } else { return "" } }()
            guard let fields = shapes[op] else {
                problems.append("\(cid): unknown op \(Values.show(raw["op"].map { j -> Any in if case .string(let s) = j { return s } else { return "?" } }))")
                continue
            }
            let keys = Set(pairs.map { Array($0.key.utf8) })
            let want = Set((fields + ["id", "op"]).map { Array($0.utf8) })
            if keys != want || pairs.count != want.count {
                problems.append("\(cid): fields \(pairs.map { escapeId($0.key, limit: 40) }.sorted()) != \((fields + ["id", "op"]).sorted())")
                continue
            }
            let ev = evaluateCase(cid, op, raw, &workLeft, hooks)
            problems.append(contentsOf: ev.problems)
            if !ev.ranOk { continue }
            if op == "failure_id_valid" {
                acceptedIdSeen = acceptedIdSeen || ev.accepted
                rejectedIdSeen = rejectedIdSeen || !ev.accepted
            }
            opsSeen.insert(op)
            evaluated += 1
        }

        let missing = Set(shapes.keys).subtracting(opsSeen).sorted()
        if !missing.isEmpty { problems.append("fuzz-v1: operations never exercised: \(missing)") }
        if !acceptedIdSeen { problems.append("fuzz-v1: failure_id_valid needs a registered-id acceptance") }
        if !rejectedIdSeen { problems.append("fuzz-v1: failure_id_valid needs an unregistered-id rejection") }
        let unregistered = FuzzFailureIds.all.filter { !hooks.isFailureId($0) }
        if !unregistered.isEmpty { problems.append("fuzz-v1: engine rejects registered ids \(unregistered)") }
        return Report(evaluated: evaluated, problems: problems.map { clip($0, Values.maxProblemLen) })
    }

    private struct Eval {
        var problems: [String]
        var ranOk: Bool
        var accepted: Bool
    }

    /// Decodes and checks one case. Any error (a malformed case, or an engine that
    /// threw) is reported as a problem.
    private static func evaluateCase(_ cid: String, _ op: String, _ c: Json, _ work: inout Int, _ hooks: FuzzHooks.Table) -> Eval {
        do {
            var v: [String: Value] = [:]
            if case .object(let pairs) = c {
                for (k, node) in pairs where k != "id" && k != "op" { v[k] = try Values.decode(node) }
            }
            var found: [String] = []
            var accepted = false
            switch op {
            case "corpus_admit": found = try checkCorpusAdmit(cid, v, hooks)
            case "crash_dedup": found = try checkCrashDedup(cid, v, hooks)
            case "failure_id_valid":
                if case .string(let s)? = v["input"] { accepted = hooks.isFailureId(s) }
                if Value.bool(accepted) != v["valid"] {
                    found.append("\(cid): failure id membership disagrees for \(Values.show(v["input"]))")
                }
            case "private_reporting": found = try checkPrivateReporting(cid, v, hooks)
            case "corpus_reject":
                guard case .string(let text)? = v["text"] else { throw CaseError("text must be a string") }
                if hooks.tfvValid(text) { found.append("\(cid): corpus_reject text was accepted by the strict parser") }
            default: found = try checkMutator(cid, op, v, &work, hooks)
            }
            return Eval(problems: found, ranOk: true, accepted: accepted)
        } catch let e as CaseError {
            return Eval(problems: ["\(cid): \(op) could not be evaluated: \(clip(e.message, Values.maxFragmentLen))"], ranOk: false, accepted: false)
        } catch {
            // Only static engine errors reach here; their payload is fixed text.
            return Eval(problems: ["\(cid): \(op) could not be evaluated: \(clip(String(describing: error), Values.maxFragmentLen))"],
                        ranOk: false, accepted: false)
        }
    }

    // MARK: typed field access ------------------------------------------------------------

    private static func bytesOf(_ v: Value?, _ what: String) throws -> [UInt8] {
        if case .bytes(let b)? = v { return b }
        throw CaseError("\(what) must be bytes")
    }
    private static func textOf(_ v: Value?, _ what: String) throws -> String {
        if case .string(let s)? = v { return s }
        throw CaseError("\(what) must be a string")
    }
    private static func boolOf(_ v: Value?, _ what: String) throws -> Bool {
        if case .bool(let b)? = v { return b }
        throw CaseError("\(what) must be a bool")
    }
    private static func intOf(_ v: Value?, _ what: String) throws -> BigInt {
        if case .int(let b)? = v { return b }
        throw CaseError("\(what) must be an int")
    }
    /// An int that drives a loop or an allocation: 0 <= n <= maxSize.
    private static func sizeOf(_ v: Value?, _ what: String) throws -> Int {
        let b = try intOf(v, what)
        guard let n = b.asInt, n >= 0, n <= Values.maxSize else {
            throw CaseError("\(what) \(clip(b.description, 40)) outside [0, \(Values.maxSize)]")
        }
        return n
    }
    /// Maps a corpus width to an Int; anything outside [0, 64] becomes -1, which the
    /// mutators treat as "not one of 1, 2, 4, 8" (input unchanged).
    private static func widthOf(_ v: Value?) throws -> Int {
        let b = try intOf(v, "width")
        guard let n = b.asInt, n >= 0, n <= 64 else { return -1 }
        return n
    }
    private static func endianOf(_ v: Value?) throws -> Endian {
        switch try textOf(v, "endian") {
        case "little": return .little
        case "big": return .big
        default: throw CaseError("endian must be 'little' or 'big', got \(Values.show(v))")
        }
    }
    private static func textsOf(_ v: Value?, _ what: String) throws -> [String] {
        guard case .list(let items)? = v else { throw CaseError("\(what) must be a list of strings") }
        return try items.map {
            if case .string(let s) = $0 { return s }
            throw CaseError("\(what) must be a list of strings")
        }
    }
    /// A list of edge ids; each must fit in an unsigned 64-bit word.
    private static func edgesOf(_ v: Value?, _ what: String) throws -> [UInt64] {
        guard case .list(let items)? = v else { throw CaseError("\(what) must be a list of ints") }
        return try items.map {
            guard case .int(let b) = $0 else { throw CaseError("\(what) must be a list of ints") }
            guard let u = b.asUInt64 else { throw CaseError("\(what) element \(clip(b.description, 40)) outside [0, 2^64)") }
            return u
        }
    }

    private static func hex(_ b: [UInt8]) -> String {
        let d = Array("0123456789abcdef".utf8)
        var out = [UInt8]()
        out.reserveCapacity(b.count * 2)
        for x in b { out.append(d[Int(x >> 4)]); out.append(d[Int(x & 15)]) }
        return String(decoding: out, as: UTF8.self)
    }

    // MARK: per-op checks ---------------------------------------------------------------------

    private static func checkMutator(_ cid: String, _ op: String, _ v: [String: Value], _ work: inout Int, _ h: FuzzHooks.Table) throws -> [String] {
        let got: [UInt8]
        switch op {
        case "mutate_bit_flip":
            got = h.bitFlip(try bytesOf(v["input"], "input"), try intOf(v["bit_offset"], "bit_offset"), try intOf(v["count"], "count"))
        case "mutate_byte_flip":
            got = h.byteFlip(try bytesOf(v["input"], "input"), try intOf(v["byte_offset"], "byte_offset"), try intOf(v["count"], "count"))
        case "mutate_arithmetic":
            got = h.arithmetic(try bytesOf(v["input"], "input"), try intOf(v["byte_offset"], "byte_offset"),
                               try intOf(v["delta"], "delta"), try widthOf(v["width"]), try endianOf(v["endian"]))
        case "mutate_boundary":
            got = h.boundary(try bytesOf(v["input"], "input"), try intOf(v["byte_offset"], "byte_offset"),
                             try widthOf(v["width"]), try intOf(v["val"], "val"), try endianOf(v["endian"]))
        case "mutate_block_delete":
            got = h.blockDelete(try bytesOf(v["input"], "input"), try intOf(v["offset"], "offset"), try intOf(v["length"], "length"))
        case "mutate_block_insert":
            got = h.blockInsert(try bytesOf(v["input"], "input"), try intOf(v["offset"], "offset"), try bytesOf(v["insert_bytes"], "insert_bytes"))
        case "mutate_block_duplicate":
            got = h.blockDuplicate(try bytesOf(v["input"], "input"), try intOf(v["src_offset"], "src_offset"),
                                   try intOf(v["length"], "length"), try intOf(v["dst_offset"], "dst_offset"))
        case "mutate_splice":
            let target = try bytesOf(v["target"], "target"), donor = try bytesOf(v["donor"], "donor")
            let ts = try intOf(v["target_split"], "target_split"), ds = try intOf(v["donor_split"], "donor_split")
            got = h.splice(target, donor, ts, ds)
            var problems = compare(cid, op, got, v)
            let want = Mutate.spliceLength(targetLen: target.count, donorLen: donor.count,
                                           targetSplit: Mutate.satU64(ts), donorSplit: Mutate.satU64(ds))
            if got.count != want { problems.append("\(cid): splice length is not prefix + suffix") }
            return problems
        default: // mutate_sequence
            let input = try bytesOf(v["input"], "input")
            let seed = try intOf(v["seed"], "seed")
            let steps = try sizeOf(v["steps"], "steps")
            // Charge the REAL work: the budget handed down is what is left of the
            // aggregate cap, and what the call actually copied (block inserts and
            // duplicates grow the buffer, so steps x initial length would
            // undercount) is deducted afterwards. A call that hits its cap is
            // charged too, and an exhausted aggregate fails every later case at
            // once without running it: otherwise each failing case would be
            // handed the full budget again.
            let exhausted = CaseError("mutate_sequence work budget of \(maxAggregateSequenceWork) bytes exhausted")
            if work <= 0 { throw exhausted }
            do {
                let done = try h.sequence(input, Mutate.low64(seed), steps, Mutate.maxInputBytes, work)
                work -= done.work
                got = done.data
            } catch FuzzError.workExhausted(let spent) {
                work -= max(spent, 0)
                if work > 0 { work = 0 } // the aggregate is breached: nothing is left for any later case
                throw exhausted
            }
        }
        var problems = compare(cid, op, got, v)
        // Engine invariant, independent of the corpus's expected bytes.
        if ["mutate_bit_flip", "mutate_byte_flip", "mutate_arithmetic", "mutate_boundary"].contains(op),
           case .bytes(let input)? = v["input"], got.count != input.count {
            problems.append("\(cid): \(op) changed the length of its input")
        }
        return problems
    }

    private static func compare(_ cid: String, _ op: String, _ got: [UInt8], _ v: [String: Value]) -> [String] {
        guard case .bytes(let want)? = v["output"] else { return ["\(cid): \(op) output must be bytes"] }
        if got != want {
            return ["\(cid): \(op) expected \(clip(hex(want), Values.maxFragmentLen)), got \(clip(hex(got), Values.maxFragmentLen))"]
        }
        return []
    }

    private static func checkCorpusAdmit(_ cid: String, _ v: [String: Value], _ h: FuzzHooks.Table) throws -> [String] {
        let existing = try edgesOf(v["existing_edges"], "existing_edges")
        let trace = try edgesOf(v["trace_edges"], "trace_edges")
        let admitted = try boolOf(v["admitted"], "admitted")
        let newEdges = try intOf(v["new_edges"], "new_edges")
        let got = h.corpusAdmit(existing, trace)
        var problems: [String] = []
        if got.admitted != admitted { problems.append("\(cid): corpus_admit admitted \(got.admitted), corpus \(admitted)") }
        if BigInt(got.newEdges) != newEdges {
            problems.append("\(cid): corpus_admit new_edges \(got.newEdges), corpus \(clip(newEdges.description, 40))")
        }
        // Invariant: admission is exactly "some trace edge is unseen".
        let have = Set(existing)
        if got.admitted != trace.contains(where: { !have.contains($0) }) {
            problems.append("\(cid): corpus_admit disagrees with set difference")
        }
        return problems
    }

    private static func checkCrashDedup(_ cid: String, _ v: [String: Value], _ h: FuzzHooks.Table) throws -> [String] {
        let ka = try textOf(v["crash_a_kind"], "crash_a_kind"), fa = try textsOf(v["crash_a_frames"], "crash_a_frames")
        let kb = try textOf(v["crash_b_kind"], "crash_b_kind"), fb = try textsOf(v["crash_b_frames"], "crash_b_frames")
        let want = try boolOf(v["is_duplicate"], "is_duplicate")
        let got = h.isDuplicate(ka, fa, kb, fb)
        return got == want ? [] : ["\(cid): crash_dedup is_duplicate \(got), corpus \(want)"]
    }

    private static func checkPrivateReporting(_ cid: String, _ v: [String: Value], _ h: FuzzHooks.Table) throws -> [String] {
        let raw = try bytesOf(v["raw_payload"], "raw_payload")
        let target = try textOf(v["target"], "target"), fid = try textOf(v["failure_id"], "failure_id")
        let sig = try textOf(v["sig_hash"], "sig_hash"), tap = try textOf(v["tap_line"], "tap_line")
        let wantLeak = try boolOf(v["contains_raw_payload"], "contains_raw_payload")
        let line: String
        do { line = try h.formatTap(1, target, fid, sig) } catch { throw CaseError("format_tap_failure refused its arguments") }
        var problems: [String] = []
        if line != tap {
            problems.append("\(cid): tap_line expected \(clip(Values.show(tap), Values.maxFragmentLen)), got \(clip(Values.show(line), Values.maxFragmentLen))")
        }
        let leaked = !raw.isEmpty && line.contains(hex(raw))
        if leaked != wantLeak { problems.append("\(cid): privacy verdict wrong, payload leaked: \(leaked)") }
        if line.contains("\r") || line.contains("\n") || line.unicodeScalars.contains(where: { $0.value == 0x0A || $0.value == 0x0D }) {
            problems.append("\(cid): failure line spans more than one line")
        }
        return problems
    }
}
