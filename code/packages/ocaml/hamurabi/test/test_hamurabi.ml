open Coding_adventures_hamurabi

let hamurabi_error_testable =
  Alcotest.testable
    (fun ppf -> function
      | GameAlreadyEnded -> Format.fprintf ppf "GameAlreadyEnded"
      | InvalidOrderBounds -> Format.fprintf ppf "InvalidOrderBounds"
      | InsufficientBushelsForLand ->
          Format.fprintf ppf "InsufficientBushelsForLand"
      | InsufficientAcresToSell -> Format.fprintf ppf "InsufficientAcresToSell"
      | InsufficientBushelsToFeed ->
          Format.fprintf ppf "InsufficientBushelsToFeed"
      | InsufficientAcresToPlant ->
          Format.fprintf ppf "InsufficientAcresToPlant"
      | InsufficientPopulationToPlant ->
          Format.fprintf ppf "InsufficientPopulationToPlant"
      | InsufficientGrainForSeed ->
          Format.fprintf ppf "InsufficientGrainForSeed")
    ( = )

let hamurabi_state_testable =
  Alcotest.testable
    (fun ppf s ->
      Format.fprintf ppf
        "{ year=%d; pop=%d; bus=%d; acr=%d; lp=%d; hpa=%d; rats=%d; \
         starved=%d; st_tot=%d; pss=%d; imm=%d; plague=%b; rng_state=%Lu; \
         rng_inc=%Lu }"
        s.year s.population s.bushels s.acres s.land_price s.harvest_per_acre
        s.rats_eaten s.starved_last_year s.starved_total s.percent_starved_sum
        s.immigrants_last_year s.plague_struck s.rng_state s.rng_inc)
    ( = )

let test_new_game () =
  let s = new_game 42L in
  Alcotest.(check int) "year" 1 s.year;
  Alcotest.(check int) "population" 100 s.population;
  Alcotest.(check int) "bushels" 2800 s.bushels;
  Alcotest.(check int) "acres" 1000 s.acres;
  Alcotest.(check int) "harvest_per_acre" 3 s.harvest_per_acre;
  Alcotest.(check int) "rats_eaten" 200 s.rats_eaten;
  Alcotest.(check int) "starved_last_year" 0 s.starved_last_year;
  Alcotest.(check int) "starved_total" 0 s.starved_total;
  Alcotest.(check int) "percent_starved_sum" 0 s.percent_starved_sum;
  Alcotest.(check int) "immigrants_last_year" 5 s.immigrants_last_year;
  Alcotest.(check bool) "plague_struck" false s.plague_struck;
  Alcotest.(check bool)
    "land_price in range" true
    (s.land_price >= 17 && s.land_price <= 26);
  Alcotest.(check int64) "rng_inc odd" 1L (Int64.logand s.rng_inc 1L)

let test_validate_order () =
  let s = new_game 42L in
  let valid_order =
    { acres_delta = 0; bushels_to_feed = 2000; acres_to_plant = 500 }
  in
  Alcotest.(check (result unit hamurabi_error_testable))
    "valid order" (Ok ())
    (validate_order s valid_order);

  let s_won = { s with status = Won Charlemagne } in
  Alcotest.(check (result unit hamurabi_error_testable))
    "game won" (Error GameAlreadyEnded)
    (validate_order s_won valid_order);

  let s_even_inc = { s with rng_inc = 108L } in
  Alcotest.(check (result unit hamurabi_error_testable))
    "even inc" (Error InvalidOrderBounds)
    (validate_order s_even_inc valid_order);

  let buy_too_much =
    { acres_delta = 200; bushels_to_feed = 0; acres_to_plant = 0 }
  in
  Alcotest.(check (result unit hamurabi_error_testable))
    "insufficient bushels for land"
    (Error InsufficientBushelsForLand)
    (validate_order s buy_too_much);

  let sell_too_much =
    { acres_delta = -1001; bushels_to_feed = 0; acres_to_plant = 0 }
  in
  Alcotest.(check (result unit hamurabi_error_testable))
    "insufficient acres to sell"
    (Error InsufficientAcresToSell)
    (validate_order s sell_too_much);

  let feed_too_much =
    { acres_delta = 0; bushels_to_feed = 2801; acres_to_plant = 0 }
  in
  Alcotest.(check (result unit hamurabi_error_testable))
    "insufficient bushels to feed"
    (Error InsufficientBushelsToFeed)
    (validate_order s feed_too_much);

  let plant_too_much_acres =
    { acres_delta = 0; bushels_to_feed = 0; acres_to_plant = 1001 }
  in
  Alcotest.(check (result unit hamurabi_error_testable))
    "insufficient acres to plant"
    (Error InsufficientAcresToPlant)
    (validate_order s plant_too_much_acres);

  let plant_too_much_pop =
    { acres_delta = 0; bushels_to_feed = 0; acres_to_plant = 200 }
  in
  Alcotest.(check (result unit hamurabi_error_testable))
    "insufficient population to plant"
    (Error InsufficientPopulationToPlant)
    (validate_order { s with population = 10 } plant_too_much_pop);

  let plant_no_grain =
    { acres_delta = 0; bushels_to_feed = 2800; acres_to_plant = 100 }
  in
  Alcotest.(check (result unit hamurabi_error_testable))
    "insufficient grain for seed"
    (Error InsufficientGrainForSeed)
    (validate_order s plant_no_grain)

let test_format_report () =
  let s = { (new_game 42L) with status = GameOverExtinction } in
  Alcotest.(check string)
    "extinction teletype"
    "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n"
    (format_report s)

(* ── Conformance Corpus Parser & Test ────────────────────────────────────── *)

type turn_check = {
  turn_num : int;
  turn_order : turn_order;
  expected_state : hamurabi_state;
  expected_report : string;
}

type refusal_check = {
  refusal_order : turn_order;
  expected_error : hamurabi_error;
}

type test_case = {
  case_id : string;
  case_seed : int64 option;
  init_state : hamurabi_state;
  turns : turn_check list;
  refusal : refusal_check option;
}

let find_corpus () =
  let rec climb dir =
    let candidate =
      Filename.concat dir "code/specs/fixtures/hamurabi-v1/cases.txt"
    in
    if Sys.file_exists candidate then candidate
    else
      let parent = Filename.dirname dir in
      if parent = dir then failwith ("Could not find cases.txt; started at " ^ Sys.getcwd ())
      else climb parent
  in
  let path = climb (Sys.getcwd ()) in
  In_channel.with_open_bin path In_channel.input_all

let unescape_report s =
  let b = Buffer.create (String.length s) in
  let i = ref 0 in
  let len = String.length s in
  while !i < len do
    if !i + 1 < len && s.[!i] = '\\' && s.[!i + 1] = 'n' then (
      Buffer.add_char b '\n';
      i := !i + 2)
    else (
      Buffer.add_char b s.[!i];
      incr i)
  done;
  Buffer.contents b

let parse_status s =
  if s = "Playing" then Playing
  else if s = "GameOverExtinction" then GameOverExtinction
  else if String.starts_with ~prefix:"Won(" s then
    let inner = String.sub s 4 (String.length s - 5) in
    match inner with
    | "Charlemagne" -> Won Charlemagne
    | "Acre" -> Won Acre
    | "Nero" -> Won Nero
    | "Disaster" -> Won Disaster
    | other -> failwith ("Unknown rating: " ^ other)
  else if String.starts_with ~prefix:"Impeached(" s then
    Scanf.sscanf s "Impeached(starved=%d, percent=%d)" (fun starved percent ->
        Impeached { starved_this_turn = starved; percent })
  else failwith ("Unknown status: " ^ s)

let parse_u64 s = Scanf.sscanf s "%Lu" Fun.id

let parse_error = function
  | "GameAlreadyEnded" -> GameAlreadyEnded
  | "InvalidOrderBounds" -> InvalidOrderBounds
  | "InsufficientBushelsForLand" -> InsufficientBushelsForLand
  | "InsufficientAcresToSell" -> InsufficientAcresToSell
  | "InsufficientBushelsToFeed" -> InsufficientBushelsToFeed
  | "InsufficientAcresToPlant" -> InsufficientAcresToPlant
  | "InsufficientPopulationToPlant" -> InsufficientPopulationToPlant
  | "InsufficientGrainForSeed" -> InsufficientGrainForSeed
  | other -> failwith ("Unknown error: " ^ other)

let split_words s =
  List.filter (fun w -> String.length w > 0) (String.split_on_char ' ' s)

let parse_state tokens =
  let yr = int_of_string (List.nth tokens 1) in
  let pop = int_of_string (List.nth tokens 2) in
  let bus = int_of_string (List.nth tokens 3) in
  let acr = int_of_string (List.nth tokens 4) in
  let lp = int_of_string (List.nth tokens 5) in
  let hpa = int_of_string (List.nth tokens 6) in
  let re = int_of_string (List.nth tokens 7) in
  let sly = int_of_string (List.nth tokens 8) in
  let st = int_of_string (List.nth tokens 9) in
  let pss = int_of_string (List.nth tokens 10) in
  let ily = int_of_string (List.nth tokens 11) in
  let ps = int_of_string (List.nth tokens 12) <> 0 in
  let arr = Array.of_list tokens in
  let len = Array.length arr in
  let rng_inc_val = parse_u64 arr.(len - 1) in
  let rng_state_val = parse_u64 arr.(len - 2) in
  let status_words = Array.sub arr 13 (len - 2 - 13) in
  let status_str = String.concat " " (Array.to_list status_words) in
  let status_val = parse_status status_str in
  {
    year = yr;
    population = pop;
    bushels = bus;
    acres = acr;
    land_price = lp;
    harvest_per_acre = hpa;
    rats_eaten = re;
    starved_last_year = sly;
    starved_total = st;
    percent_starved_sum = pss;
    immigrants_last_year = ily;
    plague_struck = ps;
    status = status_val;
    rng_state = rng_state_val;
    rng_inc = rng_inc_val;
  }

let clean_line l =
  let len = String.length l in
  if len > 0 && l.[len - 1] = '\r' then String.sub l 0 (len - 1) else l

let parse_corpus raw =
  let all_lines = List.map clean_line (String.split_on_char '\n' raw) in
  let filtered =
    List.filter
      (fun l ->
        let trimmed = String.trim l in
        String.length trimmed > 0
        && not (String.starts_with ~prefix:"#" trimmed)
        && not (String.starts_with ~prefix:"COUNT " trimmed))
      all_lines
  in
  let rec parse_cases acc = function
    | [] -> List.rev acc
    | line :: rest when String.starts_with ~prefix:"CASE " line ->
        let ws = split_words line in
        let c_id = List.nth ws 1 in
        let seed_str = List.nth ws 2 in
        let mb_seed =
          if seed_str = "null" then None else Some (parse_u64 seed_str)
        in
        let rec collect_case_lines case_acc = function
          | [] -> (List.rev case_acc, [])
          | next_line :: next_rest
            when String.starts_with ~prefix:"CASE " next_line ->
              (List.rev case_acc, next_line :: next_rest)
          | other :: next_rest -> collect_case_lines (other :: case_acc) next_rest
        in
        let case_lines, remaining = collect_case_lines [] rest in
        let tc = build_case c_id mb_seed case_lines in
        parse_cases (tc :: acc) remaining
    | _ :: rest -> parse_cases acc rest
  and build_case c_id mb_seed lines =
    let rec loop mb_init rev_turns mb_pending mb_ref = function
      | [] -> (
          match mb_init with
          | None -> failwith ("Case " ^ c_id ^ " missing INIT")
          | Some ini ->
              {
                case_id = c_id;
                case_seed = mb_seed;
                init_state = ini;
                turns = List.rev rev_turns;
                refusal = mb_ref;
              })
      | line :: rest when String.starts_with ~prefix:"INIT " line ->
          let st = parse_state (split_words line) in
          loop (Some st) rev_turns mb_pending mb_ref rest
      | line :: rest when String.starts_with ~prefix:"TURN " line ->
          let ws = split_words line in
          let t_num = int_of_string (List.nth ws 1) in
          let d = int_of_string (List.nth ws 2) in
          let f = int_of_string (List.nth ws 3) in
          let p = int_of_string (List.nth ws 4) in
          let ord =
            { acres_delta = d; bushels_to_feed = f; acres_to_plant = p }
          in
          loop mb_init rev_turns (Some (t_num, ord)) mb_ref rest
      | line :: rest when String.starts_with ~prefix:"STATE " line ->
          let st = parse_state (split_words line) in
          let t_num, ord =
            match mb_pending with
            | Some p -> p
            | None -> failwith ("STATE without TURN in " ^ c_id)
          in
          let rep_line, after_rep =
            match rest with
            | r_line :: after when String.starts_with ~prefix:"REPORT " r_line ->
                (r_line, after)
            | _ -> failwith ("Expected REPORT after STATE in " ^ c_id)
          in
          let rep =
            unescape_report (String.sub rep_line 7 (String.length rep_line - 7))
          in
          let tc =
            {
              turn_num = t_num;
              turn_order = ord;
              expected_state = st;
              expected_report = rep;
            }
          in
          loop mb_init (tc :: rev_turns) None mb_ref after_rep
      | line :: rest when String.starts_with ~prefix:"REFUSAL " line ->
          let ws = split_words line in
          let d = int_of_string (List.nth ws 1) in
          let f = int_of_string (List.nth ws 2) in
          let p = int_of_string (List.nth ws 3) in
          let err = parse_error (List.nth ws 4) in
          let ord =
            { acres_delta = d; bushels_to_feed = f; acres_to_plant = p }
          in
          let ref_check = { refusal_order = ord; expected_error = err } in
          loop mb_init rev_turns mb_pending (Some ref_check) rest
      | _ :: rest -> loop mb_init rev_turns mb_pending mb_ref rest
    in
    loop None [] None None lines
  in
  parse_cases [] filtered

let test_conformance_cases () =
  let corpus_text = find_corpus () in
  let cases = parse_corpus corpus_text in
  Alcotest.(check int) "count 31 cases" 31 (List.length cases);
  List.iter
    (fun tc ->
      (match tc.case_seed with
      | Some seed when String.starts_with ~prefix:"full_" tc.case_id ->
          let initial = new_game seed in
          Alcotest.(check hamurabi_state_testable)
            (tc.case_id ^ " new_game init")
            tc.init_state initial
      | _ -> ());

      let cur = ref tc.init_state in
      List.iter
        (fun turn ->
          match step_year !cur turn.turn_order with
          | Error _ ->
              Alcotest.fail
                (Printf.sprintf "%s turn %d failed: unexpected error" tc.case_id
                   turn.turn_num)
          | Ok next_st ->
              Alcotest.(check hamurabi_state_testable)
                (Printf.sprintf "%s turn %d state" tc.case_id turn.turn_num)
                turn.expected_state next_st;
              Alcotest.(check string)
                (Printf.sprintf "%s turn %d report" tc.case_id turn.turn_num)
                turn.expected_report (format_report next_st);
              cur := next_st)
        tc.turns;

      match tc.refusal with
      | None -> ()
      | Some ref_check -> (
          Alcotest.(check (result unit hamurabi_error_testable))
            (tc.case_id ^ " validate_order refusal")
            (Error ref_check.expected_error)
            (validate_order tc.init_state ref_check.refusal_order);
          match step_year tc.init_state ref_check.refusal_order with
          | Error err ->
              Alcotest.(check hamurabi_error_testable)
                (tc.case_id ^ " step_year refusal")
                ref_check.expected_error err
          | Ok _ ->
              Alcotest.fail (tc.case_id ^ " expected refusal but succeeded")))
    cases

let () =
  Alcotest.run "hamurabi"
    [
      ( "unit",
        [
          Alcotest.test_case "new_game baseline" `Quick test_new_game;
          Alcotest.test_case "validate_order constraints" `Quick
            test_validate_order;
          Alcotest.test_case "format_report extinction" `Quick
            test_format_report;
        ] );
      ( "conformance",
        [
          Alcotest.test_case "cases.txt 31 cases bit-for-bit" `Quick
            test_conformance_cases;
        ] );
    ]
