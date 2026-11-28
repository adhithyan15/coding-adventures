open Rank_statistics

let check = Alcotest.check
let float = Alcotest.float 0.0

let unwrap = function
  | Ok value -> value
  | Error message -> Alcotest.fail message

let rec popcount value =
  if value = 0 then 0 else (value land 1) + popcount (value lsr 1)

let test_medians () =
  let values = [| 4.; 1.; 3.; 2. |] in
  check float "even median" 2.5 (unwrap (median values));
  check (Alcotest.array float) "median leaves input unchanged"
    [| 4.; 1.; 3.; 2. |] values;
  let d = Int64.float_of_bits 1L in
  check float "positive subnormal midpoint" (2. *. d)
    (unwrap (median [| d; 2. *. d |]));
  check float "negative subnormal midpoint" (-2. *. d)
    (unwrap (median [| -2. *. d; -.d |]));
  check float "normal boundary tie" Float.min_float
    (unwrap (median [| Float.pred Float.min_float; Float.min_float |]));
  check float "negative normal boundary tie" (-.Float.min_float)
    (unwrap (median [| -.Float.min_float; -.Float.pred Float.min_float |]));
  check float "opposite maximum values" 0.
    (unwrap (median [| -.max_float; max_float |]));
  check float "same maximum values" max_float
    (unwrap (median [| max_float; max_float |]));
  Alcotest.check Alcotest.bool "empty sample rejected" true
    (Result.is_error (median [||]));
  Alcotest.check Alcotest.bool "NaN rejected" true
    (Result.is_error (median [| nan |]))

let exact_oracle n confidence =
  let cutoff = (100 - confidence) * (1 lsl n) in
  let rank = ref 0 and searching = ref true in
  for k = 1 to n / 2 do
    if !searching then (
      let tail = ref 0 in
      for mask = 0 to (1 lsl n) - 1 do
        if popcount mask < k then incr tail
      done;
      if 200 * !tail <= cutoff then rank := k else searching := false)
  done;
  !rank

let test_exact_cutoff_oracle () =
  for n = 1 to 12 do
    for confidence = 1 to 99 do
      let expected = exact_oracle n confidence in
      let actual = unwrap (exact_median_rank n confidence) in
      Alcotest.check Alcotest.int
        (Printf.sprintf "n=%d confidence=%d" n confidence)
        expected actual
    done
  done

let test_wide_frozen_endpoints () =
  let cases =
    [
      (64, 95, 23, 40);
      (65, 99, 21, 43);
      (127, 95, 51, 75);
      (128, 95, 52, 75);
      (256, 99, 106, 149);
      (1000, 95, 468, 531);
      (10000, 50, 4965, 5034);
      (10000, 95, 4901, 5098);
      (10000, 99, 4870, 5129);
    ]
  in
  List.iter
    (fun (n, confidence, low, high) ->
      let sample = Array.init n float_of_int in
      match unwrap (median_interval sample confidence) with
      | None -> Alcotest.fail "wide sample unexpectedly had no interval"
      | Some actual ->
          check float "frozen low endpoint" (float_of_int low) actual.low;
          check float "frozen high endpoint" (float_of_int high) actual.high)
    cases

let test_seeded_order_properties () =
  let state = ref 0x5eed in
  for _trial = 1 to 500 do
    state := ((!state * 1103515245) + 12345) land 0x7fffffff;
    let n = 1 + (!state mod 96) in
    let values =
      Array.init n (fun i ->
          float_of_int ((((i * 37) + (!state * 13)) mod 2001) - 1000))
    in
    let original = Array.copy values in
    let center = unwrap (median values) in
    let interval = unwrap (median_interval values 90) in
    check (Alcotest.array float) "input unmodified" original values;
    Alcotest.check Alcotest.bool "center between extrema" true
      (center >= Array.fold_left min infinity values
      && center <= Array.fold_left max neg_infinity values);
    Option.iter
      (fun ci ->
        Alcotest.check Alcotest.bool "interval contains median" true
          (ci.low <= center && center <= ci.high))
      interval
  done

let () =
  Alcotest.run "rank-statistics"
    [
      ("median", [ Alcotest.test_case "edges" `Quick test_medians ]);
      ( "exact cutoff",
        [
          Alcotest.test_case "sign enumeration" `Quick test_exact_cutoff_oracle;
          Alcotest.test_case "wide endpoints" `Quick test_wide_frozen_endpoints;
        ] );
      ( "generated",
        [
          Alcotest.test_case "seeded order properties" `Quick
            test_seeded_order_properties;
        ] );
    ]
