// ============================================================================
// Corpus.swift -- evaluating the pbt-v1 conformance corpus
// ============================================================================
//
// The corpus (code/specs/fixtures/pbt-v1/cases.json) pins what every lane's
// property engine must reproduce: PRNG outputs, generator draws, shrink minima
// AND step counts, replay, and failure-id membership. This evaluator runs every
// case through the engine's PUBLIC functions (the same ones a user calls) and
// returns a `Report`: how many cases ran and a list of problems. No problems and
// the right count means the lane conforms.
//
// STRICTNESS (a corpus check that cannot fail proves nothing):
//   * each case must have exactly the fields of its op, no more, no fewer;
//   * an unknown op, unknown property name or malformed value is a PROBLEM,
//     never a skip;
//   * the case count must equal `pbtv1CaseCount` (25): a truncated or padded
//     corpus fails;
//   * every op and all four failure ids must be exercised, and at least one
//     unregistered id must be rejected;
//   * `corpus_reject` texts are run through the strict TFV validator and must be
//     REFUSED;
//   * `filter_exhausted` runs end to end through `Runner.check` and counts the
//     predicate's calls exactly.
// Every problem line starts with the case id, so a test can assert that a
// corruption is attributed to exactly the corrupted case.

public enum Corpus {
    public static let pbtv1CaseCount = 25
    private static let maxCases = 10_000
    private static let maxGenBytes = 1 << 20

    public struct Report {
        public let evaluated: Int
        public let problems: [String]
    }

    /// Parses the corpus file: exactly `{"schema_version": 1, "cases": [...]}`.
    public static func parse(_ data: [UInt8]) throws -> [Json] {
        let root = try StrictJson.parse(data)
        guard case .object(let pairs) = root, pairs.count == 2 else {
            throw JsonError("pbt corpus: want exactly {\"schema_version\": 1, \"cases\": [...]}")
        }
        guard case .number(let v)? = root["schema_version"], v == "1" else {
            throw JsonError("pbt corpus: schema_version must be 1")
        }
        guard case .array(let cases)? = root["cases"] else {
            throw JsonError("pbt corpus: \"cases\" must be an array")
        }
        return cases
    }

    // MARK: properties named by the corpus (true means "holds") ---------------

    private static let b100 = BigInt(100), b50 = BigInt(50), bM10 = BigInt(-10), b10 = BigInt(10), b20 = BigInt(20)

    private static let intProps: [String: (BigInt) -> Bool] = [
        "int_le_100": { $0 <= b100 },
        "int_in_range_neg10_50": { $0 >= bM10 && $0 <= b50 },
        "int_is_even": { $0.isEven },                 // parity of the magnitude: correct for negatives
        "int_abs_le_10": { $0.magnitude <= b10 },
        "refinement_x_le_50": { $0 <= b50 },
    ]

    private static let listProps: [String: ([BigInt]) -> Bool] = [
        "list_no_consecutive_duplicates": { l in
            var i = 0
            while i + 1 < l.count {
                if l[i] == l[i + 1] { return false }
                i += 1
            }
            return true
        },
        "list_sum_le_20": { l in l.reduce(BigInt.zero, +) <= b20 },
    ]

    private static let bytesProps: [String: ([UInt8]) -> Bool] = [
        "bytes_len_lt_5": { $0.count < 5 },
        "bytes_no_zero_byte": { !$0.contains(0) },
    ]

    private static let shapes: [String: [String]] = [
        "gen_primitive": ["type", "seed", "count", "outputs"],
        "gen_bounded": ["min", "max", "seed", "count", "outputs"],
        "gen_bytes": ["min_len", "max_len", "seed", "count", "outputs"],
        "shrink_int": ["initial", "target", "min", "max", "property", "expected_minimal", "expected_steps"],
        "shrink_refinement": ["initial", "target", "min", "max", "property", "expected_minimal", "expected_steps"],
        "shrink_list": ["initial", "elem_target", "property", "expected_minimal", "expected_steps"],
        "shrink_bytes": ["initial", "property", "expected_minimal", "expected_steps"],
        "replay_consistency": ["seed", "property", "initial_fail", "expected_minimal"],
        "filter_exhausted": ["max_retries", "verdict", "failure_id"],
        "failure_id_valid": ["input", "valid"],
        "corpus_reject": ["text"],
    ]

    // MARK: the evaluator -----------------------------------------------------

    public static func evaluate(_ cases: [Json]) -> Report { evaluate(cases, expected: pbtv1CaseCount) }

    public static func evaluate(_ casesIn: [Json], expected: Int) -> Report {
        var problems: [String] = []
        var evaluated = 0
        var cases = casesIn
        if expected >= 0 && cases.count != expected {
            problems.append("pbt-v1: expected \(expected) cases, got \(cases.count)")
        }
        if cases.count > maxCases {
            problems.append("pbt-v1: \(cases.count) cases exceeds the cap of \(maxCases)")
            cases = Array(cases.prefix(maxCases))
        }

        // Ids are compared as strings only: a non-string id is a reported problem.
        var seenIds = Set<[UInt8]>()   // byte-exact ids (F6)
        for raw in cases {
            guard case .string(let id)? = raw["id"], !id.isEmpty, seenIds.insert(Array(id.utf8)).inserted else {
                problems.append("duplicate or missing case id")
                break
            }
        }

        var opsSeen = Set<String>()
        var registeredSeen = Set<String>()
        var rejectedIdSeen = false

        for raw in cases {
            guard case .object(let pairs) = raw else {
                problems.append("case is not an object")
                continue
            }
            guard case .string(let rawId)? = raw["id"], !rawId.isEmpty else {
                problems.append("case missing a string id")
                continue
            }
            let cid = clip(rawId, 100)
            let op: String = { if case .string(let s)? = raw["op"] { return s } else { return "" } }()
            guard let fields = shapes[op] else {
                problems.append("\(cid): unknown op \(Values.show(op))")
                continue
            }
            let keys = Set(pairs.map { Array($0.key.utf8) })
            let want = Set((fields + ["id", "op"]).map { Array($0.utf8) })
            if keys != want || pairs.count != want.count {
                problems.append("\(cid): fields \(pairs.map { $0.key }.sorted()) != \((fields + ["id", "op"]).sorted())")
                continue
            }
            let ev = evaluateCase(cid, op, raw)
            problems.append(contentsOf: ev.problems)
            if !ev.ranOk { continue }
            if op == "failure_id_valid" {
                if ev.registered { registeredSeen.insert(ev.candidate) } else { rejectedIdSeen = true }
            }
            opsSeen.insert(op)
            evaluated += 1
        }

        let missingOps = Set(shapes.keys).subtracting(opsSeen).sorted()
        if !missingOps.isEmpty { problems.append("pbt-v1: operations never exercised: \(missingOps)") }
        let missingIds = FailureIds.all.filter { !registeredSeen.contains($0) }
        if !missingIds.isEmpty { problems.append("pbt-v1: registered failure ids missing: \(missingIds)") }
        if !rejectedIdSeen { problems.append("pbt-v1: failure_id_valid needs an unregistered-id rejection") }
        return Report(evaluated: evaluated, problems: problems.map { clip($0, Values.maxProblemLen) })
    }

    private struct Eval {
        var problems: [String]
        var ranOk: Bool
        var candidate: String
        var registered: Bool
    }

    /// Decodes and checks one case. Any error (a malformed case, or an engine
    /// that threw) is reported as a problem.
    private static func evaluateCase(_ cid: String, _ op: String, _ c: Json) -> Eval {
        do {
            var v: [String: Value] = [:]
            if case .object(let pairs) = c {
                for (k, node) in pairs where k != "id" && k != "op" { v[k] = try Values.decode(node) }
            }
            var found: [String] = []
            var candidate = ""
            var registered = false
            switch op {
            case "gen_primitive", "gen_bounded", "gen_bytes": found = try checkGen(cid, op, v)
            case "shrink_int", "shrink_refinement": found = try checkShrinkInt(cid, op, v)
            case "shrink_list": found = try checkShrinkList(cid, v)
            case "shrink_bytes": found = try checkShrinkBytes(cid, v)
            case "replay_consistency": found = try checkReplay(cid, v)
            case "filter_exhausted": found = try checkFilterExhausted(cid, v)
            case "failure_id_valid":
                var got = false
                if case .string(let s)? = v["input"] { got = FailureIds.isPbtFailureId(s); candidate = s }
                if Value.bool(got) != v["valid"] {
                    found.append("\(cid): failure id membership disagrees for \(Values.show(v["input"]))")
                }
                registered = got
            default: // corpus_reject
                guard case .string(let text)? = v["text"] else { throw CaseError("text must be a string") }
                if Tfv.isValid(text) { found.append("\(cid): corpus_reject text was accepted by the strict parser") }
            }
            return Eval(problems: found, ranOk: true, candidate: candidate, registered: registered)
        } catch {
            let why: String
            if let e = error as? CaseError { why = e.message }
            else if let e = error as? PbtError { why = "\(e)" }
            else { why = Canon.render(error) }
            return Eval(problems: ["\(cid): \(op) could not be evaluated: \(clip(why, Values.maxFragmentLen))"],
                        ranOk: false, candidate: "", registered: false)
        }
    }

    // MARK: field accessors ---------------------------------------------------

    private static func seedOf(_ v: Value?) throws -> UInt64 {
        guard case .int(let b)? = v else { throw CaseError("seed must be an int") }
        return b.lowU64   // the low 64 bits of the two's-complement form
    }

    private static func intOf(_ v: Value?, _ what: String) throws -> BigInt {
        guard case .int(let b)? = v else { throw CaseError("\(what) must be an int") }
        return b
    }

    private static func sizeOf(_ v: Value?, _ what: String) throws -> Int {
        let b = try intOf(v, what)
        guard let n = b.asInt, n >= 0, n <= Values.maxSize else {
            throw CaseError("\(what) \(Values.show(b)) outside [0, \(Values.maxSize)]")
        }
        return n
    }

    private static func propName(_ v: Value?) throws -> String {
        guard case .string(let s)? = v else { throw CaseError("unknown property \(Values.show(v))") }
        return s
    }

    private static func steps(_ n: Int) -> Value { .int(BigInt(n)) }

    // MARK: per-op checks -----------------------------------------------------

    private static func checkGen(_ cid: String, _ op: String, _ v: [String: Value]) throws -> [String] {
        let seed = try seedOf(v["seed"])
        let count = try sizeOf(v["count"], "count")
        // The expected outputs are in hand already: a case whose list length is
        // not `count` cannot conform, so refuse it before generating anything.
        guard case .list(let expected)? = v["outputs"], expected.count == count else {
            throw CaseError("outputs must be a list of exactly count (\(count)) values")
        }
        var got: [Value] = []
        switch op {
        case "gen_primitive":
            guard case .string(let name)? = v["type"] else { throw CaseError("type must be a string") }
            got = try Gens.genPrimitive(seed: seed, typeName: name, count: count).map {
                switch $0 {
                case .bool(let b): return Value.bool(b)
                case .int(let i): return Value.int(i)
                }
            }
        case "gen_bounded":
            let lo = try intOf(v["min"], "min"), hi = try intOf(v["max"], "max")
            // Gens.genBounded enforces the count x width generation budget itself.
            got = try Gens.genBounded(seed: seed, lo: lo, hi: hi, count: count).map { Value.int($0) }
        default:
            let minLen = try sizeOf(v["min_len"], "min_len"), maxLen = try sizeOf(v["max_len"], "max_len")
            if count * maxLen > maxGenBytes {
                throw CaseError("count \(count) x max_len \(maxLen) exceeds the generation budget of \(maxGenBytes) bytes")
            }
            got = try Gens.genBytes(seed: seed, minLen: minLen, maxLen: maxLen, count: count).map { Value.bytes($0) }
        }
        return got == expected ? [] : ["\(cid): \(op) outputs mismatch"]
    }

    private static func checkShrinkInt(_ cid: String, _ op: String, _ v: [String: Value]) throws -> [String] {
        let name = try propName(v["property"])
        guard let prop = intProps[name] else { throw CaseError("unknown property \(Values.show(name)) for \(op)") }
        let initial = try intOf(v["initial"], "initial"), target = try intOf(v["target"], "target")
        let lo = try intOf(v["min"], "min"), hi = try intOf(v["max"], "max")
        let r = try Shrink.shrinkInt(initial: initial, target: target, lo: lo, hi: hi, prop: prop, maxSteps: 1000)
        var problems: [String] = []
        if Value.int(r.value) != v["expected_minimal"] {
            problems.append("\(cid): \(op) minimal mismatch: got \(Values.show(r.value)), corpus \(Values.show(v["expected_minimal"]))")
        }
        if steps(r.steps) != v["expected_steps"] {
            problems.append("\(cid): \(op) step count mismatch: got \(r.steps), corpus \(Values.show(v["expected_steps"]))")
        }
        // Engine invariants, independent of the corpus's expected values.
        if prop(r.value) { problems.append("\(cid): \(op) minimal \(Values.show(r.value)) does not falsify the property") }
        if r.value < lo || r.value > hi {
            problems.append("\(cid): \(op) minimal \(Values.show(r.value)) left [\(Values.show(lo)), \(Values.show(hi))]")
        }
        return problems
    }

    private static func checkShrinkList(_ cid: String, _ v: [String: Value]) throws -> [String] {
        let name = try propName(v["property"])
        guard let prop = listProps[name] else { throw CaseError("unknown property \(Values.show(name)) for shrink_list") }
        guard case .list(let items)? = v["initial"] else { throw CaseError("initial must be a list of ints") }
        let initial = try items.map { try intOf($0, "element") }
        let elemTarget = try intOf(v["elem_target"], "elem_target")
        let r = try Shrink.shrinkList(initial: initial, elemTarget: elemTarget, prop: prop, maxSteps: 1000)
        var problems: [String] = []
        if Value.list(r.value.map { Value.int($0) }) != v["expected_minimal"] {
            problems.append("\(cid): shrink_list minimal mismatch: got \(Canon.render(r.value)), corpus \(Values.show(v["expected_minimal"]))")
        }
        if steps(r.steps) != v["expected_steps"] {
            problems.append("\(cid): shrink_list steps mismatch: got \(r.steps), corpus \(Values.show(v["expected_steps"]))")
        }
        if prop(r.value) { problems.append("\(cid): shrink_list minimal \(Values.show(r.value)) does not falsify the property") }
        return problems
    }

    private static func checkShrinkBytes(_ cid: String, _ v: [String: Value]) throws -> [String] {
        let name = try propName(v["property"])
        guard let prop = bytesProps[name] else { throw CaseError("unknown property \(Values.show(name)) for shrink_bytes") }
        guard case .bytes(let initial)? = v["initial"] else { throw CaseError("initial must be bytes") }
        let r = try Shrink.shrinkBytes(initial: initial, prop: prop, maxSteps: 1000)
        var problems: [String] = []
        if Value.bytes(r.value) != v["expected_minimal"] {
            problems.append("\(cid): shrink_bytes minimal mismatch: got \(Values.show(r.value)), corpus \(Values.show(v["expected_minimal"]))")
        }
        if steps(r.steps) != v["expected_steps"] {
            problems.append("\(cid): shrink_bytes steps mismatch: got \(r.steps), corpus \(Values.show(v["expected_steps"]))")
        }
        if prop(r.value) { problems.append("\(cid): shrink_bytes minimal \(Values.show(r.value)) does not falsify the property") }
        return problems
    }

    private static func checkReplay(_ cid: String, _ v: [String: Value]) throws -> [String] {
        let seed = try seedOf(v["seed"])
        let name = try propName(v["property"])
        guard let prop = intProps[name] else { throw CaseError("unknown property \(Values.show(name)) for replay_consistency") }
        let r = try Shrink.simulateReplay(seed: seed, prop: prop)
        var problems: [String] = []
        if Value.int(r.initialFail) != v["initial_fail"] {
            problems.append("\(cid): replay initial_fail mismatch: got \(Values.show(r.initialFail)), corpus \(Values.show(v["initial_fail"]))")
        }
        if Value.int(r.minimal) != v["expected_minimal"] {
            problems.append("\(cid): replay minimal mismatch: got \(Values.show(r.minimal)), corpus \(Values.show(v["expected_minimal"]))")
        }
        return problems
    }

    /// An unsatisfiable filter must end as `fail` / `property-exhausted`, having
    /// sampled exactly `max_retries` times.
    private static func checkFilterExhausted(_ cid: String, _ v: [String: Value]) throws -> [String] {
        let maxRetries = try sizeOf(v["max_retries"], "max_retries")
        if maxRetries < 1 { throw CaseError("max_retries must be at least 1") }
        final class Calls { var n = 0 }
        let calls = Calls()
        let gen = try Gens.filter(try Gens.integer(.zero, BigInt(10)), maxRetries: maxRetries) { _ in
            calls.n += 1
            return false
        }
        let verdict = try Runner.check(seed: 0, gen: gen, options: Options(numTests: 1, maxShrinkSteps: 1000, ci: .on)) { _ in true }
        var problems: [String] = []
        if Value.string("fail") != v["verdict"] || verdict.passed {
            problems.append("\(cid): an unsatisfiable filter must be a fail verdict")
        }
        if verdict.failureId.map({ Value.string($0) }) != v["failure_id"] {
            problems.append("\(cid): failure id \"\(verdict.failureId ?? "nil")\", corpus \(Values.show(v["failure_id"]))")
        }
        if calls.n != maxRetries { problems.append("\(cid): filter sampled \(calls.n) times, expected \(maxRetries)") }
        return problems
    }
}
