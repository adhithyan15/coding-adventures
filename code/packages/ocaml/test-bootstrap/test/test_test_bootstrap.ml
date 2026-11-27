(* Tests for the OCaml lane of VER05 (VER05q).  Part 1 replays the complete
   pbt-v1 corpus (embedded in corpus_data.ml, so the tests touch no filesystem),
   with negative controls.  Parts 2 onward pin what the corpus cannot catch and
   exercise every OCaml-specific hazard of VER05q section 9. *)

open Coding_adventures_test_bootstrap
open Types
module B = Bigint
module J = Strict_json

let bi = B.of_string
let bs = B.to_string
let u64s w = Printf.sprintf "%Lu" w
let i64 s = Int64.of_string ("0u" ^ s)
let cfg = Types.default_config
let check_s = Alcotest.(check string)
let check_i = Alcotest.(check int)
let check_b = Alcotest.(check bool)
let tc name f = Alcotest.test_case name `Quick f

let opts ?(n = 100) ?(steps = 1000) ?(ci = Types.Ci_off) () =
  { Types.default_options with num_tests = n; max_shrink_steps = steps; ci }

let fid (v : Types.verdict) = Option.value v.failure_id ~default:"<pass>"

(* Spin (bounded) until a flag is set, so a bug fails instead of hanging. *)
let wait_for flag =
  let spins = ref 0 in
  while (not (Atomic.get flag)) && !spins < 200_000_000 do
    incr spins;
    Domain.cpu_relax ()
  done;
  Alcotest.(check bool) "handshake completed" true (Atomic.get flag)

let contains hay needle =
  let n = String.length needle and h = String.length hay in
  let rec go i = i + n <= h && (String.sub hay i n = needle || go (i + 1)) in
  go 0

(* ===================================================================== *)
(* Bigint                                                                  *)
(* ===================================================================== *)

let test_bigint_vectors () =
  List.iter
    (fun (a, b, sum, diff, prod, q, r) ->
      let x = bi a and y = bi b in
      check_s ("add " ^ a ^ " " ^ b) sum (bs (B.add x y));
      check_s ("sub " ^ a ^ " " ^ b) diff (bs (B.sub x y));
      check_s ("mul " ^ a ^ " " ^ b) prod (bs (B.mul x y));
      let q', r' = B.div_rem x y in
      check_s ("quot " ^ a ^ " " ^ b) q (bs q');
      check_s ("rem " ^ a ^ " " ^ b) r (bs r'))
    Vectors.arith_vectors

let test_bigint_misc () =
  check_s "zero" "0" (bs B.zero);
  check_s "neg zero" "0" (bs (B.neg B.zero));
  check_s "-0 parses to 0" "0" (bs (bi "-0"));
  check_s "leading zeros tolerated by of_string" "12" (bs (bi "0012"));
  check_s "min_int" (string_of_int min_int) (bs (B.of_int min_int));
  check_s "max_int" (string_of_int max_int) (bs (B.of_int max_int));
  Alcotest.(check (option int))
    "to_int_opt min_int" (Some min_int)
    (B.to_int_opt (B.of_int min_int));
  Alcotest.(check (option int))
    "to_int_opt max_int" (Some max_int)
    (B.to_int_opt (B.of_int max_int));
  Alcotest.(check (option int))
    "to_int_opt 2^62" None
    (B.to_int_opt (B.pow2 62));
  Alcotest.(check (option int))
    "to_int_opt -2^62-1" None
    (B.to_int_opt (B.sub (B.of_int min_int) B.one));
  Alcotest.(check (option int)) "to_int_opt 0" (Some 0) (B.to_int_opt B.zero);
  Alcotest.(check (option int))
    "to_int_opt -5" (Some (-5))
    (B.to_int_opt (B.of_int (-5)));
  Alcotest.(check (option int))
    "to_int_opt huge" None
    (B.to_int_opt (bi "123456789012345678901234567890"));
  check_s "of_uint64 -1" "18446744073709551615" (bs (B.of_uint64 (-1L)));
  check_s "of_int64 min" "-9223372036854775808" (bs (B.of_int64 Int64.min_int));
  check_s "of_int64 max" "9223372036854775807" (bs (B.of_int64 Int64.max_int));
  check_s "of_int64 -1" "-1" (bs (B.of_int64 (-1L)));
  check_s "of_int64 7" "7" (bs (B.of_int64 7L));
  check_s "wrap 2^64+5" "5" (u64s (B.to_int64_wrap (bi "18446744073709551621")));
  check_s "wrap -1" "18446744073709551615"
    (u64s (B.to_int64_wrap (B.of_int (-1))));
  check_s "wrap 2^63" "9223372036854775808" (u64s (B.to_int64_wrap (B.pow2 63)));
  check_s "wrap 0" "0" (u64s (B.to_int64_wrap B.zero));
  check_i "bit_length 0" 0 (B.bit_length B.zero);
  check_i "bit_length 1" 1 (B.bit_length B.one);
  check_i "bit_length 2^64" 65 (B.bit_length (B.pow2 64));
  check_i "bit_length 2^64-1" 64 (B.bit_length (B.sub (B.pow2 64) B.one));
  check_i "bit_length -2^30" 31 (B.bit_length (B.neg (B.pow2 30)));
  check_s "shift_left" "1267650600228229401496703205376"
    (bs (B.shift_left B.one 100));
  check_s "shift_right" "1" (bs (B.shift_right (B.pow2 100) 100));
  check_s "shift_right past width" "0" (bs (B.shift_right (B.pow2 100) 200));
  check_s "shift_right keeps sign" "-3" (bs (B.shift_right (B.of_int (-7)) 1));
  check_s "fdiv2 -81" "-41" (bs (B.fdiv2 (B.of_int (-81))));
  check_s "fdiv2 -80" "-40" (bs (B.fdiv2 (B.of_int (-80))));
  check_s "fdiv2 -1" "-1" (bs (B.fdiv2 (B.of_int (-1))));
  check_s "fdiv2 81" "40" (bs (B.fdiv2 (B.of_int 81)));
  check_s "rem_pos negative" "2" (bs (B.rem_pos (B.of_int (-7)) (B.of_int 3)));
  check_s "rem_pos big" "1"
    (bs (B.rem_pos (B.sub (B.pow2 100) B.one) (B.of_int 2)));
  check_i "sign" (-1) (B.sign (B.of_int (-9)));
  check_s "abs" "9" (bs (B.abs (B.of_int (-9))));
  check_b "equal" true (B.equal (bi "5") (B.of_int 5));
  check_b "compare neg/pos" true (B.compare (B.of_int (-5)) (B.of_int 5) < 0);
  check_b "compare neg/neg" true (B.compare (B.of_int (-5)) (B.of_int (-7)) > 0);
  check_b "compare zero" true (B.compare B.zero B.zero = 0);
  check_b "add cancels" true (B.sign (B.add (B.of_int 5) (B.of_int (-5))) = 0);
  check_b "add zero left" true
    (B.equal (B.add B.zero (B.of_int 5)) (B.of_int 5));
  check_b "add mixed sign small wins" true
    (B.equal (B.add (B.of_int 3) (B.of_int (-5))) (B.of_int (-2)));
  Alcotest.check_raises "div by zero" Division_by_zero (fun () ->
      ignore (B.div_rem B.one B.zero));
  check_s "0 / x" "0" (bs (fst (B.div_rem B.zero (B.of_int 5))));
  Alcotest.check_raises "negative shift" (Invalid_argument "Bigint.shift_left")
    (fun () -> ignore (B.shift_left B.one (-1)));
  Alcotest.check_raises "negative shift right"
    (Invalid_argument "Bigint.shift_right") (fun () ->
      ignore (B.shift_right B.one (-1)))

let test_bigint_parse_errors () =
  let bad s =
    try
      ignore (bi s);
      false
    with Invalid_argument _ -> true
  in
  check_b "empty" true (bad "");
  check_b "just minus" true (bad "-");
  check_b "letters" true (bad "12a");
  check_b "space" true (bad "1 2");
  check_b "too many digits" true (bad (String.make 100_001 '7'));
  check_b "100000 digits ok" false (bad (String.make 1000 '7'))

(* Division identity a = q*b + r with |r| < |b|, on values spanning many limbs. *)
let test_bigint_identities () =
  let rng = Prng.create 2024L in
  let rand_big bits =
    let r = ref B.zero in
    for _ = 1 to (bits / 64) + 1 do
      r := B.add (B.shift_left !r 64) (B.of_uint64 (Prng.next_u64 rng))
    done;
    if Prng.bounded_u64 rng 2L = 1L then B.neg !r else !r
  in
  for i = 1 to 200 do
    let a = rand_big (50 + (i * 7)) and b = rand_big (1 + (i mod 190)) in
    if B.sign b <> 0 then (
      let q, r = B.div_rem a b in
      check_b "a = q*b + r" true (B.equal a (B.add (B.mul q b) r));
      check_b "|r| < |b|" true (B.compare (B.abs r) (B.abs b) < 0);
      check_b "round trip" true (B.equal a (bi (bs a))))
  done

(* ===================================================================== *)
(* Prng                                                                    *)
(* ===================================================================== *)

let test_prng_basics () =
  check_s "seed 0 first word" "e220a8397b1dcdaf"
    (Printf.sprintf "%Lx" (Prng.next_u64 (Prng.create 0L)));
  check_s "fnv empty" "14695981039346656037" (u64s (Prng.fnv1a64 ""));
  check_s "fnv a" "12638187200555641996" (u64s (Prng.fnv1a64 "a"));
  check_s "fnv prop/x" "18085136578129828119" (u64s (Prng.fnv1a64 "prop/x"));
  check_s "sub_seed 0 a" "6857225946766476583" (u64s (Prng.sub_seed 0L "a"));
  check_s "sub_seed 12345" "9898446428729937406"
    (u64s (Prng.sub_seed 12345L "prop/ct_compare_eq_reflexive"));
  check_s "sub_seed -1 empty" "3526150024844925518"
    (u64s (Prng.sub_seed (-1L) ""));
  let r = Prng.create 5L in
  check_s "state" "5" (u64s (Prng.state r));
  ignore (Prng.next_u64 r);
  check_s "state advances by gamma"
    (u64s (Int64.add 5L Prng.golden_gamma))
    (u64s (Prng.state r))

let test_bounded_vectors () =
  List.iter
    (fun (seed, n, expect, next) ->
      let r = Prng.create (i64 seed) in
      let v = Prng.bounded r (bi n) in
      check_s (Printf.sprintf "bounded seed=%s n=%s" seed n) expect (bs v);
      check_s
        (Printf.sprintf "next word seed=%s n=%s" seed n)
        next
        (u64s (Prng.next_u64 r)))
    Vectors.bounded_vectors

let test_bounded_big_vectors () =
  List.iter
    (fun (seed, span, expect, next) ->
      let r = Prng.create (i64 seed) in
      let v = Prng.bounded_big r (bi span) in
      check_s
        (Printf.sprintf "bounded_big seed=%s span=%s" seed span)
        expect (bs v);
      check_s "next word" next (u64s (Prng.next_u64 r)))
    Vectors.big_vectors

(* The pins of VER05q 7.1. *)
let test_pins () =
  (* the rejection draw: the first word is 0 and MUST be rejected *)
  let seed = 7046029254386353131L in
  let r = Prng.create seed in
  check_s "first word is 0" "0" (u64s (Prng.next_u64 r));
  check_s "second word" "16294208416658607535" (u64s (Prng.next_u64 r));
  check_s "bounded(3) rejects 0" "1"
    (bs (Prng.bounded (Prng.create seed) (B.of_int 3)));
  check_i "bounded_int 3 agrees" 1 (Prng.bounded_int (Prng.create seed) 3);
  (* the 2^63+1 boundary *)
  let n = B.add (B.pow2 63) B.one in
  check_s "threshold 2^63-1" "9223372036854775807"
    (u64s (Prng.default_threshold (B.to_int64_wrap n)));
  check_s "seed 3: second word" "3694763184872335752"
    (bs (Prng.bounded (Prng.create 3L) n));
  check_s "seed 1: first word accepted" "1227844342346046656"
    (bs (Prng.bounded (Prng.create 1L) n));
  (* bounded(1) consumes a word *)
  let r = Prng.create 7L in
  check_s "bounded 1 is 0" "0" (bs (Prng.bounded r B.one));
  check_s "next after bounded 1" "309689372594955804" (u64s (Prng.next_u64 r));
  (* n = 2^64 is one raw word; n = 2^64-1 uses the threshold *)
  check_s "n=2^64 raw" "16294208416658607535"
    (bs (Prng.bounded (Prng.create 0L) (B.pow2 64)));
  check_s "threshold of 1" "0" (u64s (Prng.default_threshold 1L));
  check_s "threshold of 3" "1" (u64s (Prng.default_threshold 3L))

let test_bounded_errors () =
  let r = Prng.create 0L in
  let raises f =
    try
      ignore (f ());
      false
    with Error.Argument_error _ -> true
  in
  check_b "n = 0" true (raises (fun () -> Prng.bounded r B.zero));
  check_b "n < 0" true (raises (fun () -> Prng.bounded r (B.of_int (-1))));
  check_b "n > 2^64" true
    (raises (fun () -> Prng.bounded r (B.add (B.pow2 64) B.one)));
  check_b "span 0" true (raises (fun () -> Prng.bounded_big r B.zero));
  check_b "bounded_int 0" true (raises (fun () -> Prng.bounded_int r 0));
  check_b "bounded_int negative" true
    (raises (fun () -> Prng.bounded_int r (-4)));
  check_b "bounded_int max_int fits" true (Prng.bounded_int r max_int >= 0)

(* Unbiasedness of the k-word path: the three thirds of a 3 * 2^126 span (the Dart bug). *)
let test_bounded_big_thirds () =
  let span = B.shift_left (B.of_int 3) 126 in
  let third = B.shift_left B.one 126 in
  let r = Prng.create 77L in
  let counts = Array.make 3 0 in
  for _ = 1 to 3000 do
    let v = Prng.bounded_big r span in
    let k =
      if B.compare v third < 0 then 0
      else if B.compare v (B.shift_left third 1) < 0 then 1
      else 2
    in
    counts.(k) <- counts.(k) + 1
  done;
  Array.iter
    (fun c -> check_b "each third near 1000" true (c > 850 && c < 1150))
    counts

(* ===================================================================== *)
(* Shrinking: pinned step counts and evaluation order                      *)
(* ===================================================================== *)

let si ?max_steps ~initial ~target ~lo ~hi prop =
  let m, s =
    Shrink.shrink_int ?max_steps ~initial:(B.of_int initial)
      ~target:(B.of_int target) ~lo:(B.of_int lo) ~hi:(B.of_int hi) prop
  in
  (bs m, s)

let le n x = B.compare x (B.of_int n) <= 0
let pair = Alcotest.(pair string int)

let test_shrink_int_pins () =
  Alcotest.check pair "54321 <= 100" ("101", 15)
    (si ~initial:54321 ~target:0 ~lo:0 ~hi:100000 (le 100));
  Alcotest.check pair "everything failing" ("0", 1)
    (si ~initial:77 ~target:0 ~lo:0 ~hi:100 (fun _ -> false));
  Alcotest.check pair "max_steps 0" ("54321", 0)
    (si ~max_steps:0 ~initial:54321 ~target:0 ~lo:0 ~hi:100000 (le 100));
  Alcotest.check pair "max_steps 3" ("6791", 3)
    (si ~max_steps:3 ~initial:54321 ~target:0 ~lo:0 ~hi:100000 (le 100));
  Alcotest.check pair "100 in [-100,500]" ("51", 49)
    (si ~initial:100 ~target:0 ~lo:(-100) ~hi:500 (fun x ->
         B.compare x (B.of_int (-10)) >= 0 && le 50 x));
  Alcotest.check pair "99 target 10 (< 5)" ("10", 1)
    (si ~initial:99 ~target:10 ~lo:10 ~hi:500 (fun x ->
         B.compare x (B.of_int 5) < 0))

(* A negative bisection floors: from -81 toward 0 the first candidate is -40. *)
let test_shrink_floor () =
  let seen = ref [] in
  let prop x =
    seen := bs x :: !seen;
    true
  in
  (try
     ignore
       (si ~initial:(-81) ~target:0 ~lo:(-100) ~hi:0 (fun x ->
            if bs x = "-81" then false else prop x))
   with _ -> ());
  Alcotest.(check (list string))
    "target probe then floored bisection" [ "0"; "-40" ]
    (List.rev !seen |> List.filteri (fun i _ -> i < 2))

let test_shrink_monotone_thresholds () =
  for t = 1 to 120 do
    List.iter
      (fun start ->
        if start >= t then
          let m, _ =
            si ~initial:start ~target:0 ~lo:0 ~hi:1000 (fun x ->
                B.compare x (B.of_int t) < 0)
          in
          check_s
            (Printf.sprintf "threshold %d from %d" t start)
            (string_of_int t) m)
      [ t; t + 1; 3 * t; 999 ];
    let m, _ =
      si ~initial:(-999) ~target:0 ~lo:(-1000) ~hi:0 (fun x ->
          B.compare x (B.of_int (-t)) > 0)
    in
    check_s "negative side" (string_of_int (-t)) m
  done

let sl ?max_steps initial prop =
  let m, s =
    Shrink.shrink_list ?max_steps ~initial:(List.map B.of_int initial)
      ~elem_target:B.zero (fun l ->
        prop (List.map (fun x -> Option.get (B.to_int_opt x)) l))
  in
  (List.map (fun x -> Option.get (B.to_int_opt x)) m, s)

let test_shrink_list_pins () =
  let log = ref [] in
  let prop l =
    log := l :: !log;
    not (List.mem 5 l)
  in
  let m, s = sl [ 0; 0; 0; 0; 5; 0; 0; 0 ] prop in
  Alcotest.(check (list int)) "minimal" [ 5 ] m;
  check_i "steps" 3 s;
  let order = List.rev !log in
  let prefix = List.filteri (fun i _ -> i < 8) order in
  Alcotest.(check (list (list int)))
    "evaluation order"
    [
      [ 0; 0; 0; 0; 5; 0; 0; 0 ];
      [];
      [ 0; 0; 0; 0 ];
      [ 5; 0; 0; 0 ];
      [ 5; 0 ];
      [ 5 ];
      [ 3 ];
      [ 4 ];
    ]
    prefix;
  let m1, s1 = sl [ 5; 0; 0; 0; 0; 0; 0; 0 ] (fun l -> not (List.mem 5 l)) in
  Alcotest.(check (pair (list int) int)) "5 first" ([ 5 ], 3) (m1, s1);
  let m2, s2 = sl [ 0; 0; 0; 0; 0; 0; 0; 5 ] (fun l -> not (List.mem 5 l)) in
  Alcotest.(check (pair (list int) int)) "5 last" ([ 5 ], 3) (m2, s2);
  let m3, s3 = sl ~max_steps:0 [ 1; 2; 3 ] (fun l -> List.length l < 1) in
  Alcotest.(check (pair (list int) int)) "max_steps 0" ([ 1; 2; 3 ], 0) (m3, s3);
  let m4, _ =
    sl [ 1; 2; 3; 3; 4; 5 ] (fun l ->
        let rec ok = function
          | a :: (b :: _ as r) -> a <> b && ok r
          | _ -> true
        in
        ok l)
  in
  Alcotest.(check (list int))
    "adjacent duplicates keep local minimum" [ 3; 3 ] m4

(* 1-minimal by deletion, checked over every list of length 0..4 over {0,1,2,3}. *)
let test_shrink_list_one_minimal () =
  let rec lists n =
    if n = 0 then [ [] ]
    else
      List.concat_map
        (fun l -> List.map (fun x -> x :: l) [ 0; 1; 2; 3 ])
        (lists (n - 1))
  in
  let props =
    [
      ("sum<=3", fun l -> List.fold_left ( + ) 0 l <= 3);
      ("no 3", fun l -> not (List.mem 3 l));
      ("len<2", fun l -> List.length l < 2);
    ]
  in
  List.iter
    (fun (name, prop) ->
      for n = 0 to 4 do
        List.iter
          (fun l ->
            if not (prop l) then (
              let m, _ = sl l prop in
              check_b (name ^ " falsifies") false (prop m);
              check_b (name ^ " no longer") true (List.length m <= List.length l);
              List.iteri
                (fun i _ ->
                  let d = List.filteri (fun j _ -> j <> i) m in
                  check_b (name ^ " 1-minimal") true (prop d))
                m))
          (lists n)
      done)
    props

let test_shrink_bytes_pins () =
  let m, s =
    Shrink.shrink_bytes ~initial:"\x01\x02\x03\x04\x05\x06\x07\x08" (fun b ->
        String.length b < 5)
  in
  check_s "minimal" "\000\000\000\000\000" m;
  check_i "steps" 16 s;
  let m, s = Shrink.shrink_bytes ~max_steps:0 ~initial:"abc" (fun _ -> false) in
  check_s "max_steps 0" "abc" m;
  check_i "no steps" 0 s

let test_simulate_replay () =
  let a, b = Shrink.simulate_replay 42L (le 100) in
  check_s "initial" "413" (bs a);
  check_s "minimal" "101" (bs b);
  let r =
    try
      ignore (Shrink.simulate_replay 42L (fun _ -> true));
      false
    with Error.Argument_error _ -> true
  in
  check_b "never falsified raises" true r

let test_shrink_errors () =
  let raises f =
    try
      ignore (f ());
      `No
    with
    | Error.Argument_error _ -> `Arg
    | Error.Not_falsifying -> `Nf
  in
  let z = B.zero and h = B.of_int 100 in
  Alcotest.(check bool)
    "not falsifying" true
    (raises (fun () ->
         Shrink.shrink_int ~initial:z ~target:z ~lo:z ~hi:h (fun _ -> true))
    = `Nf);
  Alcotest.(check bool)
    "lo > hi" true
    (raises (fun () ->
         Shrink.shrink_int ~initial:z ~target:z ~lo:h ~hi:z (fun _ -> false))
    = `Arg);
  Alcotest.(check bool)
    "initial out of range" true
    (raises (fun () ->
         Shrink.shrink_int ~initial:(B.of_int 500) ~target:z ~lo:z ~hi:h
           (fun _ -> false))
    = `Arg);
  Alcotest.(check bool)
    "steps out of range" true
    (raises (fun () ->
         Shrink.shrink_int ~max_steps:10_001 ~initial:z ~target:z ~lo:z ~hi:h
           (fun _ -> false))
    = `Arg);
  Alcotest.(check bool)
    "negative steps" true
    (raises (fun () ->
         Shrink.shrink_list ~max_steps:(-1) ~initial:[] ~elem_target:z (fun _ ->
             false))
    = `Arg);
  Alcotest.(check bool)
    "list not falsifying" true
    (raises (fun () ->
         Shrink.shrink_list ~initial:[ z ] ~elem_target:z (fun _ -> true))
    = `Nf);
  Alcotest.(check bool)
    "bytes not falsifying" true
    (raises (fun () -> Shrink.shrink_bytes ~initial:"a" (fun _ -> true)) = `Nf)

(* ===================================================================== *)
(* Generators                                                              *)
(* ===================================================================== *)

let draws g seed n =
  let r = Prng.create seed in
  List.init n (fun _ -> Gen.sample g r)

let test_gen_basics () =
  let ints g = List.map bs (draws g 1L 200) in
  List.iter
    (fun s ->
      let v = int_of_string s in
      check_b "u8 in range" true (v >= 0 && v < 256))
    (ints Gen.u8);
  check_b "bool both values" true
    (let d = draws Gen.bool 3L 50 in
     List.mem true d && List.mem false d);
  List.iter
    (fun s ->
      let v = int_of_string s in
      check_b "integer in range" true (v >= -5 && v <= 5))
    (ints (Gen.integer (B.of_int (-5)) (B.of_int 5)));
  List.iter
    (fun v -> check_b "int in range" true (v >= 10 && v <= 12))
    (draws (Gen.int 10 12) 2L 100);
  check_b "int covers the range" true
    (let d = draws (Gen.int 10 12) 2L 100 in
     List.mem 10 d && List.mem 12 d);
  List.iter
    (fun v -> check_b "u64 non-negative" true (B.sign v >= 0))
    (draws Gen.u64 4L 20);
  let wide = draws Gen.i128 5L 40 in
  check_b "i128 has negatives" true (List.exists (fun v -> B.sign v < 0) wide);
  check_b "i128 within range" true
    (List.for_all
       (fun v -> B.bit_length v <= 127 || B.equal v (B.neg (B.pow2 127)))
       wide);
  List.iter
    (fun s ->
      check_b "bytes length inclusive" true
        (String.length s >= 2 && String.length s <= 4))
    (draws (Gen.bytes 2 4) 6L 100);
  check_b "bytes hits both ends" true
    (let l = List.map String.length (draws (Gen.bytes 2 4) 6L 100) in
     List.mem 2 l && List.mem 4 l);
  let ls = draws (Gen.list ~min_len:1 ~max_len:3 Gen.bool) 7L 100 in
  check_b "list length inclusive" true
    (List.for_all
       (fun l ->
         let n = List.length l in
         n >= 1 && n <= 3)
       ls);
  check_b "constant draws nothing" true
    (let r = Prng.create 9L in
     ignore
       (Gen.sample (Gen.constant ~canon:(fun x -> Val.Int (B.of_int x)) 5) r);
     Prng.state r = 9L)

let test_gen_combinators () =
  let canon x = Val.Int (B.of_int x) in
  let g = Gen.map ~canon (fun x -> x * 2) (Gen.int 0 5) in
  List.iter (fun v -> check_b "map doubles" true (v mod 2 = 0)) (draws g 1L 50);
  check_b "map drops shrinking" true (g.Gen.shrink = None);
  let one =
    Gen.one_of ~canon [ Gen.constant ~canon 1; Gen.constant ~canon 2 ]
  in
  let d = draws one 1L 200 in
  check_b "one_of uses both arms" true (List.mem 1 d && List.mem 2 d);
  let freq =
    Gen.frequency ~canon
      [
        (0, Gen.constant ~canon 0);
        (9, Gen.constant ~canon 1);
        (1, Gen.constant ~canon 2);
      ]
  in
  let d = draws freq 1L 2000 in
  check_b "zero-weight arm never selected" true (not (List.mem 0 d));
  let ones = List.length (List.filter (( = ) 1) d) in
  check_b "weights followed" true (ones > 1700 && ones < 1900);
  let fm = Gen.flat_map ~canon (fun n -> Gen.int n n) (Gen.int 3 4) in
  check_b "flat_map" true
    (List.for_all (fun v -> v = 3 || v = 4) (draws fm 1L 20));
  let calls = ref 0 in
  let f =
    Gen.filter ~max_retries:5
      (fun _ ->
        incr calls;
        false)
      (Gen.int 0 3)
  in
  let r = Prng.create 1L in
  (try
     ignore (Gen.sample f r);
     Alcotest.fail "filter should be exhausted"
   with Error.Exhausted _ -> ());
  check_i "filter samples exactly max_retries" 5 !calls;
  let ev = Gen.filter (fun x -> x mod 2 = 0) (Gen.int 0 100) in
  check_b "filter keeps only matching" true
    (List.for_all (fun v -> v mod 2 = 0) (draws ev 1L 100));
  let t2 = Gen.tuple2 (Gen.int 0 9) Gen.bool in
  check_b "tuple2 samples left to right" true
    (let r1 = Prng.create 3L in
     let a = Gen.sample t2 r1 in
     let r2 = Prng.create 3L in
     let x = Gen.sample (Gen.int 0 9) r2 in
     let y = Gen.sample Gen.bool r2 in
     a = (x, y));
  let t3 = Gen.tuple3 (Gen.int 0 9) Gen.bool Gen.u8 in
  ignore (draws t3 1L 5)

let test_gen_invalid () =
  let canon x = Val.Int (B.of_int x) in
  let bad g =
    try
      ignore (Gen.sample g (Prng.create 0L));
      false
    with Error.Argument_error _ -> true
  in
  check_b "integer min>max" true (bad (Gen.integer B.one B.zero));
  check_b "int min>max" true (bad (Gen.int 5 1));
  check_b "span too wide" true (bad (Gen.integer B.zero (B.pow2 16384)));
  check_b "span at cap ok" false
    (bad (Gen.integer B.zero (B.sub (B.pow2 16384) (B.of_int 2))));
  check_b "bytes min>max" true (bad (Gen.bytes 3 1));
  check_b "bytes too long" true (bad (Gen.bytes 0 (Types.max_input_bytes + 1)));
  check_b "list negative" true (bad (Gen.list ~min_len:(-1) Gen.bool));
  check_b "list too long" true
    (bad (Gen.list ~max_len:(Types.max_list_len + 1) Gen.bool));
  check_b "one_of empty" true (bad (Gen.one_of ~canon []));
  check_b "frequency empty" true (bad (Gen.frequency ~canon []));
  check_b "frequency negative" true
    (bad (Gen.frequency ~canon [ (-1, Gen.constant ~canon 0) ]));
  check_b "frequency zero total" true
    (bad (Gen.frequency ~canon [ (0, Gen.constant ~canon 0) ]));
  check_b "frequency too big" true
    (bad
       (Gen.frequency ~canon
          [ (Types.max_frequency_total + 1, Gen.constant ~canon 0) ]));
  check_b "filter retries 0" true
    (bad (Gen.filter ~max_retries:0 (fun _ -> true) Gen.bool));
  check_b "filter retries too many" true
    (bad (Gen.filter ~max_retries:10_001 (fun _ -> true) Gen.bool));
  (* an invalid generator can never pass *)
  let v = Runner.check ~seed:1L (Gen.int 5 1) (opts ()) (fun _ -> true) in
  check_s "invalid generator is exhausted" "property-exhausted" (fid v)

let test_gen_shrinkers () =
  let falsifies_ge n x = x >= n in
  let shr g v pred = fst (Gen.shrink g v pred 1000) in
  check_i "int shrinks toward zero in range" 17
    (shr (Gen.int 0 100) 99 (falsifies_ge 17));
  check_i "int respects lo" 10 (shr (Gen.int 10 100) 99 (falsifies_ge 5));
  check_i "int negative-only range targets hi" (-3)
    (shr (Gen.int (-10) (-3)) (-9) (fun _ -> true));
  check_b "bool true -> false" false (shr Gen.bool true (fun _ -> true));
  check_b "bool stays when false fails to falsify" true
    (shr Gen.bool true (fun b -> b));
  check_s "u8 shrinks" "20"
    (bs (shr Gen.u8 (B.of_int 200) (fun x -> B.compare x (B.of_int 20) >= 0)));
  check_s "u64 shrinks" "1000"
    (bs
       (shr Gen.u64 (bi "123456789012345678") (fun x ->
            B.compare x (B.of_int 1000) >= 0)));
  check_s "i128 shrinks" "-7"
    (bs
       (shr Gen.i128 (bi "-170141183460469231731687303715884105728") (fun x ->
            B.compare x (B.of_int (-7)) <= 0)));
  check_s "bytes keep min_len" "\000\000"
    (shr (Gen.bytes 2 8) "abcdefg" (fun _ -> true));
  Alcotest.(check (list int))
    "list shrinks elements" [ 5; 5 ]
    (shr
       (Gen.list (Gen.int 0 100))
       [ 40; 50; 60 ]
       (fun l -> List.length l >= 2 && List.for_all (fun x -> x >= 5) l));
  let t = Gen.tuple2 (Gen.int 0 100) (Gen.int 0 100) in
  Alcotest.(check (pair int int))
    "tuple2 coordinate-wise" (3, 4)
    (shr t (90, 80) (fun (a, b) -> a >= 3 && b >= 4));
  let t3 = Gen.tuple3 (Gen.int 0 100) (Gen.int 0 100) (Gen.int 0 100) in
  check_b "tuple3 coordinate-wise" true
    (shr t3 (90, 80, 70) (fun (a, b, c) -> a >= 3 && b >= 4 && c >= 5)
    = (3, 4, 5));
  let filt = Gen.filter (fun x -> x >= 3) (Gen.int 0 100) in
  check_i "filter shrinks within the predicate (0 is not a valid input)" 3
    (shr filt 99 (fun _ -> true));
  check_i "max_steps 0 returns input" 99
    (fst (Gen.shrink (Gen.int 0 100) 99 (fun _ -> true) 0));
  check_i "no shrinker returns input" 4
    (shr
       (Gen.map
          ~canon:(fun x -> Val.Int (B.of_int x))
          (fun x -> x)
          (Gen.int 0 100))
       4
       (fun _ -> true));
  let raises =
    try
      ignore (Gen.shrink (Gen.int 0 1) 0 (fun _ -> true) 10_001);
      false
    with Error.Argument_error _ -> true
  in
  check_b "shrink validates steps" true raises

(* ===================================================================== *)
(* Runner                                                                  *)
(* ===================================================================== *)

let int_gen = Gen.int 0 100_000

let test_runner_verdicts () =
  let v = Runner.check ~seed:42L int_gen (opts ()) (fun x -> x >= 0) in
  check_b "passes" true v.passed;
  check_i "cases_run" 100 v.cases_run;
  let v = Runner.check ~seed:42L int_gen (opts ()) (fun x -> x <= 100) in
  check_s "counterexample" "property-counterexample" (fid v);
  Alcotest.(check (option string)) "minimal" (Some "101") v.minimal;
  check_b "seed disclosed locally" true (v.seed = Some 42L);
  check_b "initial disclosed" true (v.counterexample <> None);
  check_b "message names the minimum" true
    (contains (Option.get v.message) "minimal=101");
  check_b "steps recorded" true (v.shrink_steps > 0);
  let v0 = Runner.check ~seed:1L int_gen (opts ~n:0 ()) (fun _ -> true) in
  check_s "num_tests 0 is exhausted, never a pass" "property-exhausted" (fid v0);
  check_b "not passed" false v0.passed;
  let vp = Runner.check ~seed:1L int_gen (opts ()) (fun _ -> failwith "boom") in
  check_s "panic" "property-panic" (fid vp);
  check_b "panic seed disclosed locally" true (vp.seed = Some 1L);
  check_b "panic message names the constructor" true
    (contains (Option.get vp.message) "Failure");
  check_b "payload never shown" false (contains (Option.get vp.message) "boom");
  let vx =
    Runner.check ~seed:1L
      (Gen.filter ~max_retries:3 (fun _ -> false) int_gen)
      (opts ())
      (fun _ -> true)
  in
  check_s "filter exhaustion" "property-exhausted" (fid vx);
  let vg =
    Runner.check ~seed:1L
      (Gen.custom
         ~canon:(fun x -> Val.Int (B.of_int x))
         (fun _ _ -> failwith "sampler"))
      (opts ())
      (fun _ -> true)
  in
  check_s "sampler failure is exhausted" "property-exhausted" (fid vg);
  check_b "sampler failure names the constructor" true
    (contains (Option.get vg.message) "Failure");
  let bad_opts f =
    try
      ignore (f ());
      false
    with Error.Argument_error _ -> true
  in
  check_b "negative num_tests" true
    (bad_opts (fun () ->
         Runner.check ~seed:1L int_gen (opts ~n:(-1) ()) (fun _ -> true)));
  check_b "too many tests" true
    (bad_opts (fun () ->
         Runner.check ~seed:1L int_gen
           (opts ~n:(Types.max_cases + 1) ())
           (fun _ -> true)));
  check_b "steps out of range" true
    (bad_opts (fun () ->
         Runner.check ~seed:1L int_gen (opts ~steps:20_000 ()) (fun _ -> true)))

let test_runner_redaction () =
  List.iter
    (fun ci ->
      let v =
        Runner.check ~seed:42L int_gen (opts ~ci ()) (fun x -> x <= 100)
      in
      check_s "counterexample" "property-counterexample" (fid v);
      check_b "no seed" true (v.seed = None);
      check_b "no counterexample" true (v.counterexample = None);
      check_b "no minimal" true (v.minimal = None);
      check_b "no minimal in message" false
        (contains (Option.get v.message) "minimal=");
      let d = Option.get v.digest in
      check_i "digest is 16 hex digits" 16 (String.length d);
      check_b "digest hex" true
        (String.for_all
           (fun c -> (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'))
           d);
      let vp =
        Runner.check ~seed:1L int_gen (opts ~ci ()) (fun _ -> failwith "secret")
      in
      check_b "panic redacted seed" true (vp.seed = None);
      check_b "panic message has no payload" false
        (contains (Option.get vp.message) "secret"))
    [ Types.Ci_on; Types.Ci_unset ];
  check_b "unset is the default" true (Types.default_options.ci = Types.Ci_unset);
  check_b "env None" true (Types.ci_of_env None = Types.Ci_off);
  check_b "env empty" true (Types.ci_of_env (Some "") = Types.Ci_off);
  check_b "env 0" true (Types.ci_of_env (Some " 0 ") = Types.Ci_off);
  check_b "env False" true (Types.ci_of_env (Some "False") = Types.Ci_off);
  check_b "env true" true (Types.ci_of_env (Some "true") = Types.Ci_on);
  check_b "env 1" true (Types.ci_of_env (Some "1") = Types.Ci_on);
  let a =
    Runner.check ~seed:42L int_gen (opts ~ci:Types.Ci_on ()) (fun x -> x <= 100)
  in
  let b =
    Runner.check ~seed:42L int_gen (opts ~ci:Types.Ci_on ()) (fun x -> x <= 100)
  in
  check_b "digest is deterministic" true (a.digest = b.digest)

let test_runner_reverification () =
  (* a defective shrinker returns a value that no longer falsifies *)
  let lying : int Gen.shrinker = fun _ _ _ _ -> (0, 5) in
  let v =
    Runner.check ~shrink_override:lying ~seed:42L int_gen (opts ()) (fun x ->
        x <= 100)
  in
  check_s "still a counterexample" "property-counterexample" (fid v);
  check_i "original witness, 0 steps" 0 v.shrink_steps;
  check_b "witness really fails" true
    (int_of_string (Option.get v.minimal) > 100);
  let raising : int Gen.shrinker = fun _ _ _ _ -> failwith "shrinker" in
  let v =
    Runner.check ~shrink_override:raising ~seed:42L int_gen (opts ()) (fun x ->
        x <= 100)
  in
  check_i "raising shrinker keeps the witness" 0 v.shrink_steps;
  let silly : int Gen.shrinker = fun _ v _ _ -> (v, 99_999) in
  let v =
    Runner.check ~shrink_override:silly ~seed:42L int_gen (opts ()) (fun x ->
        x <= 100)
  in
  check_i "out-of-range step count is distrusted" 0 v.shrink_steps;
  (* a property that raises on smaller inputs: the raising candidate is "not falsifying" *)
  let v =
    Runner.check ~seed:42L int_gen (opts ()) (fun x ->
        if x < 5000 then failwith "small" else x <= 6000)
  in
  check_s "still a counterexample" "property-counterexample" (fid v);
  check_b "witness above the raising region" true
    (int_of_string (Option.get v.minimal) >= 5000)

let test_runner_determinism () =
  let a = Runner.check ~seed:9L int_gen (opts ()) (fun x -> x <= 100) in
  let b = Runner.check ~seed:9L int_gen (opts ()) (fun x -> x <= 100) in
  check_b "same seed same verdict" true (a = b)

(* ===================================================================== *)
(* Hazard: exceptions and effects                                          *)
(* ===================================================================== *)

exception Secret of string

type _ Effect.t += Ask : int Effect.t

let test_exceptions () =
  let printer_ran = ref false in
  Printexc.register_printer (function
    | Secret _ ->
        printer_ran := true;
        Some "PRINTED"
    | _ -> None);
  let panic_of exn =
    let v = Runner.check ~seed:1L int_gen (opts ~n:3 ()) (fun _ -> raise exn) in
    check_s "panic" "property-panic" (fid v);
    Option.get v.message
  in
  List.iter
    (fun (name, exn) ->
      check_b
        (name ^ " named by constructor")
        true
        (contains (panic_of exn) name))
    [
      ("Failure", Failure "x");
      ("Not_found", Not_found);
      ("Invalid_argument", Invalid_argument "x");
      ("Division_by_zero", Division_by_zero);
      ("Assert_failure", Assert_failure ("f", 1, 2));
      ("Stack_overflow", Stack_overflow);
      ("Exit", Exit);
    ];
  let m = panic_of (Secret "hunter2") in
  check_b "user exception named" true (contains m "Secret");
  check_b "payload not shown" false (contains m "hunter2");
  check_b "registered printer did not run" false !printer_ran;
  check_b "Printexc.to_string would have run it" true
    (Printexc.to_string (Secret "x") = "PRINTED" && !printer_ran);
  (* classification does not depend on message text *)
  let v =
    Runner.check ~seed:1L int_gen (opts ~n:3 ()) (fun _ -> failwith "Sys.Break")
  in
  check_s "Failure \"Sys.Break\" is a panic" "property-panic" (fid v);
  (* an exception whose name needs escaping stays one line *)
  check_b "name has no raw control characters" true
    (String.for_all
       (fun c -> Char.code c >= 0x20)
       (Safe.exn_name (Failure "x")))

let test_break_and_oom_propagate () =
  Alcotest.check_raises "Sys.Break propagates" Sys.Break (fun () ->
      ignore
        (Runner.check ~seed:1L int_gen (opts ()) (fun _ -> raise Sys.Break)));
  check_b "scope closed after Sys.Break" true (Budget.current () = None);
  Alcotest.check_raises "Out_of_memory propagates" Out_of_memory (fun () ->
      ignore
        (Runner.check ~seed:1L int_gen (opts ()) (fun _ -> raise Out_of_memory)));
  check_b "scope closed after Out_of_memory" true (Budget.current () = None);
  (* also while shrinking *)
  Alcotest.check_raises "Sys.Break during shrinking propagates" Sys.Break
    (fun () ->
      ignore
        (Runner.check ~seed:42L int_gen (opts ()) (fun x ->
             if x < 50_000 then raise Sys.Break else x <= 100)));
  check_b "scope closed after shrinking Break" true (Budget.current () = None);
  (* from a sampler too *)
  Alcotest.check_raises "Sys.Break from a sampler" Sys.Break (fun () ->
      ignore
        (Runner.check ~seed:1L
           (Gen.custom ~canon:(fun _ -> Val.Nil) (fun _ _ -> raise Sys.Break))
           (opts ())
           (fun _ -> true)));
  Alcotest.check_raises "Sys.Break from canon" Sys.Break (fun () ->
      ignore (Canon.render_with (fun _ -> raise Sys.Break) 1));
  Alcotest.check_raises "Out_of_memory from canon" Out_of_memory (fun () ->
      ignore (Canon.render_with (fun _ -> raise Out_of_memory) 1));
  Alcotest.check_raises "Sys.Break from Safe" Sys.Break (fun () ->
      ignore (Safe.recoverable (fun () -> raise Sys.Break)));
  check_b "other exceptions are values" true
    (Result.is_error (Safe.recoverable (fun () -> raise Exit)))

let test_effects () =
  let v =
    Runner.check ~seed:1L int_gen (opts ~n:3 ()) (fun _ ->
        Effect.perform Ask > 0)
  in
  check_s "unhandled effect is a panic" "property-panic" (fid v);
  check_b "named" true (contains (Option.get v.message) "Unhandled");
  let handled =
    Runner.check ~seed:1L int_gen (opts ~n:10 ()) (fun x ->
        Effect.Deep.try_with
          (fun () -> Effect.perform Ask + x >= 0)
          ()
          {
            Effect.Deep.effc =
              (fun (type a) (e : a Effect.t) ->
                match e with
                | Ask ->
                    Some
                      (fun (k : (a, _) Effect.Deep.continuation) ->
                        Effect.Deep.continue k 1)
                | _ -> None);
          })
  in
  check_b "a handled effect works" true handled.passed;
  check_b "scope closed" true (Budget.current () = None)

(* ===================================================================== *)
(* Hazard: user values                                                     *)
(* ===================================================================== *)

type fn_box = { f : int -> int; n : int }

let test_user_values () =
  (* a record with a closure: polymorphic compare would raise; the engine never applies it *)
  let g =
    Gen.map
      ~canon:(fun b -> Val.Int (B.of_int b.n))
      (fun n -> { f = (fun x -> x + n); n })
      (Gen.int 0 1000)
  in
  let v = Runner.check ~seed:3L g (opts ()) (fun b -> b.f 0 < 10) in
  check_s "closure-carrying values are generated, falsified and rendered"
    "property-counterexample" (fid v);
  check_b "rendered through canon" true
    (int_of_string (Option.get v.minimal) >= 10);
  check_i "map drops shrinking: no steps" 0 v.shrink_steps;
  (* a cyclic Val *)
  let rec cyc = Val.List [ cyc ] in
  let s = Canon.render cyc in
  check_b "cyclic render ends" true (String.length s < 200 && contains s "...");
  check_b "cyclic equality fails closed" false (Canon.same_val cyc cyc);
  let rec cyc2 = Val.Tuple [ Val.Nil; cyc2 ] in
  check_b "cyclic tuple equality fails closed" false (Canon.same_val cyc2 cyc2);
  let cg = Gen.custom ~canon:(fun () -> cyc) (fun _ _ -> ()) in
  let v =
    Runner.check ~seed:1L cg (opts ~ci:Types.Ci_off ()) (fun () -> false)
  in
  check_s "cyclic witness is rendered safely" "property-counterexample" (fid v);
  check_s "raising canon is unrenderable" "<unrenderable>"
    (Canon.render_with (fun _ -> failwith "x") 1);
  let v =
    Runner.check ~seed:1L
      (Gen.custom
         ~canon:(fun _ -> failwith "canon")
         (fun _ r -> Int64.to_int (Prng.next_u64 r)))
      (opts ())
      (fun _ -> false)
  in
  check_s "raising canon never crashes the runner" "property-counterexample"
    (fid v);
  Alcotest.(check (option string))
    "minimal unrenderable" (Some "<unrenderable>") v.minimal;
  (* a million nodes and a 10 MiB text render within the caps *)
  let big = Val.List (List.init 1_000_000 (fun _ -> Val.Nil)) in
  let s = Canon.render big in
  check_b "million nodes truncated" true
    (String.length s <= Canon.max_output + 3 && contains s "...");
  let text = Val.Text (String.make (10 * 1024 * 1024) 'a') in
  let s = Canon.render text in
  check_b "10 MiB text bounded" true (String.length s <= Canon.max_output + 10);
  let bytes = Val.Bytes (String.make (3 * 1024 * 1024) '\255') in
  check_b "big bytes bounded" true
    (String.length (Canon.render bytes) <= Canon.max_output + 10);
  let deep = ref Val.Nil in
  for _ = 1 to 100_000 do
    deep := Val.List [ !deep ]
  done;
  check_b "deep nesting is cut" true (contains (Canon.render !deep) "...");
  check_b "deep equality fails closed" false (Canon.same_val !deep !deep);
  check_s "huge int summarised" "<bigint 100001 bits>"
    (Canon.render (Val.Int (B.pow2 100_000)));
  check_s "int below the cap" "1267650600228229401496703205376"
    (Canon.render (Val.Int (B.pow2 100)))

let test_canon_render () =
  let r = Canon.render in
  check_s "nil" "nil" (r Val.Nil);
  check_s "true" "true" (r (Val.Bool true));
  check_s "false" "false" (r (Val.Bool false));
  check_s "int" "-12" (r (Val.Int (B.of_int (-12))));
  check_s "bytes" "0x00ff" (r (Val.Bytes "\000\255"));
  check_s "empty bytes" "0x" (r (Val.Bytes ""));
  check_s "text" "\"a\\\"b\\\\c\"" (r (Val.Text "a\"b\\c"));
  check_s "list" "[1,2]" (r (Val.List [ Val.Int B.one; Val.Int (B.of_int 2) ]));
  check_s "tuple" "(true,nil)" (r (Val.Tuple [ Val.Bool true; Val.Nil ]));
  check_s "nested" "[(1,[])]"
    (r (Val.List [ Val.Tuple [ Val.Int B.one; Val.List [] ] ]));
  (* escapes *)
  check_s "newline" "\"a\\nb\"" (r (Val.Text "a\nb"));
  check_s "cr tab" "\"\\r\\t\"" (r (Val.Text "\r\t"));
  check_s "nul" "\"\\u0000\"" (r (Val.Text "\000"));
  check_s "DEL" "\"\\u007f\"" (r (Val.Text "\127"));
  check_s "C1" "\"\\u0085\"" (r (Val.Text "\xC2\x85"));
  check_s "bidi override" "\"\\u202e\"" (r (Val.Text "\xE2\x80\xAE"));
  check_s "line separator" "\"\\u2028\"" (r (Val.Text "\xE2\x80\xA8"));
  check_s "zero width space" "\"\\u200b\"" (r (Val.Text "\xE2\x80\x8B"));
  check_s "BOM" "\"\\ufeff\"" (r (Val.Text "\xEF\xBB\xBF"));
  check_s "soft hyphen" "\"\\u00ad\"" (r (Val.Text "\xC2\xAD"));
  check_s "tag block" "\"\\u{e0001}\"" (r (Val.Text "\xF3\xA0\x80\x81"));
  check_s "ordinary unicode kept" "\"\xC3\xA9\"" (r (Val.Text "\xC3\xA9"));
  check_s "invalid utf-8" "\"\\xff\"" (r (Val.Text "\xFF"));
  check_s "unquoted does not escape quotes" "a\"b"
    (Escape.escape_text false "a\"b");
  List.iter
    (fun cp ->
      check_b (Printf.sprintf "U+%04X escaped" cp) true (Escape.needs_escape cp))
    [
      0;
      0x1F;
      0x7F;
      0x80;
      0x9F;
      0x2028;
      0x2029;
      0x61C;
      0x200E;
      0x200F;
      0x202A;
      0x202E;
      0x2066;
      0x2069;
      0xAD;
      0x34F;
      0x180E;
      0x200B;
      0x200D;
      0x2060;
      0x2064;
      0xFEFF;
      0xFFF9;
      0xFFFB;
      0xE0000;
      0xE007F;
    ];
  List.iter
    (fun cp ->
      check_b (Printf.sprintf "U+%04X kept" cp) false (Escape.needs_escape cp))
    [
      0x20;
      0x41;
      0x7E;
      0xA0;
      0xAE;
      0x2027;
      0x202F;
      0x2065;
      0x206A;
      0xFEFE;
      0xFFFC;
      0xDFFFF;
      0xE0080;
    ];
  (* clipping *)
  check_s "clip keeps short" "abc" (Escape.clip_text 10 "abc");
  check_s "clip cuts" "ab..." (Escape.clip_text 2 "abcdef");
  check_s "clip on a character boundary" "..."
    (Escape.clip_text 1 "\xC3\xA9\xC3\xA9");
  let s, cut = Escape.escape_bounded true 5 "abcdefghij" in
  check_s "bounded escape" "abcde" s;
  check_b "reports the cut" true cut;
  let s, cut = Escape.escape_bounded true 100 "abc" in
  check_s "unbounded escape" "abc" s;
  check_b "no cut" false cut;
  check_b "clipped text ends with an ellipsis" true
    (contains (r (Val.Text (String.make 2_000_000 'a'))) "...")

let test_same_val () =
  let i n = Val.Int (B.of_int n) in
  check_b "kind-sensitive: true is not 1" false
    (Canon.same_val (Val.Bool true) (i 1));
  check_b "bytes are not text" false
    (Canon.same_val (Val.Bytes "a") (Val.Text "a"));
  check_b "list is not a tuple" false
    (Canon.same_val (Val.List []) (Val.Tuple []));
  check_b "nil" true (Canon.same_val Val.Nil Val.Nil);
  check_b "bool" true (Canon.same_val (Val.Bool true) (Val.Bool true));
  check_b "int equal" true (Canon.same_val (i 5) (i 5));
  check_b "int differ" false (Canon.same_val (i 5) (i 6));
  check_b "text" true (Canon.same_val (Val.Text "a") (Val.Text "a"));
  check_b "bytes" true (Canon.same_val (Val.Bytes "a") (Val.Bytes "a"));
  check_b "lists" true
    (Canon.same_val (Val.List [ i 1; i 2 ]) (Val.List [ i 1; i 2 ]));
  check_b "length differs" false
    (Canon.same_val (Val.List [ i 1 ]) (Val.List [ i 1; i 2 ]));
  check_b "element differs" false
    (Canon.same_val (Val.List [ i 1; i 2 ]) (Val.List [ i 1; i 3 ]));
  check_b "tuples" true (Canon.same_val (Val.Tuple [ i 1 ]) (Val.Tuple [ i 1 ]));
  let wide n = Val.List (List.init n (fun _ -> Val.Nil)) in
  check_b "at the node cap" true (Canon.same_val (wide 9_998) (wide 9_998));
  check_b "past the node cap" false (Canon.same_val (wide 20_000) (wide 20_000))

(* ===================================================================== *)
(* Budget tree                                                             *)
(* ===================================================================== *)

let test_budget_charges () =
  Budget.with_root cfg ~max_attempts:3 (fun root ->
      Budget.with_child cfg (Some root) ~max_steps:0 (fun child ->
          check_b "1" true (Budget.charge child 5);
          check_b "2" true (Budget.charge child 5);
          check_b "3" true (Budget.charge child 5);
          check_b "4th refused (root cap)" false (Budget.charge child 5);
          check_i "root attempts unchanged by the refusal" 3
            (Budget.attempts root);
          check_i "child attempts unchanged by the refusal" 3
            (Budget.attempts child);
          check_i "root work" 15 (Budget.work root);
          check_i "child work" 15 (Budget.work child);
          check_b "refusal is recorded tree-wide" true
            (Budget.is_refused root && Budget.is_refused child);
          check_b "copy refused too (attempt cap is shared)" false
            (Budget.charge_copy child 1);
          check_b "room is false" false (Budget.room child);
          check_b "nested" true (Budget.is_nested child);
          check_b "root is not nested" false (Budget.is_nested root);
          check_b "root accessor" true (Budget.root child == root);
          check_i "depth" 1 (Budget.depth child);
          check_i "checks" 0 (Budget.checks child);
          check_i "child max attempts" 64 (Budget.max_attempts child);
          check_b "max work" true (Budget.max_work child > 0);
          check_b "config" true
            (Budget.config child == cfg
            || Budget.config child = Budget.config root)))

let test_budget_huge_cost () =
  Budget.with_root cfg ~max_attempts:10 (fun root ->
      check_b "max_int refused without overflow" false
        (Budget.charge root max_int);
      check_b "max_work + 1 refused" false
        (Budget.charge_copy root (Budget.max_work root + 1));
      check_i "nothing charged" 0 (Budget.attempts root);
      check_i "no work charged" 0 (Budget.work root);
      check_b "exactly the remaining work is afforded" true
        (Budget.charge_copy root (Budget.max_work root));
      check_b "then nothing" false (Budget.charge_copy root 1);
      check_i "copies are not attempts" 0 (Budget.attempts root))

let test_budget_scope_release () =
  let node = ref None in
  (try
     Budget.with_root cfg ~max_attempts:5 (fun r ->
         node := Some r;
         check_b "ambient inside" true
           (match Budget.current () with Some c -> c == r | None -> false);
         raise Exit)
   with Exit -> ());
  let r = Option.get !node in
  check_b "closed after an exception" true (Budget.is_closed r);
  check_b "ambient restored" true (Budget.current () = None);
  check_b "closed handle is spent" false (Budget.charge r 1);
  let child_node = ref None in
  let parent_node = ref None in
  Budget.with_root cfg ~max_attempts:5 (fun p ->
      parent_node := Some p;
      Budget.with_child cfg (Some p) ~max_steps:0 (fun c ->
          child_node := Some c));
  let c = Option.get !child_node and p = Option.get !parent_node in
  check_b "closed child" true (Budget.is_closed c);
  check_b "closed parent" true (Budget.is_spent c && Budget.is_spent p);
  check_b "a child of a closed parent charges nothing" true
    (Budget.with_child cfg (Some p) ~max_steps:10 (fun n ->
         not (Budget.charge n 1)));
  (* an open child of a still-open parent is not spent *)
  Budget.with_root cfg ~max_attempts:5 (fun p ->
      check_b "open is not spent" false (Budget.is_spent p))

let test_budget_depth_cap () =
  let rec nest n b =
    if n = 0 then Budget.depth b
    else Budget.with_child cfg (Some b) ~max_steps:0 (fun c -> nest (n - 1) c)
  in
  check_i "owning nesting stops at depth 63; deeper scopes join" 63
    (Budget.with_root cfg ~max_attempts:10 (fun r -> nest 100 r));
  check_i "shallow nesting nests" 5
    (Budget.with_root cfg ~max_attempts:10 (fun r -> nest 5 r))

let test_budget_ambient () =
  check_b "no ambient at top level" true (Budget.current () = None);
  Budget.with_root cfg ~max_attempts:5 (fun r ->
      check_b "ambient is the scope" true
        (match Budget.current () with Some c -> c == r | None -> false);
      Budget.with_child cfg None ~max_steps:0 (fun c ->
          check_b "explicit None is a fresh root" false (Budget.is_nested c)));
  check_b "resolve explicit" true
    (match Budget.resolve None with None -> true | Some _ -> false);
  Budget.with_root cfg ~max_attempts:5 (fun r ->
      check_b "resolve ambient" true
        (match Budget.resolve None with Some c -> c == r | None -> false));
  (* with_check refuses past the nesting cap *)
  let rec dive n b =
    match
      Budget.with_check cfg (Some b) ~num_tests:1 ~max_steps:0 (fun c ->
          if n = 0 then None else Some (dive (n - 1) c))
    with
    | None -> `Refused (Budget.checks b)
    | Some None -> `Bottom
    | Some (Some r) -> r
  in
  Budget.with_root cfg ~max_attempts:100 (fun r ->
      match dive 100 r with
      | `Refused k ->
          check_b "refused at 64 enclosing checks or depth 63" true (k >= 1)
      | `Bottom -> Alcotest.fail "should refuse")

let test_nested_checks () =
  (* nesting depth 1 to 3 and a branching case never pass when starved and never exceed the root cap *)
  let rec nest n =
    let v =
      Runner.check ~seed:(Int64.of_int n) (Gen.int 0 9) (opts ~n:3 ~steps:5 ())
        (fun _ -> if n = 0 then true else (nest (n - 1)).passed)
    in
    v
  in
  for depth = 0 to 3 do
    check_b
      (Printf.sprintf "nesting depth %d passes within budget" depth)
      true (nest depth).passed
  done;
  (* exact accounting: an inner check of 3 cases charges exactly 3 attempts to the outer scope *)
  let seen = ref [] in
  let v =
    Runner.check ~seed:1L (Gen.int 0 9) (opts ~n:6 ~steps:5 ()) (fun _ ->
        let outer = Option.get (Budget.current ()) in
        let before = Budget.attempts (Budget.root outer) in
        let inner =
          Runner.check ~seed:2L (Gen.int 0 9) (opts ~n:3 ~steps:5 ()) (fun _ ->
              true)
        in
        seen :=
          (Budget.attempts (Budget.root outer) - before, inner.passed) :: !seen;
        true)
  in
  check_b "outer passes" true v.passed;
  List.iter
    (fun (d, p) ->
      check_i "inner check charges entry + 2 further cases" 3 d;
      check_b "inner passes" true p)
    !seen;
  check_i "six outer cases" 6 (List.length !seen);
  (* branching: m checks per evaluation *)
  let v =
    Runner.check ~seed:1L (Gen.int 0 9) (opts ~n:4 ~steps:5 ()) (fun _ ->
        List.for_all
          (fun s ->
            (Runner.check ~seed:s (Gen.int 0 9) (opts ~n:2 ~steps:5 ())
               (fun _ -> true))
              .passed)
          [ 1L; 2L; 3L ])
  in
  check_b "branching passes within budget" true v.passed;
  (* starvation is never a pass: a tight root starves its nested checks *)
  let starved = ref 0 in
  let v =
    Budget.with_root cfg ~max_attempts:3 (fun root ->
        let r =
          Runner.check ~parent:root ~seed:1L (Gen.int 0 9)
            (opts ~n:50 ~steps:0 ()) (fun _ -> true)
        in
        if not r.passed then incr starved;
        r)
  in
  check_b "a starved nested check is not a pass" false v.passed;
  check_s "starved is exhausted" "property-exhausted" (fid v);
  check_i "starved once" 1 !starved;
  (* a closed handle is never a way to a fresh root *)
  let stale = ref None in
  Budget.with_root cfg ~max_attempts:50 (fun r -> stale := Some r);
  let v =
    Runner.check ~parent:(Option.get !stale) ~seed:1L (Gen.int 0 9)
      (opts ~n:3 ()) (fun _ -> true)
  in
  check_s "stale parent is exhausted" "property-exhausted" (fid v);
  check_b "nothing evaluated" true (v.cases_run = 0)

let test_check_nesting_cap () =
  let deepest = ref 100 in
  let rec nest n =
    Runner.check ~seed:1L (Gen.int 0 9) (opts ~n:1 ~steps:0 ()) (fun _ ->
        deepest := min !deepest n;
        if n = 0 then true else (nest (n - 1)).passed)
  in
  let v = nest 100 in
  check_b "a 100-deep chain of checks never passes" false v.passed;
  check_b "evaluation stopped at the nesting cap" true (100 - !deepest <= 64);
  check_b "but went reasonably deep" true (100 - !deepest >= 30)

let test_filter_budget () =
  (* top-level filter retries are WORK, not attempts: a legitimate rare filter passes *)
  let rare = Gen.filter ~max_retries:10_000 (fun x -> x < 5) (Gen.int 0 999) in
  let v =
    Runner.check ~seed:5L rare (opts ~n:200 ~steps:10 ()) (fun x -> x < 5)
  in
  check_b
    "rare filter at top level passes (200 cases x ~200 retries > attempt cap)"
    true v.passed;
  (* the same filter inside a nested check pays attempts and is starved: never a pass *)
  let outer =
    Runner.check ~seed:1L (Gen.int 0 9) (opts ~n:1 ~steps:0 ()) (fun _ ->
        (Runner.check ~seed:5L rare (opts ~n:200 ~steps:0 ()) (fun _ -> true))
          .passed)
  in
  check_b "nested rare filter is starved, never a pass" false outer.passed;
  (* a filter sampler rejecting everything *)
  let none = Gen.filter ~max_retries:50 (fun _ -> false) (Gen.int 0 9) in
  check_s "rejecting filter is exhausted" "property-exhausted"
    (fid (Runner.check ~seed:1L none (opts ()) (fun _ -> true)));
  (* stacked filters charge every retry *)
  let st =
    Gen.filter ~max_retries:100
      (fun x -> x = 0)
      (Gen.filter ~max_retries:100 (fun x -> x < 50) (Gen.int 0 99))
  in
  ignore (Runner.check ~seed:1L st (opts ~n:5 ()) (fun _ -> true));
  (* filter exhaustion in a spent scope *)
  let stale = ref None in
  Budget.with_root cfg ~max_attempts:5 (fun r -> stale := Some r);
  check_b "filter sample on a spent scope is exhausted" true
    (try
       ignore
         (Gen.sample ~ctx:(Option.get !stale)
            (Gen.filter (fun _ -> true) Gen.bool)
            (Prng.create 1L));
       false
     with Error.Exhausted _ -> true);
  (* a filter predicate that itself runs a nested check *)
  let weird =
    Gen.filter
      (fun x ->
        (Runner.check ~seed:1L (Gen.int 0 5) (opts ~n:2 ~steps:0 ()) (fun _ ->
             x >= 0))
          .passed)
      (Gen.int 0 9)
  in
  check_b "predicate running a nested check passes within budget" true
    (Runner.check ~seed:1L weird (opts ~n:5 ~steps:5 ()) (fun _ -> true)).passed

let test_recursing_shrinker () =
  let root_attempts = ref 0 and root_cap = ref 0 in
  let rec recurse depth : int Gen.shrinker =
   fun sc v f steps ->
    if depth = 0 then Gen.shrink_nested sc int_gen v f steps
    else (
      root_attempts := Budget.attempts (Budget.root sc);
      root_cap := Budget.max_attempts (Budget.root sc);
      let r, used = Gen.shrink ~parent:sc int_gen v f steps in
      let r2, used2 = (recurse (depth - 1)) sc r f (steps - used) in
      (r2, used + used2))
  in
  let v =
    Runner.check ~shrink_override:(recurse 3) ~seed:42L int_gen (opts ())
      (fun x -> x <= 100)
  in
  check_s "recursing shrinker is bounded and honest" "property-counterexample"
    (fid v);
  check_b "witness really fails" true
    (int_of_string (Option.get v.minimal) > 100);
  check_b "root cap respected" true (!root_attempts <= !root_cap)

let test_refused_reverification () =
  (* a shrinker that exhausts the check scope's budget: the re-verification is refused,
     the shrunk value is not adopted and the original witness is reported with 0 steps *)
  let hog : int Gen.shrinker =
   fun sc _v _f _steps ->
    while Budget.charge (Budget.root sc) 1 do
      ()
    done;
    (0, 1)
  in
  let v =
    Runner.check ~shrink_override:hog ~seed:42L int_gen (opts ~steps:1 ())
      (fun x -> x <= 100)
  in
  check_s "still a counterexample" "property-counterexample" (fid v);
  check_i "original witness, 0 steps" 0 v.shrink_steps;
  check_b "the witness is a real counterexample" true
    (int_of_string (Option.get v.minimal) > 100)

let test_concurrent_charging () =
  Budget.with_root cfg ~max_attempts:1000 (fun root ->
      let worker () =
        let ok = ref 0 in
        for _ = 1 to 2000 do
          if Budget.charge root 1 then incr ok
        done;
        !ok
      in
      let ds = List.init 4 (fun _ -> Domain.spawn worker) in
      let total = List.fold_left (fun a d -> a + Domain.join d) 0 ds in
      check_i "exactly the cap was granted" 1000 total;
      check_i "no lost or doubled charge" 1000 (Budget.attempts root);
      check_i "work agrees" 1000 (Budget.work root))

let test_domain_inheritance () =
  Budget.with_root cfg ~max_attempts:100 (fun root ->
      let d =
        Domain.spawn (fun () ->
            match Budget.current () with Some c -> c == root | None -> false)
      in
      check_b "a spawned domain starts with the scope's node" true
        (Domain.join d);
      let d =
        Domain.spawn (fun () ->
            Budget.charge (Option.get (Budget.current ())) 1)
      in
      check_b "and charges the same tree" true (Domain.join d);
      check_i "charge visible in the parent" 1 (Budget.attempts root));
  (* a domain that outlives its scope holds a spent handle *)
  let go = Atomic.make false and ready = Atomic.make false in
  let handle = ref None in
  let d =
    Budget.with_root cfg ~max_attempts:100 (fun r ->
        handle := Some r;
        Domain.spawn (fun () ->
            Atomic.set ready true;
            wait_for go;
            (* the inherited ambient chain is wholly closed: current () is the spent node *)
            let h = Option.get !handle in
            let ambient_spent =
              match Budget.current () with Some c -> c == h | None -> false
            in
            let charged = Budget.charge h 1 in
            let v =
              Runner.check ~parent:h ~seed:1L (Gen.int 0 9) (opts ~n:3 ())
                (fun _ -> true)
            in
            let fresh =
              Runner.check ~seed:1L (Gen.int 0 9) (opts ~n:3 ()) (fun _ -> true)
            in
            ( charged,
              fid v,
              v.cases_run,
              ambient_spent
              && fid fresh = "property-exhausted"
              && not fresh.passed )))
  in
  Atomic.set go true;
  let charged, id, cases, fresh_ok = Domain.join d in
  check_b
    "a plain check in a domain that outlives its scope is spent, never a fresh \
     root"
    true fresh_ok;
  check_b "outliving domain: charge refused" false charged;
  check_s "outliving domain: its check is exhausted" "property-exhausted" id;
  check_i "nothing evaluated" 0 cases;
  ignore ready;
  (* a domain that existed before the scope has no handle: a fresh root, bounded by itself *)
  let go = Atomic.make false in
  let d =
    Domain.spawn (fun () ->
        wait_for go;
        let none = Budget.current () = None in
        let v =
          Runner.check ~seed:1L (Gen.int 0 9) (opts ~n:3 ()) (fun _ -> true)
        in
        (none, v.passed))
  in
  Budget.with_root cfg ~max_attempts:1 (fun _ ->
      Atomic.set go true;
      ());
  let none, passed = Domain.join d in
  check_b "pre-existing domain has no ambient scope" true none;
  check_b "and gets a fresh root (documented escape)" true passed;
  (* a property that spawns a domain which runs a nested check on the shared tree *)
  let v =
    Runner.check ~seed:1L (Gen.int 0 9) (opts ~n:5 ~steps:0 ()) (fun _ ->
        let d =
          Domain.spawn (fun () ->
              (Runner.check ~seed:2L (Gen.int 0 9) (opts ~n:2 ~steps:0 ())
                 (fun _ -> true))
                .passed)
        in
        Domain.join d)
  in
  check_b "a domain spawned by a property charges the property's tree" true
    v.passed

(* ===================================================================== *)
(* Strict JSON and TFV                                                     *)
(* ===================================================================== *)

let json_ok s = Result.is_ok (J.parse s)

let test_json_accepts () =
  check_b "object" true
    (json_ok "{\"a\": [1, -2.5e+3, true, false, null, \"x\"]}");
  check_b "whitespace" true (json_ok " \n\t\r [ ] ");
  check_b "empty object" true (json_ok "{}");
  check_b "zero" true (json_ok "0");
  check_b "minus zero" true (json_ok "-0");
  check_b "exponent" true (json_ok "1E5");
  (match
     J.parse "{\"k\": \"\\u0041\\ud83d\\ude00\\n\\/\\b\\f\\r\\t\\\"\\\\\"}"
   with
  | Ok (J.Object [ ("k", J.String s) ]) ->
      check_s "decoded escapes" "A\xF0\x9F\x98\x80\n/\b\012\r\t\"\\" s
  | _ -> Alcotest.fail "expected an object");
  (match J.parse "123456789012345678901234567890" with
  | Ok (J.Number n) ->
      check_s "numbers keep their text" "123456789012345678901234567890" n
  | _ -> Alcotest.fail "number");
  (match J.parse "{\"a\\u0062\": 1}" with
  | Ok (J.Object [ ("ab", _) ]) -> ()
  | _ -> Alcotest.fail "decoded key");
  check_b "64 nested arrays" true
    (json_ok (String.make 64 '[' ^ String.make 64 ']'));
  check_b "64 nested objects" true
    (json_ok
       (String.concat "" (List.init 63 (fun _ -> "{\"a\":"))
       ^ "[]" ^ String.make 63 '}'))

let test_json_rejects () =
  List.iter
    (fun (name, s) -> check_b (name ^ " rejected") false (json_ok s))
    [
      ("empty", "");
      ("trailing data", "1 2");
      ("trailing comma array", "[1,]");
      ("trailing comma object", "{\"a\":1,}");
      ("leading zero", "01");
      ("plus", "+1");
      ("NaN", "NaN");
      ("comment", "1 // x");
      ("bare minus", "-");
      ("dot", "1.");
      ("exponent digits", "1e");
      ("single quote", "'a'");
      ("unterminated string", "\"abc");
      ("raw control", "\"a\001b\"");
      ("bad escape", "\"\\q\"");
      ("unterminated escape", "\"\\");
      ("lone high surrogate", "\"\\ud83d\"");
      ("reversed surrogates", "\"\\ude00\\ud83d\"");
      ("lone low surrogate", "\"\\ude00\"");
      ("bad hex", "\"\\u12G4\"");
      ("short hex", "\"\\u12\"");
      ("duplicate keys", "{\"a\":1,\"a\":2}");
      ("duplicate by decoded bytes", "{\"a\":1,\"\\u0061\":2}");
      ("non-string key", "{1:2}");
      ("missing colon", "{\"a\" 1}");
      ("missing comma", "[1 2]");
      ("missing comma object", "{\"a\":1 \"b\":2}");
      ("BOM", "\xEF\xBB\xBF{}");
      ("invalid utf-8", "\"\xFF\"");
      ("overlong", "\"\xC0\x80\"");
      ("surrogate utf-8", "\"\xED\xA0\x80\"");
      ("above 10FFFF", "\"\xF4\x90\x80\x80\"");
      ("bad literal", "tru");
      ("unexpected byte", "@");
      ("unclosed array", "[1");
      ("unclosed object", "{\"a\":1");
    ];
  check_b "65 nested arrays" false
    (json_ok (String.make 65 '[' ^ String.make 65 ']'));
  check_b "65 nested objects" false
    (json_ok
       (String.concat "" (List.init 65 (fun _ -> "{\"a\":"))
       ^ "0" ^ String.make 65 '}'));
  (* refused BEFORE recursing: a hostile nesting cannot exhaust the native stack *)
  check_b "100,000-deep nesting refused" false
    (json_ok (String.make 100_000 '['));
  check_b "1,000,000-deep nesting refused" false
    (json_ok (String.make 1_000_000 '['));
  check_b "more than 200,000 values" false
    (json_ok ("[" ^ String.concat "," (List.init 200_001 (fun _ -> "0")) ^ "]"));
  check_b "exactly 200,000 values (one is the array)" true
    (json_ok ("[" ^ String.concat "," (List.init 199_999 (fun _ -> "0")) ^ "]"));
  check_b "object keys count as values" false
    (json_ok
       ("{"
       ^ String.concat ","
           (List.init 100_001 (fun i -> Printf.sprintf "\"%d\":0" i))
       ^ "}"));
  check_b "8 MiB + 1 refused" false
    (json_ok ("\"" ^ String.make (8 * 1024 * 1024) 'a' ^ "\""));
  check_b "just under 8 MiB accepted" true
    (json_ok ("\"" ^ String.make ((8 * 1024 * 1024) - 3) 'a' ^ "\""));
  match J.parse "01" with
  | Error m -> check_b "errors carry a position" true (contains m "byte")
  | Ok _ -> Alcotest.fail "x"

let tfv_ok s = Tfv.valid s

let test_tfv () =
  let good =
    "tfv 1\n\
     # a comment\n\n\
     rec/1 R2 Ka I-12 Kb S68690a\n\
     rec/2 R1 Kl L2 T N\n\
     rec.3 R2 Km M1 I1 X00 Kf D3ff0000000000000\n"
  in
  ignore good;
  check_b "minimal" true (tfv_ok "tfv 1\n");
  check_b "header without newline" true (tfv_ok "tfv 1");
  check_b "records" true
    (tfv_ok "tfv 1\n# c\n\nr1 R2 Ka I-12 Kb S6869\nr2 R1 Kl L2 T N\n");
  check_b "map" true (tfv_ok "tfv 1\nr R1 Km M2 I1 X00 I2 X01\n");
  check_b "float" true (tfv_ok "tfv 1\nr R1 Kf D3ff0000000000000\n");
  check_b "nan floats are one key" false
    (tfv_ok "tfv 1\nr R1 Km M2 D7ff8000000000000 I1 D7ff0000000000001 I2\n");
  check_b "zero floats are distinct" true
    (tfv_ok "tfv 1\nr R1 Km M2 D0000000000000000 I1 D8000000000000000 I2\n");
  check_b "infinity is a float" true
    (tfv_ok "tfv 1\nr R1 Kf D7ff0000000000000\n");
  List.iter
    (fun (name, s) -> check_b (name ^ " rejected") false (tfv_ok s))
    [
      ("bad header", "bad header\n");
      ("empty", "");
      ("uppercase hex", "tfv 1\nr R1 Kop S706274 Kdata XFF\n");
      ("odd hex", "tfv 1\nr R1 Kd X0\n");
      ("-0", "tfv 1\nr R1 Ka I-0\n");
      ("leading zero int", "tfv 1\nr R1 Ka I007\n");
      ("empty int", "tfv 1\nr R1 Ka I\n");
      ("non-digit int", "tfv 1\nr R1 Ka I1x\n");
      ("count disagrees", "tfv 1\nr R2 Ka I1\n");
      ("count too low", "tfv 1\nr R1 Ka I1 Kb I2\n");
      ("trailing tokens", "tfv 1\nr R1 Ka I1 I2\n");
      ("doubled space", "tfv 1\nr R1  Ka I1\n");
      ("trailing space", "tfv 1\nr R1 Ka I1 \n");
      ("duplicate field", "tfv 1\nr R2 Ka I1 Ka I2\n");
      ("duplicate record id", "tfv 1\nr R1 Ka I1\nr R1 Ka I2\n");
      ("bad utf-8 in S", "tfv 1\nr R1 Ka Sff\n");
      ("carriage return", "tfv 1\nr R1 Ka I1\r\n");
      ("bad record id", "tfv 1\nr! R1 Ka I1\n");
      ("missing R", "tfv 1\nr X1 Ka I1\n");
      ("bad count", "tfv 1\nr Rx Ka I1\n");
      ("leading zero count", "tfv 1\nr R01 Ka I1\n");
      ("count too large", "tfv 1\nr R1 Ka L9999999999 \n");
      ("bad field key", "tfv 1\nr R1 KA I1\n");
      ("field without K", "tfv 1\nr R1 a I1\n");
      ("unknown tag", "tfv 1\nr R1 Ka Q1\n");
      ("N with body", "tfv 1\nr R1 Ka N1\n");
      ("short float", "tfv 1\nr R1 Ka D00\n");
      ("record ends early", "tfv 1\nr R1 Ka\n");
      ("record ends in list", "tfv 1\nr R1 Ka L2 T\n");
      ("missing id", "tfv 1\nr\n");
      ("duplicate map key", "tfv 1\nr R1 Km M2 I1 I1 I1 I2\n");
      ("not utf-8", "tfv 1\n\xFF\n");
      ("1001-digit int", "tfv 1\nr R1 Ka I" ^ String.make 1001 '7' ^ "\n");
      ("empty count", "tfv 1\nr R1 Ka L\n");
      ("non-digit count", "tfv 1\nr R1 Ka Lx\n");
    ];
  check_b "odd-length S hex" false (tfv_ok "tfv 1\nr R1 Ka S6\n");
  check_b "1000-digit int" true
    (tfv_ok ("tfv 1\nr R1 Ka I" ^ String.make 1000 '7' ^ "\n"));
  let nest d = String.concat " " (List.init d (fun _ -> "L1")) ^ " N" in
  check_b "64-deep list" true (tfv_ok ("tfv 1\nr R1 Ka " ^ nest 64 ^ "\n"));
  check_b "65-deep list" false (tfv_ok ("tfv 1\nr R1 Ka " ^ nest 65 ^ "\n"));
  check_b "1 MiB + 1 refused" false
    (tfv_ok ("tfv 1\n#" ^ String.make 1_048_576 'a' ^ "\n"));
  check_b "too many tokens" false
    (tfv_ok
       ("tfv 1\nr R1 Ka L190000 "
       ^ String.concat " " (List.init 190000 (fun _ -> "N"))
       ^ "\nq R1 Ka L20000 "
       ^ String.concat " " (List.init 20000 (fun _ -> "N"))
       ^ "\n"));
  (match Tfv.validate "bad header\n" with
  | Error m -> check_b "reason" true (contains m "tfv 1")
  | Ok () -> Alcotest.fail "x");
  (match Tfv.validate "tfv 1\nr R1 Ka I-0\n" with
  | Error m -> check_b "line number in reason" true (contains m "line 2")
  | Ok () -> Alcotest.fail "x");
  check_b "constants" true
    (Tfv.max_input = 1_048_576 && Tfv.max_tokens = 200_000)

let test_values_decode () =
  let dec s =
    match J.parse s with Ok j -> Values.decode j | Error e -> Error e
  in
  let ok s = Result.is_ok (dec s) in
  check_b "int" true (ok "{\"kind\":\"int\",\"value\":\"-12\"}");
  check_b "zero" true (ok "{\"kind\":\"int\",\"value\":\"0\"}");
  check_b "bytes" true (ok "{\"kind\":\"bytes\",\"hex\":\"00ff\"}");
  check_b "empty bytes" true (ok "{\"kind\":\"bytes\",\"hex\":\"\"}");
  check_b "bool" true (ok "true");
  check_b "text" true (ok "\"abc\"");
  check_b "list" true (ok "[true,\"a\",[]]");
  List.iter
    (fun (n, s) -> check_b (n ^ " rejected") false (ok s))
    [
      ("number literal", "5");
      ("null", "null");
      ("-0", "{\"kind\":\"int\",\"value\":\"-0\"}");
      ("leading zero", "{\"kind\":\"int\",\"value\":\"01\"}");
      ("plus", "{\"kind\":\"int\",\"value\":\"+1\"}");
      ("empty", "{\"kind\":\"int\",\"value\":\"\"}");
      ("int value not a string", "{\"kind\":\"int\",\"value\":1}");
      ("extra field", "{\"kind\":\"int\",\"value\":\"1\",\"x\":1}");
      ("uppercase hex", "{\"kind\":\"bytes\",\"hex\":\"FF\"}");
      ("odd hex", "{\"kind\":\"bytes\",\"hex\":\"0\"}");
      ("hex not a string", "{\"kind\":\"bytes\",\"hex\":1}");
      ("unknown kind", "{\"kind\":\"float\",\"value\":\"1\"}");
      ("no kind", "{}");
      ( "1001 digits",
        "{\"kind\":\"int\",\"value\":\"" ^ String.make 1001 '7' ^ "\"}" );
      ( "-1000 digits ok is fine but 1001 negative is not",
        "{\"kind\":\"int\",\"value\":\"-" ^ String.make 1001 '7' ^ "\"}" );
      ("list element bad", "[5]");
      ( "bytes too long",
        "{\"kind\":\"bytes\",\"hex\":\"" ^ String.make 20_002 '0' ^ "\"}" );
      ( "list too long",
        "[" ^ String.concat "," (List.init 10_001 (fun _ -> "true")) ^ "]" );
      ( "too deep",
        String.concat "" (List.init 70 (fun _ -> "["))
        ^ String.concat "" (List.init 70 (fun _ -> "]")) );
    ];
  check_b "1000 digits accepted" true
    (ok ("{\"kind\":\"int\",\"value\":\"" ^ String.make 1000 '7' ^ "\"}"));
  check_b "-1000 digits accepted" true
    (ok ("{\"kind\":\"int\",\"value\":\"-" ^ String.make 1000 '7' ^ "\"}"));
  check_b "canonical_int" true
    (Values.canonical_int "0" && Values.canonical_int "-5"
    && (not (Values.canonical_int ""))
    && not (Values.canonical_int "-"))

(* ===================================================================== *)
(* Corpus: replay, negative controls, sabotage                             *)
(* ===================================================================== *)

let corpus_cases () =
  match Corpus.parse_corpus Corpus_data.json with
  | Ok c -> c
  | Error e -> Alcotest.fail e

let test_corpus_replay () =
  let r = Corpus.evaluate_text Corpus_data.json in
  check_i "evaluated" 25 r.evaluated;
  Alcotest.(check (list string)) "no problems" [] r.problems

let field k = function
  | J.Object f -> List.assoc k f
  | _ -> Alcotest.fail "object"

let replace_field k v = function
  | J.Object f ->
      J.Object
        (List.map (fun (k', v') -> if k' = k then (k', v) else (k', v')) f)
  | j -> j

let id_of c = match field "id" c with J.String s -> s | _ -> ""
let op_of c = match field "op" c with J.String s -> s | _ -> ""
let int_node s = J.Object [ ("kind", J.String "int"); ("value", J.String s) ]

let rec corrupt_val = function
  | J.Bool b -> J.Bool (not b)
  | J.Object [ ("kind", J.String "int"); ("value", J.String v) ] ->
      int_node (if v = "123456789" then "987654321" else "123456789")
  | J.Object [ ("kind", J.String "bytes"); ("hex", J.String h) ] ->
      J.Object [ ("kind", J.String "bytes"); ("hex", J.String (h ^ "ab")) ]
  | J.Array (x :: rest) -> J.Array (corrupt_val x :: rest)
  | J.Array [] -> J.Array [ J.Bool true ]
  | j -> j

let steps_plus_one = function
  | J.Object [ ("kind", J.String "int"); ("value", J.String v) ] ->
      int_node (string_of_int (int_of_string v + 1))
  | j -> j

let corruptions c =
  match op_of c with
  | "gen_primitive" | "gen_bounded" | "gen_bytes" ->
      [ replace_field "outputs" (corrupt_val (field "outputs" c)) c ]
  | "shrink_int" | "shrink_refinement" | "shrink_list" | "shrink_bytes" ->
      [
        replace_field "expected_minimal"
          (corrupt_val (field "expected_minimal" c))
          c;
        replace_field "expected_steps"
          (steps_plus_one (field "expected_steps" c))
          c;
      ]
  | "replay_consistency" ->
      [
        replace_field "expected_minimal"
          (corrupt_val (field "expected_minimal" c))
          c;
      ]
  | "filter_exhausted" ->
      [ replace_field "failure_id" (J.String "property-panic") c ]
  | "failure_id_valid" ->
      [ replace_field "valid" (corrupt_val (field "valid" c)) c ]
  | "corpus_reject" ->
      [ replace_field "text" (J.String "tfv 1\nr R1 Kop S706274\n") c ]
  | _ -> []

let test_corpus_negative_controls () =
  let cases = corpus_cases () in
  check_i "25 real cases" 25 (List.length cases);
  List.iteri
    (fun i c ->
      List.iter
        (fun bad ->
          let cases' = List.mapi (fun j x -> if i = j then bad else x) cases in
          let r = Corpus.evaluate cases' in
          check_b
            (id_of c ^ ": a corruption is reported")
            true (r.problems <> []);
          List.iter
            (fun p ->
              check_b
                (id_of c ^ ": every problem names the case: " ^ p)
                true
                (String.length p >= String.length (id_of c)
                && String.sub p 0 (String.length (id_of c)) = id_of c))
            r.problems)
        (corruptions c))
    cases;
  let has_problem r s = List.exists (fun p -> contains p s) r.Corpus.problems in
  (* truncated and dropped-case lists, unknown ops, structural damage *)
  check_b "dropped case" true
    (has_problem (Corpus.evaluate (List.tl cases)) "expected 25 cases");
  check_b "truncated list" true
    (has_problem
       (Corpus.evaluate (List.filteri (fun i _ -> i < 10) cases))
       "expected 25 cases");
  check_b "ops never exercised" true
    (has_problem
       (Corpus.evaluate ~expected:None (List.filteri (fun i _ -> i < 10) cases))
       "never exercised");
  check_b "failure ids missing" true
    (has_problem
       (Corpus.evaluate ~expected:None
          (List.filter (fun c -> op_of c <> "failure_id_valid") cases))
       "failure ids missing");
  check_b "unregistered rejection missing" true
    (has_problem
       (Corpus.evaluate ~expected:None
          (List.filter
             (fun c -> id_of c <> "pbt/failure_id/unregistered")
             cases))
       "unregistered-id rejection");
  check_b "unknown op" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [ replace_field "op" (J.String "nope") (List.hd cases) ])
       "unknown op");
  check_b "extra field" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [
            (match List.hd cases with
            | J.Object f -> J.Object (("extra", J.Bool true) :: f)
            | j -> j);
          ])
       "fields");
  check_b "unknown property" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [
            replace_field "property" (J.String "nope")
              (List.find (fun c -> op_of c = "shrink_int") cases);
          ])
       "unknown property");
  check_b "outputs length != count" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [ replace_field "outputs" (J.Array []) (List.hd cases) ])
       "exactly count");
  check_b "case not an object" true
    (has_problem (Corpus.evaluate ~expected:None [ J.Null ]) "not an object");
  check_b "case without id" true
    (has_problem
       (Corpus.evaluate ~expected:None [ J.Object [ ("op", J.String "x") ] ])
       "missing a string id");
  check_b "duplicate ids" true
    (has_problem
       (Corpus.evaluate ~expected:None [ List.hd cases; List.hd cases ])
       "duplicate or missing");
  check_b "non-canonical int" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [ replace_field "seed" (int_node "007") (List.hd cases) ])
       "could not be evaluated");
  check_b "seed above 2^64" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [
            replace_field "seed"
              (int_node "18446744073709551616")
              (List.hd cases);
          ])
       "seed outside");
  check_b "negative seed refused, not reduced" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [ replace_field "seed" (int_node "-1") (List.hd cases) ])
       "seed outside");
  check_b "count above 10000" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [ replace_field "count" (int_node "10001") (List.hd cases) ])
       "count outside");
  check_b "1001-digit fixture integer" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [
            replace_field "seed"
              (int_node (String.make 1001 '7'))
              (List.hd cases);
          ])
       "could not be evaluated");
  check_b "gen_bytes count x max_len cap" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [
            (let c = List.find (fun c -> op_of c = "gen_bytes") cases in
             replace_field "outputs"
               (J.Array
                  (List.init 5000 (fun _ ->
                       J.Object
                         [ ("kind", J.String "bytes"); ("hex", J.String "") ])))
               (replace_field "count" (int_node "5000")
                  (replace_field "max_len" (int_node "9000") c)));
          ])
       "exceeds the generation budget");
  check_b "type must be a string" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [ replace_field "type" (int_node "1") (List.hd cases) ])
       "type must be a string");
  check_b "unknown primitive" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [ replace_field "type" (J.String "f32") (List.hd cases) ])
       "unknown primitive");
  check_b "max_retries 0" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [
            replace_field "max_retries" (int_node "0")
              (List.find (fun c -> op_of c = "filter_exhausted") cases);
          ])
       "at least 1");
  check_b "filter verdict must be fail" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [
            replace_field "verdict" (J.String "pass")
              (List.find (fun c -> op_of c = "filter_exhausted") cases);
          ])
       "must be a fail verdict");
  check_b "shrink_int initial out of range" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [
            replace_field "initial" (int_node "999999999")
              (List.find (fun c -> op_of c = "shrink_int") cases);
          ])
       "could not be evaluated");
  check_b "shrink_list initial must be a list" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [
            replace_field "initial" (J.Bool true)
              (List.find (fun c -> op_of c = "shrink_list") cases);
          ])
       "list of ints");
  check_b "shrink_bytes initial must be bytes" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [
            replace_field "initial" (J.Bool true)
              (List.find (fun c -> op_of c = "shrink_bytes") cases);
          ])
       "must be bytes");
  check_b "corpus_reject text must be a string" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [
            replace_field "text" (J.Bool true)
              (List.find (fun c -> op_of c = "corpus_reject") cases);
          ])
       "text must be a string");
  check_b "shrink_int property of the wrong table" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [
            replace_field "property" (J.String "list_sum_le_20")
              (List.find (fun c -> op_of c = "shrink_int") cases);
          ])
       "unknown property");
  check_b "case cap" true
    (has_problem
       (Corpus.evaluate ~expected:None (List.init 10_001 (fun _ -> J.Null)))
       "exceeds the cap");
  check_b "problems are capped" true
    (List.length
       (Corpus.evaluate ~expected:None (List.init 5000 (fun _ -> J.Null)))
         .problems
    <= Corpus.max_problems);
  check_b "bad id type" true
    (has_problem
       (Corpus.evaluate ~expected:None
          [ replace_field "id" (J.Bool true) (List.hd cases) ])
       "missing a string id")

let test_corpus_parse () =
  let bad s = Result.is_error (Corpus.parse_corpus s) in
  check_b "not json" true (bad "nope");
  check_b "wrong shape" true (bad "[]");
  check_b "extra key" true (bad "{\"schema_version\":1,\"cases\":[],\"x\":1}");
  check_b "wrong version" true (bad "{\"schema_version\":2,\"cases\":[]}");
  check_b "version missing" true (bad "{\"x\":1,\"cases\":[]}");
  check_b "cases not an array" true (bad "{\"schema_version\":1,\"cases\":5}");
  check_b "good" false (bad "{\"schema_version\":1,\"cases\":[]}");
  let r = Corpus.evaluate_text "nope" in
  check_b "a parse failure is a problem" true
    (r.problems <> [] && r.evaluated = 0)

let test_corpus_sabotage () =
  let broken name c =
    let r = Corpus.evaluate_text ~config:c Corpus_data.json in
    check_b (name ^ " is detected by the corpus") true (r.problems <> [])
  in
  broken "a changed bisection"
    {
      cfg with
      bisect = (fun cur diff -> B.sub cur (B.add (B.fdiv2 diff) B.one));
    };
  (* the corpus never bisects an odd negative distance, so truncation (-41 instead
     of -40 from -81) is another blind spot that only the pin in test_shrink_floor sees *)
  let trunc =
    {
      cfg with
      bisect = (fun cur diff -> B.sub cur (fst (B.div_rem diff (B.of_int 2))));
    }
  in
  Alcotest.(check (list string))
    "truncating bisection passes the corpus" []
    (Corpus.evaluate_text ~config:trunc Corpus_data.json).problems;
  let seen = ref [] in
  ignore
    (Shrink.shrink_int ~config:trunc ~initial:(B.of_int (-81)) ~target:B.zero
       ~lo:(B.of_int (-100)) ~hi:B.zero (fun x ->
         if bs x = "-81" then false
         else (
           seen := bs x :: !seen;
           true)));
  Alcotest.(check (list string))
    "but the -81 pin sees -41" [ "0"; "-41" ]
    (List.rev !seen |> List.filteri (fun i _ -> i < 2));
  broken "a changed unit step"
    {
      cfg with
      unit_step =
        (fun cur diff ->
          B.sub cur (B.mul (B.of_int 2) (B.of_int (B.sign diff))));
    };
  broken "a skipped deletion phase" { cfg with skip_deletion = true };
  broken "a shifted RNG"
    { cfg with new_rng = (fun s -> Prng.create (Int64.add s 1L)) }

(* The blind spot is real: a threshold of 0 still passes the whole corpus. *)
let test_threshold_blind_spot () =
  let zero =
    { cfg with new_rng = (fun s -> Prng.create ~threshold:(fun _ -> 0L) s) }
  in
  let r = Corpus.evaluate_text ~config:zero Corpus_data.json in
  Alcotest.(check (list string))
    "a threshold of 0 still passes all 25 cases" [] r.problems;
  check_i "evaluated" 25 r.evaluated;
  (* ... and only the forced-rejection pin catches it *)
  let seed = 7046029254386353131L in
  check_s "the correct threshold rejects the 0 word" "1"
    (bs (Prng.bounded (Prng.create seed) (B.of_int 3)));
  check_s "a threshold of 0 answers 0" "0"
    (bs (Prng.bounded (Prng.create ~threshold:(fun _ -> 0L) seed) (B.of_int 3)));
  (* the signed spelling is wrong above 2^63 *)
  let n = Int64.add Int64.min_int 1L in
  check_b "signed remainder would differ above 2^63" true
    (Int64.rem (Int64.neg n) n <> Prng.default_threshold n)

let test_corpus_gen_caps () =
  let raises f =
    try
      ignore (f ());
      false
    with Error.Argument_error _ -> true
  in
  check_b "count too large" true
    (raises (fun () ->
         Gen.gen_primitive cfg 0L "bool" (Types.max_gen_count + 1)));
  check_b "negative count" true
    (raises (fun () -> Gen.gen_primitive cfg 0L "bool" (-1)));
  check_b "lo > hi" true
    (raises (fun () -> Gen.gen_bounded cfg 0L B.one B.zero 1));
  check_b "span too wide" true
    (raises (fun () -> Gen.gen_bounded cfg 0L B.zero (B.pow2 20_000) 1));
  check_b "count x width" true
    (raises (fun () -> Gen.gen_bounded cfg 0L B.zero (B.pow2 800) 20_000));
  check_b "bytes min>max" true (raises (fun () -> Gen.gen_bytes cfg 0L 5 1 1));
  check_b "bytes count x max_len" true
    (raises (fun () -> Gen.gen_bytes cfg 0L 0 1000 5000));
  check_i "bytes empty max_len ok" 3 (List.length (Gen.gen_bytes cfg 0L 0 0 3));
  check_i "bool count" 5 (List.length (Gen.gen_primitive cfg 0L "bool" 5))

let test_types () =
  check_b "failure ids" true
    (List.for_all Types.is_failure_id
       [
         "property-counterexample";
         "property-exhausted";
         "property-timeout";
         "property-panic";
       ]);
  check_b "unregistered" false (Types.is_failure_id "unregistered-id");
  check_i "attempt budget" 16064 (Types.attempt_budget 1000);
  check_i "attempt budget negative" 64 (Types.attempt_budget (-5))

(* ===================================================================== *)
(* Edges: work accounting, spent scopes, synthetic corpus cases            *)
(* ===================================================================== *)

let test_val_work () =
  let i n = Val.Int (B.of_int n) in
  check_i "nil" 0 (Runner.val_work Val.Nil);
  check_i "int" 0 (Runner.val_work (i 5));
  check_i "bytes" 5 (Runner.val_work (Val.Bytes "hello"));
  check_i "text" 3 (Runner.val_work (Val.Text "abc"));
  check_i "flat list" 3 (Runner.val_work (Val.List [ i 1; i 2; i 3 ]));
  check_i "tuple" 2 (Runner.val_work (Val.Tuple [ i 1; i 2 ]));
  check_i "one level deeper: lengths of children"
    (2 + 3 + 2)
    (Runner.val_work (Val.List [ Val.Bytes "abc"; Val.List [ i 1; i 2 ] ]));
  check_i "tuple children"
    (2 + 1 + 2)
    (Runner.val_work (Val.Tuple [ Val.Text "a"; Val.Tuple [ i 1; i 2 ] ]));
  check_i "at most 10,000 children" (10_000 + 0)
    (Runner.val_work (Val.List (List.init 20_000 (fun _ -> Val.Nil))));
  (* a nested check pays for the size of what it samples; a throwing canon costs 0 *)
  let nested_with g =
    let outer =
      Runner.check ~seed:1L (Gen.int 0 3) (opts ~n:1 ~steps:0 ()) (fun _ ->
          (Runner.check ~seed:1L g (opts ~n:3 ~steps:0 ()) (fun _ -> true))
            .passed)
    in
    outer.passed
  in
  check_b "nested list of bytes" true
    (nested_with (Gen.list ~max_len:5 (Gen.bytes 0 4)));
  check_b "nested tuple" true
    (nested_with (Gen.tuple2 (Gen.bytes 0 3) (Gen.list ~max_len:3 Gen.bool)));
  check_b "nested throwing canon costs nothing" true
    (nested_with (Gen.custom ~canon:(fun _ -> failwith "canon") (fun _ _ -> 1)));
  (* a nested check whose samples exceed the work allowance is starved, never a pass *)
  let starved =
    Budget.with_root cfg ~max_attempts:1000 (fun root ->
        Runner.check ~parent:root ~seed:1L (Gen.bytes 1000 1000)
          (opts ~n:3 ~steps:0 ()) (fun _ -> true))
  in
  check_b "work within the allowance passes" true starved.passed;
  let tiny = { cfg with max_work = 100 } in
  let starved =
    Budget.with_root tiny ~max_attempts:1000 (fun root ->
        Runner.check ~parent:root ~seed:1L (Gen.bytes 1000 1000)
          (opts ~n:3 ~steps:0 ()) (fun _ -> true))
  in
  check_b "work past the root allowance is starved" false starved.passed;
  check_s "reported as exhausted" "property-exhausted" (fid starved)

let test_top_level_starvation () =
  (* a property that spends the tree's work and is refused: a would-be pass is exhausted *)
  let v =
    Runner.check ~seed:1L (Gen.int 0 9) (opts ~n:3 ~steps:0 ()) (fun _ ->
        let scope = Option.get (Budget.current ()) in
        ignore (Budget.charge_copy scope max_int);
        true)
  in
  check_b "starved tree never passes" false v.passed;
  check_s "exhausted" "property-exhausted" (fid v);
  check_b "message says starved" true
    (contains (Option.get v.message) "starved");
  (* a falsified verdict is never overridden by starvation *)
  let v =
    Runner.check ~seed:1L (Gen.int 0 9) (opts ~n:3 ~steps:0 ()) (fun _ ->
        ignore (Budget.charge_copy (Option.get (Budget.current ())) max_int);
        false)
  in
  check_s "falsified stays falsified" "property-counterexample" (fid v)

let test_spent_scopes () =
  let stale = ref None in
  Budget.with_root cfg ~max_attempts:50 (fun r -> stale := Some r);
  let parent = Option.get !stale in
  Alcotest.(check (pair string int))
    "shrink_int under a spent parent returns the input" ("77", 0)
    (let m, s =
       Shrink.shrink_int ~parent ~initial:(B.of_int 77) ~target:B.zero
         ~lo:B.zero ~hi:(B.of_int 100) (fun _ -> false)
     in
     (bs m, s));
  Alcotest.(check (pair (list string) int))
    "shrink_list under a spent parent"
    ([ "1"; "2" ], 0)
    (let m, s =
       Shrink.shrink_list ~parent
         ~initial:[ B.one; B.of_int 2 ]
         ~elem_target:B.zero
         (fun _ -> false)
     in
     (List.map bs m, s));
  Alcotest.(check (pair string int))
    "shrink_bytes under a spent parent" ("ab", 0)
    (Shrink.shrink_bytes ~parent ~initial:"ab" (fun _ -> false));
  let g = Gen.bytes 0 8 in
  Alcotest.(check string)
    "bytes shrinker under a spent parent returns the input" "abc"
    (fst (Gen.shrink ~parent g "abc" (fun _ -> true) 10));
  Alcotest.(check string)
    "bytes shrinker with 0 steps" "abc"
    (fst (Gen.shrink g "abc" (fun _ -> true) 0));
  Alcotest.(check (list int))
    "list shrinker under a spent parent" [ 1; 2 ]
    ( fst
        (Gen.shrink ~parent
           (Gen.list (Gen.int 0 9))
           [ 1; 2 ]
           (fun _ -> true)
           10)
    |> fun l -> l );
  Alcotest.(check bool)
    "charged_falsifies stops when refused" true
    (Budget.with_root cfg ~max_attempts:1 (fun r ->
         Budget.charged_falsifies r (fun _ -> true) ()
         && not (Budget.charged_falsifies r (fun _ -> true) ())));
  (* a tight root starves the deletion phase *)
  Budget.with_root { cfg with max_work = 1_000_000 } ~max_attempts:6
    (fun root ->
      let m, _ =
        Shrink.shrink_list ~parent:root
          ~initial:(List.init 40 (fun i -> B.of_int (i + 1)))
          ~elem_target:B.zero
          (fun l -> List.length l < 2)
      in
      check_b "still falsifying under a starved budget" true (List.length m >= 2);
      check_b "the budget was refused" true (Budget.is_refused root));
  (* a sequence shrinker with no element shrinker *)
  let a, steps =
    Shrink.shrink_sequence cfg
      (Budget.with_root cfg ~max_attempts:1000 (fun r -> r |> fun _ -> r))
      ~min_len:0 None
      (fun _ -> false)
      10 [| 1; 2; 3 |]
  in
  ignore (a, steps)

let test_gen_canon_and_edges () =
  let r = Prng.create 1L in
  let render g = Canon.render (g.Gen.canon (Gen.sample g r)) in
  check_b "bool canon" true
    (render Gen.bool = "true" || render Gen.bool = "false");
  check_b "tuple2 canon" true
    (String.length (render (Gen.tuple2 Gen.u8 Gen.bool)) > 3);
  check_b "tuple3 canon" true
    (render
       (Gen.tuple3
          (Gen.constant ~canon:(fun _ -> Val.Nil) ())
          (Gen.constant ~canon:(fun _ -> Val.Nil) ())
          (Gen.constant ~canon:(fun _ -> Val.Nil) ()))
    = "(nil,nil,nil)");
  check_b "bytes canon" true (String.length (render (Gen.bytes 3 3)) = 8);
  check_b "list canon" true
    (render (Gen.list ~min_len:2 ~max_len:2 (Gen.int 0 0)) = "[0,0]");
  check_b "bool false does not shrink" false
    (fst (Gen.shrink Gen.bool false (fun _ -> true) 10));
  check_b "bool with 0 steps" true
    (fst (Gen.shrink Gen.bool true (fun _ -> true) 0));
  (* tuples whose first coordinate uses the whole step budget *)
  let t2 = Gen.tuple2 (Gen.int 0 1000) (Gen.int 0 1000) in
  let (_, y), steps = Gen.shrink t2 (900, 800) (fun _ -> true) 1 in
  check_i "tuple2 stops when steps are spent" 800 y;
  check_i "one step" 1 steps;
  let t3 = Gen.tuple3 (Gen.int 0 1000) (Gen.int 0 1000) (Gen.int 0 1000) in
  let (_, y, z), _ = Gen.shrink t3 (900, 800, 700) (fun _ -> true) 1 in
  check_b "tuple3 stops when steps are spent" true (y = 800 && z = 700);
  let (_, _, z), _ =
    Gen.shrink t3 (900, 800, 700) (fun (x, _, _) -> x >= 0 && true) 2
  in
  check_b "tuple3 second coordinate may use the rest" true (z = 700 || z >= 0);
  (* shrink_sequence with no element shrinker *)
  let b =
    Budget.with_root cfg ~max_attempts:1000 (fun r ->
        Budget.with_child cfg (Some r) ~max_steps:10 (fun c -> c))
  in
  ignore b;
  Budget.with_root cfg ~max_attempts:1000 (fun r ->
      let a, steps =
        Shrink.shrink_sequence cfg r ~min_len:1 None
          (fun a -> Array.length a >= 2)
          10 [| 1; 2; 3; 4 |]
      in
      Alcotest.(check (array int)) "no element phase" [| 1; 2 |] a;
      check_b "steps counted" true (steps >= 1))

let obj fields = J.Object fields
let nint n = int_node (string_of_int n)
let bytes_node h = J.Object [ ("kind", J.String "bytes"); ("hex", J.String h) ]

let case id op fields =
  obj (("id", J.String id) :: ("op", J.String op) :: fields)

let test_corpus_synthetic () =
  let ok name c =
    let r = Corpus.evaluate ~expected:None [ c ] in
    Alcotest.(check (list string))
      name []
      (List.filter
         (fun p ->
           not
             (contains p "never exercised"
             || contains p "failure ids missing"
             || contains p "rejection"))
         r.problems);
    check_i (name ^ " evaluated") 1 r.evaluated
  in
  ok "even property"
    (case "t/even" "shrink_int"
       [
         ("initial", nint 7);
         ("target", nint 0);
         ("min", nint 0);
         ("max", nint 100);
         ("property", J.String "int_is_even");
         ("expected_minimal", nint 7);
         ("expected_steps", nint 0);
       ]);
  ok "abs property"
    (case "t/abs" "shrink_int"
       [
         ("initial", nint 50);
         ("target", nint 0);
         ("min", nint (-100));
         ("max", nint 100);
         ("property", J.String "int_abs_le_10");
         ("expected_minimal", nint 11);
         ("expected_steps", nint 4);
       ]);
  ok "no zero byte"
    (case "t/nz" "shrink_bytes"
       [
         ("initial", bytes_node "0100");
         ("property", J.String "bytes_no_zero_byte");
         ("expected_minimal", bytes_node "00");
         ("expected_steps", nint 1);
       ]);
  ok "list sum"
    (case "t/sum" "shrink_list"
       [
         ("initial", J.Array [ nint 30 ]);
         ("elem_target", nint 0);
         ("property", J.String "list_sum_le_20");
         ("expected_minimal", J.Array [ nint 21 ]);
         ("expected_steps", nint 9);
       ]);
  let has c s =
    List.exists
      (fun p -> contains p s)
      (Corpus.evaluate ~expected:None [ c ]).problems
  in
  check_b "seed must be an int" true
    (has
       (case "t/seed" "replay_consistency"
          [
            ("seed", J.String "x");
            ("property", J.String "int_le_100");
            ("initial_fail", nint 413);
            ("expected_minimal", nint 101);
          ])
       "must be an int");
  check_b "wrong value reported" true
    (has
       (case "t/wrong" "shrink_int"
          [
            ("initial", nint 7);
            ("target", nint 0);
            ("min", nint 0);
            ("max", nint 100);
            ("property", J.String "int_is_even");
            ("expected_minimal", nint 8);
            ("expected_steps", nint 0);
          ])
       "minimal mismatch");
  check_b "filter verdict field of the wrong kind" true
    (has
       (case "t/f" "filter_exhausted"
          [
            ("max_retries", nint 3);
            ("verdict", nint 1);
            ("failure_id", J.String "property-exhausted");
          ])
       "must be a fail verdict");
  check_b "filter failure id mismatch" true
    (has
       (case "t/f2" "filter_exhausted"
          [
            ("max_retries", nint 3);
            ("verdict", J.String "fail");
            ("failure_id", J.String "property-timeout");
          ])
       "failure id");
  check_b "failure_id_valid with a wrong kind" true
    (has
       (case "t/v" "failure_id_valid"
          [ ("input", J.String "property-panic"); ("valid", nint 1) ])
       "membership disagrees");
  check_b "failure_id_valid with a non-text input" true
    (has
       (case "t/v2" "failure_id_valid"
          [ ("input", nint 1); ("valid", J.Bool true) ])
       "membership disagrees")

let test_runner_misc () =
  (* exceptions with escapable names stay on one line; engine errors are named *)
  check_b "Argument_error named" true
    (contains (Safe.exn_name (Error.Argument_error "x")) "Argument_error");
  let v =
    Runner.check ~seed:1L int_gen (opts ~n:3 ()) (fun _ ->
        raise (Error.Argument_error "x"))
  in
  check_s "engine errors from a property are panics" "property-panic" (fid v);
  let v =
    Runner.check ~seed:1L int_gen (opts ~n:3 ~ci:Ci_on ()) (fun _ ->
        raise Not_found)
  in
  check_b "redacted generator/ panic message" true
    (contains (Option.get v.message) "Not_found");
  let sampler_fail =
    Gen.custom ~canon:(fun _ -> Val.Nil) (fun _ _ -> failwith "x")
  in
  let v =
    Runner.check ~seed:1L sampler_fail (opts ~n:3 ~ci:Ci_on ()) (fun _ -> true)
  in
  check_b "redacted sampler failure names the constructor" true
    (contains (Option.get v.message) "Failure");
  (* the case counter and iteration number *)
  let v = Runner.check ~seed:42L int_gen (opts ()) (fun x -> x <= 100) in
  check_b "iteration recorded" true
    (v.iteration >= 1 && v.cases_run = v.iteration);
  ignore (Types.validate_options Types.default_options);
  check_b "validate accepts defaults" true
    (Types.default_options.num_tests = 100)

type _ Effect.t += Suspend : unit Effect.t

(* Interleaved scopes in one domain (here: effect fibres) must not poison later checks. *)
let test_interleaved_scopes () =
  let k = ref None in
  let a = ref None in
  Effect.Deep.match_with
    (fun () ->
      Budget.with_root cfg ~max_attempts:5 (fun r ->
          a := Some r;
          Effect.perform Suspend))
    ()
    {
      Effect.Deep.retc = (fun () -> ());
      exnc = raise;
      effc =
        (fun (type a) (e : a Effect.t) ->
          match e with
          | Suspend ->
              Some
                (fun (c : (a, _) Effect.Deep.continuation) ->
                  k := Some (fun () -> Effect.Deep.continue c ()))
          | _ -> None);
    };
  (* scope A is open and ambient, suspended mid-flight; open B, then finish A first *)
  Budget.with_root cfg ~max_attempts:5 (fun _ ->
      match !k with
      | Some resume -> resume ()
      | None -> Alcotest.fail "fibre did not suspend");
  check_b "A closed out of order" true (Budget.is_closed (Option.get !a));
  check_b "no closed node is ambient" true (Budget.current () = None);
  let v = Runner.check ~seed:1L (Gen.int 0 9) (opts ~n:5 ()) (fun _ -> true) in
  check_b "a later plain check still passes" true v.passed;
  Budget.with_root cfg ~max_attempts:5 (fun r ->
      check_b "resolve skips nothing open" true
        (match Budget.resolve None with Some c -> c == r | None -> false))

let test_frequency_overflow () =
  let canon x = Val.Int (B.of_int x) in
  let c = Gen.constant ~canon 0 in
  let bad arms =
    try
      ignore (Gen.sample (Gen.frequency ~canon arms) (Prng.create 0L));
      false
    with Error.Argument_error _ -> true
  in
  check_b "max_int weights do not wrap past the cap" true
    (bad [ (max_int, c); (max_int, c); (3, c) ]);
  check_b "single huge weight" true (bad [ (max_int, c) ]);
  check_b "sum over the cap" true
    (bad [ (Types.max_frequency_total, c); (1, c) ]);
  check_b "sum at the cap is fine" false
    (bad [ (Types.max_frequency_total - 1, c); (1, c) ])

let test_finally_raised () =
  Alcotest.check_raises "Sys.Break in cleanup is not a panic" Sys.Break
    (fun () ->
      ignore
        (Safe.recoverable (fun () ->
             Fun.protect ~finally:(fun () -> raise Sys.Break) (fun () -> ()))));
  Alcotest.check_raises "Out_of_memory in cleanup is not a panic" Out_of_memory
    (fun () ->
      ignore
        (Safe.recoverable (fun () ->
             Fun.protect ~finally:(fun () -> raise Out_of_memory) (fun () -> ()))));
  check_b "other cleanup failures are values" true
    (Result.is_error
       (Safe.recoverable (fun () ->
            Fun.protect ~finally:(fun () -> raise Exit) (fun () -> ()))));
  (* tfv duplicate detection stays correct (and linear) on wide records and maps *)
  let keys = List.init 5000 (fun i -> Printf.sprintf "I%d I1" i) in
  check_b "wide map accepted" true
    (Tfv.valid ("tfv 1\nr R1 Km M5000 " ^ String.concat " " keys ^ "\n"));
  check_b "wide map with a late duplicate rejected" false
    (Tfv.valid
       ("tfv 1\nr R1 Km M5001 " ^ String.concat " " (keys @ [ "I0 I2" ]) ^ "\n"))

let () =
  Alcotest.run "test-bootstrap (VER05q)"
    [
      ( "bigint",
        [
          tc "arithmetic vectors" test_bigint_vectors;
          tc "misc" test_bigint_misc;
          tc "parse errors" test_bigint_parse_errors;
          tc "division identities" test_bigint_identities;
        ] );
      ( "prng",
        [
          tc "basics and hashes" test_prng_basics;
          tc "bounded vectors (independent reference)" test_bounded_vectors;
          tc "bounded_big vectors" test_bounded_big_vectors;
          tc "pinned values (rejection, 2^63+1, bounded(1))" test_pins;
          tc "argument errors" test_bounded_errors;
          tc "bounded_big thirds" test_bounded_big_thirds;
        ] );
      ( "shrink",
        [
          tc "integer pins" test_shrink_int_pins;
          tc "negative bisection floors" test_shrink_floor;
          tc "monotone thresholds" test_shrink_monotone_thresholds;
          tc "list pins and evaluation order" test_shrink_list_pins;
          tc "list 1-minimal" test_shrink_list_one_minimal;
          tc "bytes" test_shrink_bytes_pins;
          tc "replay" test_simulate_replay;
          tc "argument errors" test_shrink_errors;
        ] );
      ( "gen",
        [
          tc "primitives" test_gen_basics;
          tc "combinators" test_gen_combinators;
          tc "invalid generators fail closed" test_gen_invalid;
          tc "integrated shrinkers" test_gen_shrinkers;
          tc "corpus entry caps" test_corpus_gen_caps;
        ] );
      ( "runner",
        [
          tc "verdicts" test_runner_verdicts;
          tc "CI redaction" test_runner_redaction;
          tc "witness re-verification" test_runner_reverification;
          tc "determinism" test_runner_determinism;
          tc "types" test_types;
        ] );
      ( "hazards",
        [
          tc "exceptions classified by constructor" test_exceptions;
          tc "Sys.Break and Out_of_memory propagate"
            test_break_and_oom_propagate;
          tc "effects" test_effects;
          tc "user values" test_user_values;
          tc "canonical rendering" test_canon_render;
          tc "same_val" test_same_val;
        ] );
      ( "budget",
        [
          tc "all-or-nothing charges" test_budget_charges;
          tc "huge costs" test_budget_huge_cost;
          tc "scope release" test_budget_scope_release;
          tc "depth cap joins" test_budget_depth_cap;
          tc "ambient carrier" test_budget_ambient;
          tc "nested checks" test_nested_checks;
          tc "check nesting cap" test_check_nesting_cap;
          tc "filter retries" test_filter_budget;
          tc "recursing shrinker" test_recursing_shrinker;
          tc "refused re-verification keeps the witness"
            test_refused_reverification;
          tc "concurrent charging" test_concurrent_charging;
          tc "domains" test_domain_inheritance;
        ] );
      ( "security fixes",
        [
          tc "interleaved scopes do not poison the domain"
            test_interleaved_scopes;
          tc "frequency weight overflow" test_frequency_overflow;
          tc "Finally_raised unwrapping and wide tfv" test_finally_raised;
        ] );
      ( "edges",
        [
          tc "val_work" test_val_work;
          tc "top-level starvation" test_top_level_starvation;
          tc "spent scopes" test_spent_scopes;
          tc "generator canon and edges" test_gen_canon_and_edges;
          tc "synthetic corpus cases" test_corpus_synthetic;
          tc "runner misc" test_runner_misc;
        ] );
      ( "json and tfv",
        [
          tc "json accepts" test_json_accepts;
          tc "json rejects" test_json_rejects;
          tc "tfv" test_tfv;
          tc "value decoding" test_values_decode;
        ] );
      ( "corpus",
        [
          tc "replay (pbt-v1 25/25)" test_corpus_replay;
          tc "negative controls" test_corpus_negative_controls;
          tc "parse" test_corpus_parse;
          tc "sabotage" test_corpus_sabotage;
          tc "threshold 0 blind spot" test_threshold_blind_spot;
        ] );
    ]
