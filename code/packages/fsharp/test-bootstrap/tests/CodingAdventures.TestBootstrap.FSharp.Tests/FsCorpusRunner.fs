/// A second, independent pbt-v1 corpus harness (VER05l section 4).
///
/// It shares only DECODING with the C# engine (`StrictJson` via `Corpus.Parse`, `Values.Decode`,
/// `Values.Same`, `Canon.Render`, `TfvValidator`). It never calls `Corpus.Evaluate`: every op is
/// evaluated through the F# API, so a broken F# layer fails here even while the C# corpus test
/// (which exercises only C#) stays green.
///
/// The runner is strict, because a conformance check that cannot report a problem is worse than
/// none: a case must have exactly its op's fields, an unknown op is a problem (never a skip),
/// every op and all four failure ids must appear, an unregistered id must be rejected, and the
/// case count must be 25. Every problem about a case starts with that case's id.
module CodingAdventures.TestBootstrap.FSharp.Tests.FsCorpusRunner

open System
open System.Collections.Generic
open CodingAdventures.TestBootstrap.CorpusJson
open CodingAdventures.TestBootstrap.Tfv
open CodingAdventures.TestBootstrap.FSharp

type Report = { Evaluated: int; Problems: string list }

[<Literal>]
let CaseCount = 25

let private shapes : Map<string, string list> =
    Map.ofList [
        "gen_primitive", [ "type"; "seed"; "count"; "outputs" ]
        "gen_bounded", [ "min"; "max"; "seed"; "count"; "outputs" ]
        "gen_bytes", [ "min_len"; "max_len"; "seed"; "count"; "outputs" ]
        "shrink_int", [ "initial"; "target"; "min"; "max"; "property"; "expected_minimal"; "expected_steps" ]
        "shrink_refinement", [ "initial"; "target"; "min"; "max"; "property"; "expected_minimal"; "expected_steps" ]
        "shrink_list", [ "initial"; "elem_target"; "property"; "expected_minimal"; "expected_steps" ]
        "shrink_bytes", [ "initial"; "property"; "expected_minimal"; "expected_steps" ]
        "replay_consistency", [ "seed"; "property"; "initial_fail"; "expected_minimal" ]
        "filter_exhausted", [ "max_retries"; "verdict"; "failure_id" ]
        "failure_id_valid", [ "input"; "valid" ]
        "corpus_reject", [ "text" ]
    ]

// ---- the properties the corpus names (true means "holds") ------------------------------

let private intProps : Map<string, bigint -> bool> =
    Map.ofList [
        "int_le_100", (fun x -> x <= 100I)
        "int_in_range_neg10_50", (fun x -> x >= -10I && x <= 50I)
        "int_is_even", (fun x -> x.IsEven)
        "int_abs_le_10", (fun x -> abs x <= 10I)
        "refinement_x_le_50", (fun x -> x <= 50I)
    ]

let private listProps : Map<string, bigint list -> bool> =
    Map.ofList [
        "list_no_consecutive_duplicates", (fun l -> l |> List.pairwise |> List.forall (fun (a, b) -> a <> b))
        "list_sum_le_20", (fun l -> List.sum l <= 20I)
    ]

let private bytesProps : Map<string, byte[] -> bool> =
    Map.ofList [
        "bytes_len_lt_5", (fun b -> b.Length < 5)
        "bytes_no_zero_byte", (fun b -> not (Array.contains 0uy b))
    ]

// ---- decoding helpers --------------------------------------------------------------------

let private fail (msg: string) : 'a = raise (CaseException msg)

let private asInt (what: string) (v: obj) : bigint =
    match v with
    | :? bigint as b -> b
    | _ -> fail (what + " must be an int")

/// A corpus seed reduced modulo 2^64, as the reference masks it (negative seeds wrap).
let private asSeed (v: obj) : uint64 = uint64 (asInt "seed" v &&& bigint UInt64.MaxValue)

let private asSize (what: string) (v: obj) : int =
    let b = asInt what v
    if b.Sign < 0 || b > bigint Values.MaxSize then
        fail (sprintf "%s %s outside [0, %d]" what (Values.Show b) Values.MaxSize)
    else
        int b

let private asString (what: string) (v: obj) : string =
    match v with
    | :? string as s -> s
    | _ -> fail (what + " must be a string")

let private lookup (name: string) (table: Map<string, 'a>) (op: string) : 'a =
    match Map.tryFind name table with
    | Some p -> p
    | None -> fail (sprintf "unknown property %s for %s" (Values.Show name) op)

let private same (a: obj) (b: obj) : bool = Values.Same(a, b)

let private objects (xs: 'a seq) : obj = box (List<obj>(xs |> Seq.map box))

let private asBigList (v: obj) : bigint list =
    match v with
    | :? IList<obj> as items -> items |> Seq.map (asInt "element") |> List.ofSeq
    | _ -> fail "initial must be a list of ints"

// ---- per-op checks (each returns its problems) ---------------------------------------------

let private checkGen (cid: string) (op: string) (v: IDictionary<string, obj>) : string list =
    let seed = asSeed v.["seed"]
    let count = asSize "count" v.["count"]
    match v.["outputs"] with
    | :? IList<obj> as expected when expected.Count = count -> ()
    | _ -> fail (sprintf "outputs must be a list of exactly count (%d) values" count)
    let got : obj =
        match op with
        | "gen_primitive" ->
            match asString "type" v.["type"] with
            | "bool" -> objects (Generate.bools seed count)
            | "u8" -> objects (Generate.u8s seed count |> List.map (fun b -> bigint (int b)))
            | "u64" -> objects (Generate.u64s seed count |> List.map bigint)
            | "i128" -> objects (Generate.i128s seed count)
            | other -> fail (sprintf "unknown primitive type %s" other)
        | "gen_bounded" ->
            objects (Generate.bounded seed (asInt "min" v.["min"]) (asInt "max" v.["max"]) count)
        | _ ->
            let minLen = asSize "min_len" v.["min_len"]
            let maxLen = asSize "max_len" v.["max_len"]
            if int64 count * int64 maxLen > int64 (1 <<< 20) then
                fail (sprintf "count %d x max_len %d exceeds the generation budget" count maxLen)
            objects (Generate.bytes seed minLen maxLen count)
    if same got v.["outputs"] then [] else [ cid + ": " + op + " outputs mismatch" ]

let private stepAndMinimal (cid: string) (op: string) (minimalOk: bool) (shownGot: string) (steps: int) (v: IDictionary<string, obj>) =
    [ if not minimalOk then
          yield sprintf "%s: %s minimal mismatch: got %s, corpus %s" cid op shownGot (Values.Show v.["expected_minimal"])
      if not (same (box (bigint steps)) v.["expected_steps"]) then
          yield sprintf "%s: %s step count mismatch: got %d, corpus %s" cid op steps (Values.Show v.["expected_steps"]) ]

let private shrinkOrFail (r: Result<Shrunk<'a>, ShrinkError>) : Shrunk<'a> =
    match r with
    | Ok s -> s
    | Error NotFalsifying -> fail "the start value does not falsify the property"

let private checkShrinkInt (cid: string) (op: string) (v: IDictionary<string, obj>) : string list =
    let holds = lookup (asString "property" v.["property"]) intProps op
    let lo = asInt "min" v.["min"]
    let hi = asInt "max" v.["max"]
    let r = Shrink.integer (asInt "initial" v.["initial"]) (asInt "target" v.["target"]) lo hi holds 1000 |> shrinkOrFail
    stepAndMinimal cid op (same (box r.Value) v.["expected_minimal"]) (Values.Show r.Value) r.Steps v
    @ [ if holds r.Value then yield sprintf "%s: %s minimal %s does not falsify the property" cid op (Values.Show r.Value)
        if r.Value < lo || r.Value > hi then yield sprintf "%s: %s minimal %s left the bounds" cid op (Values.Show r.Value) ]

let private checkShrinkList (cid: string) (v: IDictionary<string, obj>) : string list =
    let holds = lookup (asString "property" v.["property"]) listProps "shrink_list"
    let r = Shrink.list (asBigList v.["initial"]) (asInt "elem_target" v.["elem_target"]) holds 1000 |> shrinkOrFail
    stepAndMinimal cid "shrink_list" (same (objects r.Value) v.["expected_minimal"]) (Canon.Render(List<bigint>(r.Value))) r.Steps v
    @ [ if holds r.Value then yield sprintf "%s: shrink_list minimal does not falsify the property" cid ]

let private checkShrinkBytes (cid: string) (v: IDictionary<string, obj>) : string list =
    let holds = lookup (asString "property" v.["property"]) bytesProps "shrink_bytes"
    let initial =
        match v.["initial"] with
        | :? (byte[]) as b -> b
        | _ -> fail "initial must be bytes"
    let r = Shrink.bytes initial holds 1000 |> shrinkOrFail
    stepAndMinimal cid "shrink_bytes" (same (box r.Value) v.["expected_minimal"]) (Values.Show r.Value) r.Steps v
    @ [ if holds r.Value then yield sprintf "%s: shrink_bytes minimal does not falsify the property" cid ]

let private checkReplay (cid: string) (v: IDictionary<string, obj>) : string list =
    let holds = lookup (asString "property" v.["property"]) intProps "replay_consistency"
    let r = Shrink.simulateReplay (asSeed v.["seed"]) holds
    [ if not (same (box r.InitialFail) v.["initial_fail"]) then
          yield sprintf "%s: replay initial_fail mismatch: got %s, corpus %s" cid (Values.Show r.InitialFail) (Values.Show v.["initial_fail"])
      if not (same (box r.Minimal) v.["expected_minimal"]) then
          yield sprintf "%s: replay minimal mismatch: got %s, corpus %s" cid (Values.Show r.Minimal) (Values.Show v.["expected_minimal"]) ]

let private checkFilterExhausted (cid: string) (v: IDictionary<string, obj>) : string list =
    let maxRetries = asSize "max_retries" v.["max_retries"]
    if maxRetries < 1 then fail "max_retries must be at least 1"
    let mutable calls = 0
    let g =
        Gen.integer 0I 10I
        |> Gen.filterWith maxRetries (fun _ ->
            calls <- calls + 1
            false)
    let verdict = Prop.check 0UL { NumTests = 1; MaxShrinkSteps = 1000; Ci = On } g (fun _ -> true)
    let id =
        match verdict with
        | Exhausted f | Failed f -> f.FailureId
        | Passed _ -> null
    [ if not (same "fail" v.["verdict"]) || Verdict.isPassed verdict then
          yield cid + ": an unsatisfiable filter must be a fail verdict"
      if not (same (box id) v.["failure_id"]) then
          yield sprintf "%s: failure id \"%s\", corpus %s" cid id (Values.Show v.["failure_id"])
      if calls <> maxRetries then
          yield sprintf "%s: filter sampled %d times, expected %d" cid calls maxRetries ]

type private Eval = { Problems: string list; RanOk: bool; Candidate: string; Registered: bool }

let private evaluateCase (cid: string) (op: string) (c: Dictionary<string, obj>) : Eval =
    try
        let v = Dictionary<string, obj>()
        for kv in c do
            if kv.Key <> "id" && kv.Key <> "op" then
                v.[kv.Key] <- Values.Decode kv.Value
        let ok problems = { Problems = problems; RanOk = true; Candidate = ""; Registered = false }
        match op with
        | "gen_primitive" | "gen_bounded" | "gen_bytes" -> ok (checkGen cid op v)
        | "shrink_int" | "shrink_refinement" -> ok (checkShrinkInt cid op v)
        | "shrink_list" -> ok (checkShrinkList cid v)
        | "shrink_bytes" -> ok (checkShrinkBytes cid v)
        | "replay_consistency" -> ok (checkReplay cid v)
        | "filter_exhausted" -> ok (checkFilterExhausted cid v)
        | "failure_id_valid" ->
            let input = match v.["input"] with :? string as s -> s | _ -> null
            let got = not (isNull input) && FailureId.isValid input
            let problems =
                if same (box got) v.["valid"] then []
                else [ cid + ": failure id membership disagrees for " + Values.Show v.["input"] ]
            { Problems = problems; RanOk = true; Candidate = (if isNull input then "" else input); Registered = got }
        | _ ->
            let text = asString "text" v.["text"]
            ok (if TfvValidator.IsValid text then [ cid + ": corpus_reject text was accepted by the strict parser" ] else [])
    with e ->
        { Problems = [ sprintf "%s: %s could not be evaluated: %s" cid op (Clip.Apply(e.Message, Values.MaxFragmentLen)) ]
          RanOk = false
          Candidate = ""
          Registered = false }

/// Evaluates every case through the F# API. A negative `expected` skips the case-count check.
let evaluateWith (cases: obj list) (expected: int) : Report =
    let problems = List<string>()
    let mutable evaluated = 0
    if expected >= 0 && cases.Length <> expected then
        problems.Add(sprintf "pbt-v1: expected %d cases, got %d" expected cases.Length)
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
    let opsSeen = HashSet<string>()
    let registeredSeen = HashSet<string>()
    let mutable rejectedIdSeen = false
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
                        let ev = evaluateCase cid op c
                        problems.AddRange ev.Problems
                        if ev.RanOk then
                            if op = "failure_id_valid" then
                                if ev.Registered then registeredSeen.Add ev.Candidate |> ignore
                                else rejectedIdSeen <- true
                            opsSeen.Add op |> ignore
                            evaluated <- evaluated + 1
        | other -> problems.Add("case is not an object: " + Values.Show other)
    let missingOps = shapes |> Map.toList |> List.map fst |> List.filter (opsSeen.Contains >> not)
    if not missingOps.IsEmpty then
        problems.Add("pbt-v1: operations never exercised: [" + String.concat ", " missingOps + "]")
    let missingIds = FailureId.all |> List.filter (registeredSeen.Contains >> not)
    if not missingIds.IsEmpty then
        problems.Add("pbt-v1: registered failure ids missing: [" + String.concat ", " missingIds + "]")
    if not rejectedIdSeen then
        problems.Add "pbt-v1: failure_id_valid needs an unregistered-id rejection"
    { Evaluated = evaluated
      Problems = problems |> Seq.map (fun p -> Clip.Apply(p, Values.MaxProblemLen)) |> List.ofSeq }

/// Evaluates with the pbt-v1 case count.
let evaluate (cases: obj list) : Report = evaluateWith cases CaseCount
