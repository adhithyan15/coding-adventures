(** Pure, deterministic simulation core for the seminal 1968 mainframe
    resource allocation game (GAME01). *)

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

val new_game : int64 -> hamurabi_state
val validate_order : hamurabi_state -> turn_order -> (unit, hamurabi_error) result
val step_year : hamurabi_state -> turn_order -> (hamurabi_state, hamurabi_error) result
val format_report : hamurabi_state -> string
