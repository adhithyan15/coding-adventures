type rating =
  | Charlemagne
  | Acre
  | Nero
  | Disaster

type game_status =
  | Playing
  | Won of rating
  | Impeached of { starved_this_turn : int; percent : int }
  | GameOverExtinction

type hamurabi_error =
  | GameAlreadyEnded
  | InvalidOrderBounds
  | InsufficientBushelsForLand
  | InsufficientAcresToSell
  | InsufficientBushelsToFeed
  | InsufficientAcresToPlant
  | InsufficientPopulationToPlant
  | InsufficientGrainForSeed

type hamurabi_state = {
  year : int;
  population : int;
  bushels : int;
  acres : int;
  land_price : int;
  harvest_per_acre : int;
  rats_eaten : int;
  starved_last_year : int;
  starved_total : int;
  percent_starved_sum : int;
  immigrants_last_year : int;
  plague_struck : bool;
  status : game_status;
  rng_state : int64;
  rng_inc : int64;
}

type turn_order = {
  acres_delta : int;
  bushels_to_feed : int;
  acres_to_plant : int;
}

let pcg_multiplier = 6364136223846793005L
let hamurabi_stream_id = 54L
let mask32 = 0xFFFFFFFFL

let pcg32_step state inc =
  let old_state = !state in
  state := Int64.add (Int64.mul old_state pcg_multiplier) inc;
  let xor_val = Int64.logxor (Int64.shift_right_logical old_state 18) old_state in
  let shifted = Int64.shift_right_logical xor_val 27 in
  let xsh = Int64.logand shifted mask32 in
  let rot = Int64.to_int (Int64.shift_right_logical old_state 59) land 31 in
  let rot_right =
    if rot = 0 then xsh
    else
      let high_part = Int64.shift_right_logical xsh rot in
      let low_part = Int64.logand (Int64.shift_left xsh (32 - rot)) mask32 in
      Int64.logor high_part low_part
  in
  Int64.logand rot_right mask32

let pcg32_next_int_in_range state inc min_val max_val =
  let span = Int64.of_int (max_val - min_val + 1) in
  let two32 = 0x100000000L in
  let threshold = Int64.rem (Int64.sub two32 span) span in
  let rec loop () =
    let r = pcg32_step state inc in
    if Int64.compare r threshold >= 0 then
      let offset = Int64.to_int (Int64.rem r span) in
      min_val + offset
    else loop ()
  in
  loop ()

let new_game seed =
  let inc = Int64.logor (Int64.shift_left hamurabi_stream_id 1) 1L in
  let rng_state = ref 0L in
  rng_state := Int64.add (Int64.mul !rng_state pcg_multiplier) inc;
  rng_state := Int64.add !rng_state seed;
  rng_state := Int64.add (Int64.mul !rng_state pcg_multiplier) inc;
  let land_price = pcg32_next_int_in_range rng_state inc 17 26 in
  {
    year = 1;
    population = 100;
    bushels = 2800;
    acres = 1000;
    land_price;
    harvest_per_acre = 3;
    rats_eaten = 200;
    starved_last_year = 0;
    starved_total = 0;
    percent_starved_sum = 0;
    immigrants_last_year = 5;
    plague_struck = false;
    status = Playing;
    rng_state = !rng_state;
    rng_inc = inc;
  }

let validate_order state order =
  (* 1. Status Check *)
  if state.status <> Playing then Error GameAlreadyEnded
  (* State invariant checks (VER04m state_inv & CWE-400 / CWE-835) *)
  else if Int64.logand state.rng_inc 1L <> 1L then Error InvalidOrderBounds
  else if state.year < 1 || state.year > 10 then Error InvalidOrderBounds
  else if state.land_price < 17 || state.land_price > 26 then Error InvalidOrderBounds
  else if state.harvest_per_acre < 1 || state.harvest_per_acre > 5 then Error InvalidOrderBounds
  else if state.population < 0 || state.population > 0xFFFFFFFF then Error InvalidOrderBounds
  else if state.bushels < 0 || state.bushels > 0xFFFFFFFF then Error InvalidOrderBounds
  else if state.acres < 0 || state.acres > 0xFFFFFFFF then Error InvalidOrderBounds
  else if order.acres_delta < -2147483648 || order.acres_delta > 2147483647 then Error InvalidOrderBounds
  else if order.bushels_to_feed < 0 || order.bushels_to_feed > 0xFFFFFFFF then Error InvalidOrderBounds
  else if order.acres_to_plant < 0 || order.acres_to_plant > 0xFFFFFFFF then Error InvalidOrderBounds
  else
    let max_u32 = 0xFFFFFFFF in
    (* 2. Bounds & Overflow Guard on Land Trade *)
    let trade_err =
      if order.acres_delta > 0 then
        let cost = order.acres_delta * state.land_price in
        if cost > state.bushels then Some InsufficientBushelsForLand
        else if state.acres + order.acres_delta > max_u32 then Some InvalidOrderBounds
        else None
      else if order.acres_delta < 0 then
        let sell_count = -order.acres_delta in
        if sell_count > state.acres then Some InsufficientAcresToSell
        else
          let revenue = sell_count * state.land_price in
          if state.bushels + revenue > max_u32 then Some InvalidOrderBounds
          else None
      else None
    in
    match trade_err with
    | Some err -> Error err
    | None ->
        let acres_after_trade =
          if order.acres_delta >= 0 then state.acres + order.acres_delta
          else state.acres - (-order.acres_delta)
        in
        if acres_after_trade < 0 || acres_after_trade > max_u32 then Error InvalidOrderBounds
        else
          let bushels_after_trade =
            if order.acres_delta > 0 then state.bushels - (order.acres_delta * state.land_price)
            else if order.acres_delta < 0 then state.bushels + ((-order.acres_delta) * state.land_price)
            else state.bushels
          in
          if bushels_after_trade < 0 || bushels_after_trade > max_u32 then Error InvalidOrderBounds
          else if order.bushels_to_feed > bushels_after_trade then Error InsufficientBushelsToFeed
          else
            let bushels_after_feed = bushels_after_trade - order.bushels_to_feed in
            if order.acres_to_plant > acres_after_trade then Error InsufficientAcresToPlant
            else
              let max_labor = state.population * 10 in
              if order.acres_to_plant > max_labor then Error InsufficientPopulationToPlant
              else
                let seed_needed = (order.acres_to_plant + 1) / 2 in
                if seed_needed > bushels_after_feed then Error InsufficientGrainForSeed
                else
                  let bushels_after_seed = bushels_after_feed - seed_needed in
                  let max_harvest = order.acres_to_plant * 5 in
                  let max_stored = bushels_after_seed + max_harvest in
                  if max_stored > max_u32 then Error InvalidOrderBounds
                  else Ok ()

let step_year state order =
  match validate_order state order with
  | Error err -> Error err
  | Ok () ->
      let acres_after_trade =
        if order.acres_delta >= 0 then state.acres + order.acres_delta
        else state.acres - (-order.acres_delta)
      in
      let bushels_after_trade =
        if order.acres_delta > 0 then state.bushels - (order.acres_delta * state.land_price)
        else if order.acres_delta < 0 then state.bushels + ((-order.acres_delta) * state.land_price)
        else state.bushels
      in
      let bushels_after_feed = bushels_after_trade - order.bushels_to_feed in
      let seed_needed = (order.acres_to_plant + 1) / 2 in
      let bushels_after_seed = bushels_after_feed - seed_needed in

      let rng_state = ref state.rng_state in
      let inc = state.rng_inc in

      (* Step 1: Harvest Yield Draw [1, 5] *)
      let harvest_per_acre = pcg32_next_int_in_range rng_state inc 1 5 in
      let total_harvest = order.acres_to_plant * harvest_per_acre in
      let bushels_stored = bushels_after_seed + total_harvest in

      (* Step 2: Rat Infestation Draws [1, 100] *)
      let rat_roll = pcg32_next_int_in_range rng_state inc 1 100 in
      let rats_eaten =
        if rat_roll <= 40 then
          let rat_factor = pcg32_next_int_in_range rng_state inc 1 5 in
          let eaten = bushels_stored / rat_factor in
          min 0xFFFFFFFF eaten
        else 0
      in
      let bushels_final = min 0xFFFFFFFF (max 0 (bushels_stored - rats_eaten)) in

      (* Step 3: Plague Draw [1, 100] *)
      let plague_roll = pcg32_next_int_in_range rng_state inc 1 100 in
      let plague_struck = plague_roll <= 15 in

      (* Step 4: Starvation & Impeachment Evaluation *)
      let people_fed = order.bushels_to_feed / 20 in
      let starved, survivors0 =
        if people_fed >= state.population then (0, state.population)
        else (state.population - people_fed, people_fed)
      in
      let percent_starved =
        if state.population > 0 then min 100 ((starved * 100) / state.population)
        else 0
      in
      let new_starved_total = min 0xFFFFFFFF (state.starved_total + starved) in
      let new_percent_sum = min 0xFFFFFFFF (state.percent_starved_sum + percent_starved) in

      let status0 =
        if starved * 100 > state.population * 45 then
          Impeached { starved_this_turn = starved; percent = percent_starved }
        else if survivors0 = 0 then GameOverExtinction
        else Playing
      in

      (* Step 5: Plague Resolution *)
      let survivors1 =
        if plague_struck && status0 = Playing then survivors0 / 2
        else survivors0
      in

      (* Step 6: Immigration Draw [1, 5] *)
      let imm_factor = pcg32_next_int_in_range rng_state inc 1 5 in
      let immigrants =
        if starved = 0 && status0 = Playing then
          let acres_term = acres_after_trade * 20 in
          let inner = acres_term + bushels_final in
          let numerator = imm_factor * inner in
          let denominator = 100 * state.population in
          if denominator = 0 then 0
          else min 0xFFFFFFFF ((numerator / denominator) + 1)
        else 0
      in
      let final_population = min 0xFFFFFFFF (survivors1 + immigrants) in

      (* Step 7: Next Year Land Price Draw [17, 26] *)
      let next_land_price = pcg32_next_int_in_range rng_state inc 17 26 in

      (* Step 8: Turn Advance & Rating *)
      let next_year = min 255 (state.year + 1) in
      let final_status =
        if next_year = 11 && status0 = Playing then
          let avg_starved = new_percent_sum / 10 in
          let acres_per_person =
            if final_population > 0 then acres_after_trade / final_population else 0
          in
          let rating =
            if avg_starved <= 3 && acres_per_person >= 10 then Charlemagne
            else if avg_starved <= 10 && acres_per_person >= 9 then Acre
            else if avg_starved <= 33 && acres_per_person >= 7 then Nero
            else Disaster
          in
          Won rating
        else status0
      in

      Ok {
        year = next_year;
        population = final_population;
        bushels = bushels_final;
        acres = acres_after_trade;
        land_price = next_land_price;
        harvest_per_acre;
        rats_eaten;
        starved_last_year = starved;
        starved_total = new_starved_total;
        percent_starved_sum = new_percent_sum;
        immigrants_last_year = immigrants;
        plague_struck;
        status = final_status;
        rng_state = !rng_state;
        rng_inc = inc;
      }

let format_report state =
  match state.status with
  | Impeached { starved_this_turn; _ } ->
      Printf.sprintf
        "YOU STARVED %d PEOPLE IN ONE YEAR!!!\n\
         DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n"
        starved_this_turn
  | GameOverExtinction ->
      "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n"
  | Won rating ->
      let avg_starved = state.percent_starved_sum / 10 in
      let acres_per_person =
        if state.population > 0 then state.acres / state.population else 0
      in
      let rating_msg =
        match rating with
        | Charlemagne ->
            "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!"
        | Acre ->
            "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE."
        | Nero ->
            "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU."
        | Disaster ->
            "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!"
      in
      Printf.sprintf
        "IN YOUR 10-YEAR TERM OF OFFICE, %d PERCENT OF THE\n\
         POPULATION STARVED PER YEAR ON AVERAGE.\n\
         A TOTAL OF %d PEOPLE DIED!!\n\
         YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n\
         %d ACRES PER PERSON.\n\n\
         %s\n"
        avg_starved state.starved_total acres_per_person rating_msg
  | Playing ->
      let base =
        Printf.sprintf
          "HAMURABI: I BEG TO REPORT TO YOU,\n\
           IN YEAR %d, %d PEOPLE STARVED, %d CAME TO THE CITY.\n"
          state.year state.starved_last_year state.immigrants_last_year
      in
      let plague_msg =
        if state.plague_struck then
          "A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n"
        else ""
      in
      let rest =
        Printf.sprintf
          "POPULATION IS NOW %d.\n\
           THE CITY NOW OWNS %d ACRES.\n\
           YOU HARVESTED %d BUSHELS PER ACRE.\n\
           RATS ATE %d BUSHELS.\n\
           YOU NOW HAVE %d BUSHELS IN STORE.\n\
           LAND IS TRADING AT %d BUSHELS PER ACRE.\n"
          state.population state.acres state.harvest_per_acre state.rats_eaten
          state.bushels state.land_price
      in
      base ^ plague_msg ^ rest
