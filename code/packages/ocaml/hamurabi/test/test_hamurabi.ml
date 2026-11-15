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
    "insufficient bushels for land" (Error InsufficientBushelsForLand)
    (validate_order s buy_too_much);

  let sell_too_much =
    { acres_delta = -1001; bushels_to_feed = 0; acres_to_plant = 0 }
  in
  Alcotest.(check (result unit hamurabi_error_testable))
    "insufficient acres to sell" (Error InsufficientAcresToSell)
    (validate_order s sell_too_much);

  let feed_too_much =
    { acres_delta = 0; bushels_to_feed = 2801; acres_to_plant = 0 }
  in
  Alcotest.(check (result unit hamurabi_error_testable))
    "insufficient bushels to feed" (Error InsufficientBushelsToFeed)
    (validate_order s feed_too_much);

  let plant_too_much_acres =
    { acres_delta = 0; bushels_to_feed = 0; acres_to_plant = 1001 }
  in
  Alcotest.(check (result unit hamurabi_error_testable))
    "insufficient acres to plant" (Error InsufficientAcresToPlant)
    (validate_order s plant_too_much_acres);

  let plant_too_much_pop =
    { acres_delta = 0; bushels_to_feed = 0; acres_to_plant = 200 }
  in
  Alcotest.(check (result unit hamurabi_error_testable))
    "insufficient population to plant" (Error InsufficientPopulationToPlant)
    (validate_order { s with population = 10 } plant_too_much_pop);

  let plant_no_grain =
    { acres_delta = 0; bushels_to_feed = 2800; acres_to_plant = 100 }
  in
  Alcotest.(check (result unit hamurabi_error_testable))
    "insufficient grain for seed" (Error InsufficientGrainForSeed)
    (validate_order s plant_no_grain)

let test_format_report () =
  let s = { (new_game 42L) with status = GameOverExtinction } in
  Alcotest.(check string)
    "extinction teletype" "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n"
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
  {|# hamurabi-v1 flat conformance corpus (generated from cases.json)
# CASE <id> <seed_or_null> <num_turns> <has_refusal:0|1>
# INIT <year> <population> <bushels> <acres> <land_price> <harvest_per_acre> <rats_eaten> <starved_last_year> <starved_total> <percent_starved_sum> <immigrants_last_year> <plague_struck:0|1> <status> <rng_state> <rng_inc>
# TURN <turn> <acres_delta> <bushels_to_feed> <acres_to_plant>
# STATE <year> ... <rng_inc>
# REPORT <escaped_report>
# REFUSAL <acres_delta> <bushels_to_feed> <acres_to_plant> <expected_error>
COUNT 31
CASE full_10_year_charlemagne_win 2 10 0
INIT 1 100 2800 1000 21 3 200 0 0 0 5 0 Playing 15877481753564384925 109
TURN 1 0 2000 1000
STATE 2 103 4300 1000 23 4 0 0 0 0 3 0 Playing 13871520052386028674 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 2, 0 PEOPLE STARVED, 3 CAME TO THE CITY.\nPOPULATION IS NOW 103.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 4300 BUSHELS IN STORE.\nLAND IS TRADING AT 23 BUSHELS PER ACRE.\n
TURN 2 0 2060 1000
STATE 3 108 3740 1000 22 2 0 0 0 0 5 0 Playing 3339867228080909011 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 3, 0 PEOPLE STARVED, 5 CAME TO THE CITY.\nPOPULATION IS NOW 108.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 2 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 3740 BUSHELS IN STORE.\nLAND IS TRADING AT 22 BUSHELS PER ACRE.\n
TURN 3 0 2160 1000
STATE 4 115 2080 1000 22 1 0 0 0 0 7 0 Playing 5320077674710833120 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 4, 0 PEOPLE STARVED, 7 CAME TO THE CITY.\nPOPULATION IS NOW 115.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 1 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 2080 BUSHELS IN STORE.\nLAND IS TRADING AT 22 BUSHELS PER ACRE.\n
TURN 4 -38 2300 962
STATE 5 124 3983 962 22 4 0 0 0 0 9 0 Playing 2280505260147886265 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 5, 0 PEOPLE STARVED, 9 CAME TO THE CITY.\nPOPULATION IS NOW 124.\nTHE CITY NOW OWNS 962 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 3983 BUSHELS IN STORE.\nLAND IS TRADING AT 22 BUSHELS PER ACRE.\n
TURN 5 0 2480 962
STATE 6 133 1984 962 18 1 0 0 0 0 9 0 Playing 11351614347405479470 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 6, 0 PEOPLE STARVED, 9 CAME TO THE CITY.\nPOPULATION IS NOW 133.\nTHE CITY NOW OWNS 962 ACRES.\nYOU HARVESTED 1 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 1984 BUSHELS IN STORE.\nLAND IS TRADING AT 18 BUSHELS PER ACRE.\n
TURN 6 -71 2660 891
STATE 7 138 2829 891 21 3 0 0 0 0 5 0 Playing 13201901420416464335 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 7, 0 PEOPLE STARVED, 5 CAME TO THE CITY.\nPOPULATION IS NOW 138.\nTHE CITY NOW OWNS 891 ACRES.\nYOU HARVESTED 3 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 2829 BUSHELS IN STORE.\nLAND IS TRADING AT 21 BUSHELS PER ACRE.\n
TURN 7 -26 2760 865
STATE 8 76 1047 865 25 1 0 0 0 0 7 1 Playing 13917569415902942956 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 8, 0 PEOPLE STARVED, 7 CAME TO THE CITY.\nA HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\nPOPULATION IS NOW 76.\nTHE CITY NOW OWNS 865 ACRES.\nYOU HARVESTED 1 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 1047 BUSHELS IN STORE.\nLAND IS TRADING AT 25 BUSHELS PER ACRE.\n
TURN 8 -43 1520 760
STATE 9 81 871 822 19 2 871 0 0 0 5 0 Playing 18033718494871491742 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 9, 0 PEOPLE STARVED, 5 CAME TO THE CITY.\nPOPULATION IS NOW 81.\nTHE CITY NOW OWNS 822 ACRES.\nYOU HARVESTED 2 BUSHELS PER ACRE.\nRATS ATE 871 BUSHELS.\nYOU NOW HAVE 871 BUSHELS IN STORE.\nLAND IS TRADING AT 19 BUSHELS PER ACRE.\n
TURN 9 -71 1620 751
STATE 10 93 3979 751 24 5 0 0 0 0 12 0 Playing 16206033418651411071 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 10, 0 PEOPLE STARVED, 12 CAME TO THE CITY.\nPOPULATION IS NOW 93.\nTHE CITY NOW OWNS 751 ACRES.\nYOU HARVESTED 5 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 3979 BUSHELS IN STORE.\nLAND IS TRADING AT 24 BUSHELS PER ACRE.\n
TURN 10 0 1860 751
STATE 11 55 4747 751 17 4 0 0 0 0 9 1 Won(Charlemagne) 17605674360281485532 109
REPORT IN YOUR 10-YEAR TERM OF OFFICE, 0 PERCENT OF THE\nPOPULATION STARVED PER YEAR ON AVERAGE.\nA TOTAL OF 0 PEOPLE DIED!!\nYOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n13 ACRES PER PERSON.\n\nA FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!\n
CASE full_10_year_acre_win 7 10 0
INIT 1 100 2800 1000 20 3 200 0 0 0 5 0 Playing 16588482227696956458 109
TURN 1 0 2000 1000
STATE 2 111 0 1000 23 4 4300 0 0 0 11 0 Playing 1795873687129158380 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 2, 0 PEOPLE STARVED, 11 CAME TO THE CITY.\nPOPULATION IS NOW 111.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 4300 BUSHELS.\nYOU NOW HAVE 0 BUSHELS IN STORE.\nLAND IS TRADING AT 23 BUSHELS PER ACRE.\n
TURN 2 -115 2040 885
STATE 3 102 2962 885 21 4 740 9 9 8 0 0 Playing 8298780650498936990 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 3, 9 PEOPLE STARVED, 0 CAME TO THE CITY.\nPOPULATION IS NOW 102.\nTHE CITY NOW OWNS 885 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 740 BUSHELS.\nYOU NOW HAVE 2962 BUSHELS IN STORE.\nLAND IS TRADING AT 21 BUSHELS PER ACRE.\n
TURN 3 0 2040 885
STATE 4 104 0 885 23 5 4904 0 9 8 2 0 Playing 3893152778318933440 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 4, 0 PEOPLE STARVED, 2 CAME TO THE CITY.\nPOPULATION IS NOW 104.\nTHE CITY NOW OWNS 885 ACRES.\nYOU HARVESTED 5 BUSHELS PER ACRE.\nRATS ATE 4904 BUSHELS.\nYOU NOW HAVE 0 BUSHELS IN STORE.\nLAND IS TRADING AT 23 BUSHELS PER ACRE.\n
TURN 4 -117 2080 768
STATE 5 108 4067 768 23 5 0 0 9 8 4 0 Playing 9993893548820809497 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 5, 0 PEOPLE STARVED, 4 CAME TO THE CITY.\nPOPULATION IS NOW 108.\nTHE CITY NOW OWNS 768 ACRES.\nYOU HARVESTED 5 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 4067 BUSHELS IN STORE.\nLAND IS TRADING AT 23 BUSHELS PER ACRE.\n
TURN 5 0 2160 768
STATE 6 118 4595 768 26 4 0 0 9 8 10 0 Playing 8988741222041375502 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 6, 0 PEOPLE STARVED, 10 CAME TO THE CITY.\nPOPULATION IS NOW 118.\nTHE CITY NOW OWNS 768 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 4595 BUSHELS IN STORE.\nLAND IS TRADING AT 26 BUSHELS PER ACRE.\n
TURN 6 0 2360 768
STATE 7 120 4923 768 26 4 0 0 9 8 2 0 Playing 1958873358450943791 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 7, 0 PEOPLE STARVED, 2 CAME TO THE CITY.\nPOPULATION IS NOW 120.\nTHE CITY NOW OWNS 768 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 4923 BUSHELS IN STORE.\nLAND IS TRADING AT 26 BUSHELS PER ACRE.\n
TURN 7 0 2400 768
STATE 8 64 4443 768 22 3 0 0 9 8 4 1 Playing 9287407350296183500 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 8, 0 PEOPLE STARVED, 4 CAME TO THE CITY.\nA HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\nPOPULATION IS NOW 64.\nTHE CITY NOW OWNS 768 ACRES.\nYOU HARVESTED 3 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 4443 BUSHELS IN STORE.\nLAND IS TRADING AT 22 BUSHELS PER ACRE.\n
TURN 8 0 1280 640
STATE 9 67 3573 768 24 3 1190 0 9 8 3 0 Playing 10288967688569983358 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 9, 0 PEOPLE STARVED, 3 CAME TO THE CITY.\nPOPULATION IS NOW 67.\nTHE CITY NOW OWNS 768 ACRES.\nYOU HARVESTED 3 BUSHELS PER ACRE.\nRATS ATE 1190 BUSHELS.\nYOU NOW HAVE 3573 BUSHELS IN STORE.\nLAND IS TRADING AT 24 BUSHELS PER ACRE.\n
TURN 9 0 1340 670
STATE 10 78 2159 768 24 2 1079 0 9 8 11 0 Playing 9694028222408382368 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 10, 0 PEOPLE STARVED, 11 CAME TO THE CITY.\nPOPULATION IS NOW 78.\nTHE CITY NOW OWNS 768 ACRES.\nYOU HARVESTED 2 BUSHELS PER ACRE.\nRATS ATE 1079 BUSHELS.\nYOU NOW HAVE 2159 BUSHELS IN STORE.\nLAND IS TRADING AT 24 BUSHELS PER ACRE.\n
TURN 10 0 1560 768
STATE 11 85 1751 768 19 2 0 0 9 8 7 0 Won(Acre) 6517780273347070329 109
REPORT IN YOUR 10-YEAR TERM OF OFFICE, 0 PERCENT OF THE\nPOPULATION STARVED PER YEAR ON AVERAGE.\nA TOTAL OF 9 PEOPLE DIED!!\nYOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n9 ACRES PER PERSON.\n\nYOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE.\n
CASE full_10_year_nero_rating 1 10 0
INIT 1 100 2800 1000 24 3 200 0 0 0 5 0 Playing 8356584029254049972 109
TURN 1 0 1600 1000
STATE 2 80 1700 1000 24 1 0 20 20 20 0 0 Playing 5112544598722727485 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 2, 20 PEOPLE STARVED, 0 CAME TO THE CITY.\nPOPULATION IS NOW 80.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 1 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 1700 BUSHELS IN STORE.\nLAND IS TRADING AT 24 BUSHELS PER ACRE.\n
TURN 2 0 1280 800
STATE 3 64 1620 1000 25 2 0 16 36 40 0 0 Playing 193508876360195490 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 3, 16 PEOPLE STARVED, 0 CAME TO THE CITY.\nPOPULATION IS NOW 64.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 2 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 1620 BUSHELS IN STORE.\nLAND IS TRADING AT 25 BUSHELS PER ACRE.\n
TURN 3 0 1020 640
STATE 4 51 614 1000 26 1 306 13 49 60 0 0 Playing 3397986998850698404 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 4, 13 PEOPLE STARVED, 0 CAME TO THE CITY.\nPOPULATION IS NOW 51.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 1 BUSHELS PER ACRE.\nRATS ATE 306 BUSHELS.\nYOU NOW HAVE 614 BUSHELS IN STORE.\nLAND IS TRADING AT 26 BUSHELS PER ACRE.\n
TURN 4 -31 800 510
STATE 5 40 1895 969 22 3 0 11 60 81 0 0 Playing 7671814514939476589 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 5, 11 PEOPLE STARVED, 0 CAME TO THE CITY.\nPOPULATION IS NOW 40.\nTHE CITY NOW OWNS 969 ACRES.\nYOU HARVESTED 3 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 1895 BUSHELS IN STORE.\nLAND IS TRADING AT 22 BUSHELS PER ACRE.\n
TURN 5 0 640 400
STATE 6 16 2255 969 22 3 0 8 68 101 0 1 Playing 274706909115308306 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 6, 8 PEOPLE STARVED, 0 CAME TO THE CITY.\nA HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\nPOPULATION IS NOW 16.\nTHE CITY NOW OWNS 969 ACRES.\nYOU HARVESTED 3 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 2255 BUSHELS IN STORE.\nLAND IS TRADING AT 22 BUSHELS PER ACRE.\n
TURN 6 0 240 160
STATE 7 12 2735 969 22 5 0 4 72 126 0 0 Playing 4204696158712313123 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 7, 4 PEOPLE STARVED, 0 CAME TO THE CITY.\nPOPULATION IS NOW 12.\nTHE CITY NOW OWNS 969 ACRES.\nYOU HARVESTED 5 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 2735 BUSHELS IN STORE.\nLAND IS TRADING AT 22 BUSHELS PER ACRE.\n
TURN 7 0 180 120
STATE 8 9 2615 969 17 1 0 3 75 151 0 0 Playing 14045966339183966448 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 8, 3 PEOPLE STARVED, 0 CAME TO THE CITY.\nPOPULATION IS NOW 9.\nTHE CITY NOW OWNS 969 ACRES.\nYOU HARVESTED 1 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 2615 BUSHELS IN STORE.\nLAND IS TRADING AT 17 BUSHELS PER ACRE.\n
TURN 8 0 140 90
STATE 9 7 2610 969 19 2 0 2 77 173 0 0 Playing 3691797459336799625 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 9, 2 PEOPLE STARVED, 0 CAME TO THE CITY.\nPOPULATION IS NOW 7.\nTHE CITY NOW OWNS 969 ACRES.\nYOU HARVESTED 2 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 2610 BUSHELS IN STORE.\nLAND IS TRADING AT 19 BUSHELS PER ACRE.\n
TURN 9 0 100 70
STATE 10 5 2615 969 19 2 0 2 79 201 0 0 Playing 15098015746358654910 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 10, 2 PEOPLE STARVED, 0 CAME TO THE CITY.\nPOPULATION IS NOW 5.\nTHE CITY NOW OWNS 969 ACRES.\nYOU HARVESTED 2 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 2615 BUSHELS IN STORE.\nLAND IS TRADING AT 19 BUSHELS PER ACRE.\n
TURN 10 0 80 50
STATE 11 4 2560 969 20 1 0 1 80 221 0 0 Won(Nero) 5223788572027599135 109
REPORT IN YOUR 10-YEAR TERM OF OFFICE, 22 PERCENT OF THE\nPOPULATION STARVED PER YEAR ON AVERAGE.\nA TOTAL OF 80 PEOPLE DIED!!\nYOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n242 ACRES PER PERSON.\n\nYOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU.\n
CASE full_10_year_disaster_rating 20 10 0
INIT 1 100 2800 1000 24 3 200 0 0 0 5 0 Playing 3679688201474001151 109
TURN 1 0 2000 1000
STATE 2 61 5300 1000 22 5 0 0 0 0 11 1 Playing 5512381554939485532 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 2, 0 PEOPLE STARVED, 11 CAME TO THE CITY.\nA HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\nPOPULATION IS NOW 61.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 5 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 5300 BUSHELS IN STORE.\nLAND IS TRADING AT 22 BUSHELS PER ACRE.\n
TURN 2 0 1220 610
STATE 3 50 3737 1000 18 3 1868 0 0 0 20 1 Playing 12184952730819395470 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 3, 0 PEOPLE STARVED, 20 CAME TO THE CITY.\nA HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\nPOPULATION IS NOW 50.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 3 BUSHELS PER ACRE.\nRATS ATE 1868 BUSHELS.\nYOU NOW HAVE 3737 BUSHELS IN STORE.\nLAND IS TRADING AT 18 BUSHELS PER ACRE.\n
TURN 3 0 1000 500
STATE 4 65 4487 1000 21 4 0 0 0 0 15 0 Playing 9879522357575279023 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 4, 0 PEOPLE STARVED, 15 CAME TO THE CITY.\nPOPULATION IS NOW 65.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 4487 BUSHELS IN STORE.\nLAND IS TRADING AT 21 BUSHELS PER ACRE.\n
TURN 4 0 1300 650
STATE 5 84 3642 1000 26 4 1820 0 0 0 19 0 Playing 9203739601053522121 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 5, 0 PEOPLE STARVED, 19 CAME TO THE CITY.\nPOPULATION IS NOW 84.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 1820 BUSHELS.\nYOU NOW HAVE 3642 BUSHELS IN STORE.\nLAND IS TRADING AT 26 BUSHELS PER ACRE.\n
TURN 5 0 1680 840
STATE 6 96 4902 1000 19 4 0 0 0 0 12 0 Playing 1494375716485881342 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 6, 0 PEOPLE STARVED, 12 CAME TO THE CITY.\nPOPULATION IS NOW 96.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 4902 BUSHELS IN STORE.\nLAND IS TRADING AT 19 BUSHELS PER ACRE.\n
TURN 6 0 1920 960
STATE 7 102 5477 1000 17 5 1825 0 0 0 6 0 Playing 6607783439341256736 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 7, 0 PEOPLE STARVED, 6 CAME TO THE CITY.\nPOPULATION IS NOW 102.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 5 BUSHELS PER ACRE.\nRATS ATE 1825 BUSHELS.\nYOU NOW HAVE 5477 BUSHELS IN STORE.\nLAND IS TRADING AT 17 BUSHELS PER ACRE.\n
TURN 7 0 2040 1000
STATE 8 59 4937 1000 19 2 0 0 0 0 8 1 Playing 15342559281546104825 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 8, 0 PEOPLE STARVED, 8 CAME TO THE CITY.\nA HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\nPOPULATION IS NOW 59.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 2 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 4937 BUSHELS IN STORE.\nLAND IS TRADING AT 19 BUSHELS PER ACRE.\n
TURN 8 0 1180 590
STATE 9 81 5822 1000 25 4 0 0 0 0 22 0 Playing 13266071477976627310 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 9, 0 PEOPLE STARVED, 22 CAME TO THE CITY.\nPOPULATION IS NOW 81.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 5822 BUSHELS IN STORE.\nLAND IS TRADING AT 25 BUSHELS PER ACRE.\n
TURN 9 0 1620 810
STATE 10 93 3519 1000 18 4 3518 0 0 0 12 0 Playing 5519659933592149776 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 10, 0 PEOPLE STARVED, 12 CAME TO THE CITY.\nPOPULATION IS NOW 93.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 3518 BUSHELS.\nYOU NOW HAVE 3519 BUSHELS IN STORE.\nLAND IS TRADING AT 18 BUSHELS PER ACRE.\n
TURN 10 -800 1860 200
STATE 11 49 16759 200 18 4 0 0 0 0 3 1 Won(Disaster) 14040873003973145385 109
REPORT IN YOUR 10-YEAR TERM OF OFFICE, 0 PERCENT OF THE\nPOPULATION STARVED PER YEAR ON AVERAGE.\nA TOTAL OF 0 PEOPLE DIED!!\nYOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n4 ACRES PER PERSON.\n\nYOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!\n
CASE turn_with_plague_struck 6 1 0
INIT 1 100 2800 1000 25 3 200 0 0 0 5 0 Playing 9067584503386621505 109
TURN 1 0 2000 500
STATE 2 59 2050 1000 19 3 0 0 0 0 9 1 Playing 12013933719620130198 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 2, 0 PEOPLE STARVED, 9 CAME TO THE CITY.\nA HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\nPOPULATION IS NOW 59.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 3 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 2050 BUSHELS IN STORE.\nLAND IS TRADING AT 19 BUSHELS PER ACRE.\n
CASE turn_with_rat_infestation 3 1 0
INIT 1 100 2800 1000 20 3 200 0 0 0 5 0 Playing 4951635404165168262 109
TURN 1 0 2000 500
STATE 2 105 0 1000 17 4 2550 0 0 0 5 0 Playing 4788681129421504104 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 2, 0 PEOPLE STARVED, 5 CAME TO THE CITY.\nPOPULATION IS NOW 105.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 2550 BUSHELS.\nYOU NOW HAVE 0 BUSHELS IN STORE.\nLAND IS TRADING AT 17 BUSHELS PER ACRE.\n
CASE turn_with_land_purchase 1337 1 0
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
TURN 1 50 1000 500
STATE 2 50 2450 1050 26 4 0 50 50 50 0 0 Impeached(starved=50, percent=50) 11868007961037391445 109
REPORT YOU STARVED 50 PEOPLE IN ONE YEAR!!!\nDUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n
CASE turn_with_land_sale 1337 1 0
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
TURN 1 -100 2000 800
STATE 2 108 5800 900 26 4 0 0 0 0 8 0 Playing 11868007961037391445 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 2, 0 PEOPLE STARVED, 8 CAME TO THE CITY.\nPOPULATION IS NOW 108.\nTHE CITY NOW OWNS 900 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 5800 BUSHELS IN STORE.\nLAND IS TRADING AT 26 BUSHELS PER ACRE.\n
CASE immediate_impeachment_100_percent_starved 42 1 0
INIT 1 100 2800 1000 20 3 200 0 0 0 5 0 Playing 3118741472915405573 109
TURN 1 0 0 0
STATE 2 0 0 1000 22 3 2800 100 100 100 0 0 Impeached(starved=100, percent=100) 17113982732917624431 109
REPORT YOU STARVED 100 PEOPLE IN ONE YEAR!!!\nDUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n
CASE borderline_impeachment_46_percent_starved 42 1 0
INIT 1 100 2800 1000 20 3 200 0 0 0 5 0 Playing 3118741472915405573 109
TURN 1 0 1080 500
STATE 2 54 0 1000 22 3 2970 46 46 46 0 0 Impeached(starved=46, percent=46) 17113982732917624431 109
REPORT YOU STARVED 46 PEOPLE IN ONE YEAR!!!\nDUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n
CASE borderline_non_impeachment_45_percent_starved 42 1 0
INIT 1 100 2800 1000 20 3 200 0 0 0 5 0 Playing 3118741472915405573 109
TURN 1 0 1100 500
STATE 2 55 0 1000 22 3 2950 45 45 45 0 0 Playing 17113982732917624431 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 2, 45 PEOPLE STARVED, 0 CAME TO THE CITY.\nPOPULATION IS NOW 55.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 3 BUSHELS PER ACRE.\nRATS ATE 2950 BUSHELS.\nYOU NOW HAVE 0 BUSHELS IN STORE.\nLAND IS TRADING AT 22 BUSHELS PER ACRE.\n
CASE refusal_game_already_ended_impeached 42 0 1
INIT 2 0 0 1000 22 3 2800 100 100 100 0 0 Impeached(starved=100, percent=100) 17113982732917624431 109
REFUSAL 0 2000 500 GameAlreadyEnded
CASE refusal_invalid_order_bounds_revenue_overflow 1337 0 1
INIT 1 100 2800 200000000 25 3 200 0 0 0 5 0 Playing 2800423536155916460 109
REFUSAL -180000000 0 0 InvalidOrderBounds
CASE refusal_insufficient_bushels_for_land 1337 0 1
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
REFUSAL 200 0 0 InsufficientBushelsForLand
CASE refusal_insufficient_acres_to_sell 1337 0 1
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
REFUSAL -1001 0 0 InsufficientAcresToSell
CASE refusal_insufficient_bushels_to_feed 1337 0 1
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
REFUSAL 0 2801 0 InsufficientBushelsToFeed
CASE refusal_insufficient_acres_to_plant 1337 0 1
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
REFUSAL 0 2000 1001 InsufficientAcresToPlant
CASE refusal_insufficient_population_to_plant 1337 0 1
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
REFUSAL 10 2000 1001 InsufficientPopulationToPlant
CASE refusal_insufficient_grain_for_seed 1337 0 1
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
REFUSAL 0 2700 300 InsufficientGrainForSeed
CASE boundary_sell_exact_owned_acres 1337 1 0
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
TURN 1 -1000 2000 0
STATE 2 107 22800 0 26 4 0 0 0 0 7 0 Playing 11868007961037391445 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 2, 0 PEOPLE STARVED, 7 CAME TO THE CITY.\nPOPULATION IS NOW 107.\nTHE CITY NOW OWNS 0 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 22800 BUSHELS IN STORE.\nLAND IS TRADING AT 26 BUSHELS PER ACRE.\n
CASE boundary_feed_exact_available_bushels 1337 1 0
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
TURN 1 0 2800 0
STATE 2 107 0 1000 26 4 0 0 0 0 7 0 Playing 11868007961037391445 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 2, 0 PEOPLE STARVED, 7 CAME TO THE CITY.\nPOPULATION IS NOW 107.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 0 BUSHELS IN STORE.\nLAND IS TRADING AT 26 BUSHELS PER ACRE.\n
CASE boundary_plant_exact_labor_limit 1337 1 0
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
TURN 1 0 2000 1000
STATE 2 108 4300 1000 26 4 0 0 0 0 8 0 Playing 11868007961037391445 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 2, 0 PEOPLE STARVED, 8 CAME TO THE CITY.\nPOPULATION IS NOW 108.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 4300 BUSHELS IN STORE.\nLAND IS TRADING AT 26 BUSHELS PER ACRE.\n
CASE boundary_plant_exact_seed_limit 1337 1 0
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
TURN 1 0 2700 200
STATE 2 107 800 1000 26 4 0 0 0 0 7 0 Playing 11868007961037391445 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 2, 0 PEOPLE STARVED, 7 CAME TO THE CITY.\nPOPULATION IS NOW 107.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 800 BUSHELS IN STORE.\nLAND IS TRADING AT 26 BUSHELS PER ACRE.\n
CASE boundary_plant_odd_acres_ceil_seed 1337 1 0
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
TURN 1 0 2750 99
STATE 2 107 396 1000 26 4 0 0 0 0 7 0 Playing 11868007961037391445 109
REPORT HAMURABI: I BEG TO REPORT TO YOU,\nIN YEAR 2, 0 PEOPLE STARVED, 7 CAME TO THE CITY.\nPOPULATION IS NOW 107.\nTHE CITY NOW OWNS 1000 ACRES.\nYOU HARVESTED 4 BUSHELS PER ACRE.\nRATS ATE 0 BUSHELS.\nYOU NOW HAVE 396 BUSHELS IN STORE.\nLAND IS TRADING AT 26 BUSHELS PER ACRE.\n
CASE boundary_buy_max_affordable_acres 1337 1 0
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
TURN 1 127 6 0
STATE 2 0 0 1127 26 4 0 100 100 100 0 0 Impeached(starved=100, percent=100) 11868007961037391445 109
REPORT YOU STARVED 100 PEOPLE IN ONE YEAR!!!\nDUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n
CASE refusal_overflow_u32_max_bushels_to_feed 1337 0 1
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
REFUSAL 0 4294967295 0 InsufficientBushelsToFeed
CASE refusal_overflow_u32_max_acres_to_plant 1337 0 1
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
REFUSAL 0 0 4294967295 InsufficientAcresToPlant
CASE refusal_boundary_one_bushel_over_available 1337 0 1
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
REFUSAL 0 2801 0 InsufficientBushelsToFeed
CASE refusal_boundary_one_acre_over_owned 1337 0 1
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
REFUSAL 0 2000 1001 InsufficientAcresToPlant
CASE refusal_boundary_one_bushel_short_seed 1337 0 1
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
REFUSAL 0 2751 100 InsufficientGrainForSeed
CASE refusal_boundary_one_acre_over_labor 1337 0 1
INIT 1 100 2800 1000 22 3 200 0 0 0 5 0 Playing 2800423536155916460 109
REFUSAL 10 2000 1001 InsufficientPopulationToPlant
|}

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
        && (not (String.starts_with ~prefix:"#" trimmed))
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
          | other :: next_rest ->
              collect_case_lines (other :: case_acc) next_rest
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
            | r_line :: after when String.starts_with ~prefix:"REPORT " r_line
              ->
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
