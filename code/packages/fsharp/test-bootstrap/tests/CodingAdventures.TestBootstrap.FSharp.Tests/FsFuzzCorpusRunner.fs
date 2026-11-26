/// A second, independent fuzz-v1 corpus harness (VER06m section 3).
///
/// It shares only DECODING with the C# engine (`FuzzCorpus.Parse` for the envelope, `Values.Decode`,
/// `Values.Same`, `Values.Show`, `Canon.Render`).  It never calls `FuzzCorpus.Evaluate`: every op is
/// answered by the F# wrapper (`Mutate.*`, `Admission.admit`, `Signature.*`, `FuzzFailureId.isValid`,
/// `Tap.format`, `FuzzTfv.isValid`), so a broken F# layer fails here even while the C# corpus test
/// (which exercises only C#) stays green.
///
/// A conformance check that cannot report a problem is worse than none, so the runner is strict: a
/// case has exactly its op's fields, an unknown op is a problem (never a skip), all 14 ops appear, an
/// accepted and a rejected failure id appear, the case count is 25, fixture sizes are capped
/// (CWE-400) and every problem about a case starts with that case's id.
module CodingAdventures.TestBootstrap.FSharp.Tests.FsFuzzCorpusRunner

open System
open System.Collections.Generic
open CodingAdventures.TestBootstrap.CorpusJson
open CodingAdventures.TestBootstrap.FSharp.Fuzzing

type Report = { Evaluated: int; Problems: string list }

[<Literal>]
let CaseCount = 25

/// The aggregate `mutate_sequence` work budget of one corpus (VER06m section 6.5).
let MaxAggregateWork = 1L <<< 24

let private maxCases = 10_000

let private shapes : Map<string, string list> =
    Map.ofList [
        "mutate_bit_flip", [ "input"; "bit_offset"; "count"; "output" ]
        "mutate_byte_flip", [ "input"; "byte_offset"; "count"; "output" ]
        "mutate_arithmetic", [ "input"; "byte_offset"; "delta"; "width"; "endian"; "output" ]
        "mutate_boundary", [ "input"; "byte_offset"; "width"; "val"; "endian"; "output" ]
        "mutate_block_delete", [ "input"; "offset"; "length"; "output" ]
        "mutate_block_insert", [ "input"; "offset"; "insert_bytes"; "output" ]
        "mutate_block_duplicate", [ "input"; "src_offset"; "length"; "dst_offset"; "output" ]
        "mutate_splice", [ "target"; "donor"; "target_split"; "donor_split"; "output" ]
        "mutate_sequence", [ "input"; "seed"; "steps"; "output" ]
        "corpus_admit", [ "existing_edges"; "trace_edges"; "admitted"; "new_edges" ]
        "crash_dedup", [ "crash_a_kind"; "crash_a_frames"; "crash_b_kind"; "crash_b_frames"; "is_duplicate" ]
        "failure_id_valid", [ "input"; "valid" ]
        "private_reporting", [ "target"; "failure_id"; "raw_payload"; "sig_hash"; "tap_line"; "contains_raw_payload" ]
        "corpus_reject", [ "text" ]
    ]

// ---- decoding helpers ----------------------------------------------------------------------

let private fail (msg: string) : 'a = raise (CaseException msg)

let private asBytes (what: string) (v: obj) : byte[] =
    match v with
    | :? (byte[]) as b -> b
    | _ -> fail (what + " must be bytes")

let private asText (what: string) (v: obj) : string =
    match v with
    | :? string as s -> s
    | _ -> fail (what + " must be a string")

let private asBool (what: string) (v: obj) : bool =
    match v with
    | :? bool as b -> b
    | _ -> fail (what + " must be a bool")

let private asInt (what: string) (v: obj) : bigint =
    match v with
    | :? bigint as b -> b
    | _ -> fail (what + " must be an int")

let private asSize (what: string) (v: obj) : int =
    let b = asInt what v
    if b.Sign < 0 || b > bigint Values.MaxSize then
        fail (sprintf "%s %s outside [0, %d]" what (Values.Show b) Values.MaxSize)
    else
        int b

/// A corpus width; anything outside [0, 64] becomes -1, which the engine treats as "not 1, 2, 4 or 8".
let private asWidth (v: obj) : int =
    let b = asInt "width" v
    if b.Sign < 0 || b > 64I then -1 else int b

let private asEndian (v: obj) : Endian =
    match asText "endian" v with
    | "little" -> Endian.Little
    | "big" -> Endian.Big
    | other -> fail ("endian must be 'little' or 'big', got " + Values.Show other)

let private asTexts (what: string) (v: obj) : string list =
    match v with
    | :? IList<obj> as items ->
        items |> Seq.map (fun i -> match i with :? string as s -> s | _ -> fail (what + " must be a list of strings")) |> List.ofSeq
    | _ -> fail (what + " must be a list of strings")

let private two64 = bigint.Pow(2I, 64)

let private asEdges (what: string) (v: obj) : uint64 list =
    match v with
    | :? IList<obj> as items ->
        items
        |> Seq.map (fun i ->
            match i with
            | :? bigint as b ->
                if b.Sign < 0 || b >= two64 then fail (sprintf "%s element %s outside [0, 2^64)" what (Values.Show b))
                else uint64 b
            | _ -> fail (what + " must be a list of ints"))
        |> List.ofSeq
    | _ -> fail (what + " must be a list of ints")

let private same (a: obj) (b: obj) : bool = Values.Same(a, b)

let private lowerBool (b: bool) : string = if b then "true" else "false"

let private compare (cid: string) (op: string) (got: byte[]) (v: IDictionary<string, obj>) : string list =
    match v.["output"] with
    | :? (byte[]) as want ->
        if got = want then []
        else
            [ sprintf "%s: %s expected %s, got %s" cid op
                  (Clip.Apply(FuzzSupport.hex want, Values.MaxFragmentLen))
                  (Clip.Apply(FuzzSupport.hex got, Values.MaxFragmentLen)) ]
    | _ -> [ cid + ": " + op + " output must be bytes" ]

let private isHex8 (s: string) = s.Length = 8 && s |> Seq.forall (fun c -> (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'))

/// `work` is the remaining aggregate budget of the corpus (a one-element cell).
let private checkMutator (cid: string) (op: string) (v: IDictionary<string, obj>) (work: int64[]) : string list =
    let input () = asBytes "input" v.["input"]
    let int' name = asInt name v.[name]
    let got : byte[] =
        match op with
        | "mutate_bit_flip" -> Mutate.bitFlip (input ()) (int' "bit_offset") (int' "count")
        | "mutate_byte_flip" -> Mutate.byteFlip (input ()) (int' "byte_offset") (int' "count")
        | "mutate_arithmetic" ->
            Mutate.byteArithmetic (input ()) (int' "byte_offset") (int' "delta") (asWidth v.["width"]) (asEndian v.["endian"])
        | "mutate_boundary" ->
            Mutate.boundaryInsert (input ()) (int' "byte_offset") (asWidth v.["width"]) (int' "val") (asEndian v.["endian"])
        | "mutate_block_delete" -> Mutate.blockDelete (input ()) (int' "offset") (int' "length")
        | "mutate_block_insert" -> Mutate.blockInsert (input ()) (int' "offset") (asBytes "insert_bytes" v.["insert_bytes"])
        | "mutate_block_duplicate" ->
            Mutate.blockDuplicate (input ()) (int' "src_offset") (int' "length") (int' "dst_offset")
        | "mutate_splice" ->
            Mutate.splice (asBytes "target" v.["target"]) (asBytes "donor" v.["donor"]) (int' "target_split") (int' "donor_split")
        | _ ->
            // mutate_sequence.  The public call caps one call only, so the runner keeps its own
            // aggregate: an upper bound is charged BEFORE the call, and a breached budget fails every
            // later case without running it.
            let data = input ()
            let seed = uint64 (int' "seed" &&& bigint UInt64.MaxValue)
            let steps = asSize "steps" v.["steps"]
            if work.[0] <= 0L then
                fail (sprintf "mutate_sequence work budget of %d byte-steps exhausted" MaxAggregateWork)
            let maxLen = data.Length + 4096
            let charge = int64 data.Length + int64 steps * int64 maxLen
            if charge > work.[0] then
                work.[0] <- 0L
                fail (sprintf "mutate_sequence work budget of %d byte-steps exhausted" MaxAggregateWork)
            work.[0] <- work.[0] - charge
            Mutate.sequence data seed steps maxLen
    let problems = compare cid op got v
    // Engine invariants, independent of the corpus's expected bytes.
    let lenProblems =
        match op with
        | "mutate_bit_flip" | "mutate_byte_flip" | "mutate_arithmetic" | "mutate_boundary"
            when got.Length <> (asBytes "input" v.["input"]).Length ->
            [ cid + ": " + op + " changed the length of its input" ]
        | "mutate_splice" ->
            let t = asBytes "target" v.["target"]
            let d = asBytes "donor" v.["donor"]
            let clamp (s: bigint) (n: int) = if s.Sign < 0 then 0 else int (bigint.Min(s, bigint n))
            let want = clamp (int' "target_split") t.Length + d.Length - clamp (int' "donor_split") d.Length
            if got.Length <> want then [ cid + ": splice length is not prefix + suffix" ] else []
        | _ -> []
    problems @ lenProblems

let private checkAdmit (cid: string) (v: IDictionary<string, obj>) : string list =
    let existing = asEdges "existing_edges" v.["existing_edges"]
    let trace = asEdges "trace_edges" v.["trace_edges"]
    let wantAdmitted = asBool "admitted" v.["admitted"]
    let wantNew = asInt "new_edges" v.["new_edges"]
    let got = Admission.admit existing trace
    let have = Set.ofList existing
    [ if got.Admitted <> wantAdmitted then
          yield sprintf "%s: corpus_admit admitted %s, corpus %s" cid (lowerBool got.Admitted) (lowerBool wantAdmitted)
      if not (same (box (bigint got.NewEdges)) (box wantNew)) then
          yield sprintf "%s: corpus_admit new_edges %d, corpus %s" cid got.NewEdges (Values.Show wantNew)
      // Invariant: admission is exactly "some trace edge is unseen".
      if got.Admitted <> (trace |> List.exists (fun e -> not (have.Contains e))) then
          yield cid + ": corpus_admit disagrees with set difference" ]

let private checkDedup (cid: string) (v: IDictionary<string, obj>) : string list =
    let ka = asText "crash_a_kind" v.["crash_a_kind"]
    let fa = asTexts "crash_a_frames" v.["crash_a_frames"]
    let kb = asText "crash_b_kind" v.["crash_b_kind"]
    let fb = asTexts "crash_b_frames" v.["crash_b_frames"]
    let want = asBool "is_duplicate" v.["is_duplicate"]
    let got = Signature.isDuplicate ka fa kb fb
    // The signature hashes themselves must agree with the verdict and be 8 lowercase hex digits.
    let sa = Signature.crash ka fa
    let sb = Signature.crash kb fb
    [ if got <> want then
          yield sprintf "%s: crash_dedup is_duplicate %s, corpus %s" cid (lowerBool got) (lowerBool want)
      if (sa = sb) <> want then yield cid + ": crash_dedup signature hashes disagree with is_duplicate"
      if not (isHex8 sa && isHex8 sb) then yield cid + ": crash_dedup signature is not 8 lowercase hex digits" ]

let private checkPrivate (cid: string) (v: IDictionary<string, obj>) : string list =
    let raw = asBytes "raw_payload" v.["raw_payload"]
    let target = asText "target" v.["target"]
    let fid = asText "failure_id" v.["failure_id"]
    let sigHash = asText "sig_hash" v.["sig_hash"]
    let tap = asText "tap_line" v.["tap_line"]
    let wantLeak = asBool "contains_raw_payload" v.["contains_raw_payload"]
    let line =
        try
            Tap.format 1 target fid sigHash
        with :? ArgumentException as e -> fail ("format_tap_failure refused: " + e.Message)
    let leaked = raw.Length > 0 && line.Contains(FuzzSupport.hex raw, StringComparison.Ordinal)
    [ if line <> tap then
          yield sprintf "%s: tap_line expected %s, got %s" cid (Clip.Apply(tap, Values.MaxFragmentLen)) (Clip.Apply(line, Values.MaxFragmentLen))
      if leaked <> wantLeak then yield sprintf "%s: privacy verdict wrong, payload leaked: %s" cid (lowerBool leaked)
      if line.Contains '\r' || line.Contains '\n' then yield cid + ": failure line spans more than one line" ]

type private Eval = { Found: string list; RanOk: bool; Accepted: bool }

let private evaluateCase (cid: string) (op: string) (c: Dictionary<string, obj>) (work: int64[]) : Eval =
    try
        let v = Dictionary<string, obj>()
        for kv in c do
            if kv.Key <> "id" && kv.Key <> "op" then
                v.[kv.Key] <- Values.Decode kv.Value
        let ok problems = { Found = problems; RanOk = true; Accepted = false }
        match op with
        | "corpus_admit" -> ok (checkAdmit cid v)
        | "crash_dedup" -> ok (checkDedup cid v)
        | "failure_id_valid" ->
            let input = match v.["input"] with :? string as s -> s | _ -> null
            let got = not (isNull input) && FuzzFailureId.isValid input
            let problems =
                if same (box got) v.["valid"] then []
                else [ cid + ": failure id membership disagrees for " + Values.Show v.["input"] ]
            { Found = problems; RanOk = true; Accepted = got }
        | "private_reporting" -> ok (checkPrivate cid v)
        | "corpus_reject" ->
            let text = asText "text" v.["text"]
            ok (if FuzzTfv.isValid text then [ cid + ": corpus_reject text was accepted by the strict parser" ] else [])
        | _ -> ok (checkMutator cid op v work)
    with e ->
        { Found = [ sprintf "%s: %s could not be evaluated: %s" cid op (Clip.Apply(e.Message, Values.MaxFragmentLen)) ]
          RanOk = false
          Accepted = false }

/// Evaluates every case through the F# API. A negative `expected` skips the case-count check.
let evaluateWith (casesIn: obj list) (expected: int) : Report =
    let problems = List<string>()
    let mutable evaluated = 0
    let mutable cases = casesIn
    if expected >= 0 && cases.Length <> expected then
        problems.Add(sprintf "fuzz-v1: expected %d cases, got %d" expected cases.Length)
    if cases.Length > maxCases then
        problems.Add(sprintf "fuzz-v1: %d cases exceeds the cap of %d" cases.Length maxCases)
        cases <- List.truncate maxCases cases
    let idOf (raw: obj) =
        match raw with
        | :? Dictionary<string, obj> as m ->
            match m.TryGetValue "id" with
            | true, (:? string as s) when s <> "" -> Some s
            | _ -> None
        | _ -> None
    let ids = cases |> List.map idOf
    if ids |> List.exists Option.isNone || (ids |> List.choose id |> List.distinct |> List.length) <> ids.Length then
        problems.Add "duplicate or missing case id"
    let work = [| MaxAggregateWork |]
    let opsSeen = HashSet<string>()
    let mutable acceptedSeen = false
    let mutable rejectedSeen = false
    for raw in cases do
        match raw with
        | :? Dictionary<string, obj> as c ->
            match idOf raw with
            | None -> problems.Add("case missing a string id: " + Values.Show c)
            | Some rawId ->
                let cid = Clip.Apply(rawId, 100)
                let op = match c.TryGetValue "op" with | true, (:? string as s) -> s | _ -> ""
                match Map.tryFind op shapes with
                | None -> problems.Add(cid + ": unknown op " + Values.Show(match c.TryGetValue "op" with | true, x -> x | _ -> null))
                | Some fields ->
                    if c.Count <> fields.Length + 2 || not (fields |> List.forall c.ContainsKey) then
                        let keys = c.Keys |> Seq.sortWith (fun a b -> String.CompareOrdinal(a, b)) |> String.concat ", "
                        problems.Add(sprintf "%s: fields [%s] != [%s]" cid keys (String.concat ", " ("id" :: "op" :: fields)))
                    else
                        let ev = evaluateCase cid op c work
                        problems.AddRange ev.Found
                        if ev.RanOk then
                            if op = "failure_id_valid" then
                                if ev.Accepted then acceptedSeen <- true else rejectedSeen <- true
                            opsSeen.Add op |> ignore
                            evaluated <- evaluated + 1
        | other -> problems.Add("case is not an object: " + Values.Show other)
    let missingOps = shapes |> Map.toList |> List.map fst |> List.filter (opsSeen.Contains >> not) |> List.sortWith (fun a b -> String.CompareOrdinal(a, b))
    if not missingOps.IsEmpty then
        problems.Add("fuzz-v1: operations never exercised: [" + String.concat ", " missingOps + "]")
    if not acceptedSeen then problems.Add "fuzz-v1: failure_id_valid needs a registered-id acceptance"
    if not rejectedSeen then problems.Add "fuzz-v1: failure_id_valid needs an unregistered-id rejection"
    let unregistered = FuzzFailureId.all |> List.filter (FuzzFailureId.isValid >> not)
    if not unregistered.IsEmpty then
        problems.Add("fuzz-v1: engine rejects registered ids [" + String.concat ", " unregistered + "]")
    { Evaluated = evaluated
      Problems = problems |> Seq.map (fun p -> Clip.Apply(p, Values.MaxProblemLen)) |> List.ofSeq }

/// Evaluates with the fuzz-v1 case count.
let evaluate (cases: obj list) : Report = evaluateWith cases CaseCount
