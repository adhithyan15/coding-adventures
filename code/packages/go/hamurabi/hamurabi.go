// Package hamurabi provides a pure, deterministic simulation core for the
// seminal 1968 mainframe resource allocation game (GAME01).
//
// # Historical Lineage
//
//   - 1964 — The Sumerian Game: Designed by Mabel Addis (the first credited
//     female video game designer and writer) and programmed by William McKay for
//     an IBM 7090 mainframe at the Board of Cooperative Educational Services
//     (BOCES) in Westchester County, New York. It introduced the economic feedback
//     loop: governing the city-state of Lagash through annual decisions on land
//     acquisition, grain allocation for consumption, and agriculture.
//   - 1968 — FOCAL Hamurabi: Implemented by Richard Merrill on the Digital
//     Equipment Corporation (DEC) PDP-8 in FOCAL-69, shortening the historical
//     Sumerian ruler name to fit the 8-character symbol constraint of PDP-8
//     teletype terminals.
//   - 1973 — 101 BASIC Computer Games: Adapted to Dartmouth BASIC by David H.
//     Ahl and published in DEC's landmark book, becoming the archetypal computer
//     simulation game that influenced an entire generation of programmers.
//
// # Architectural Guarantees
//
// In accordance with the repository's verification architecture (VER00, VER04m,
// VER07, VER08a, VER15):
//   - Zero ambient system authority: "capabilities": [] in required_capabilities.json.
//   - Pure state transition: StepYear(state, order) -> (HamurabiState, error).
//   - Explicit pseudorandomness: all simulation rolls enter deterministically
//     through explicit PCG32 state stored directly inside HamurabiState.
//   - Pure integer arithmetic: uses exclusively bounded integer types (uint8, uint32,
//     uint64, int32) with zero floating-point operations, ensuring bit-exact
//     identical execution traces across all implementations.
//   - Zero heap allocation: StepYear and ValidateOrder operate purely on value
//     records with zero heap allocations on the simulation hot path (VER07).
//   - Concurrency safe: pure immutable value semantics with no shared mutable state.
package hamurabi

import (
	"encoding/json"
	"fmt"
	"math"
	"math/bits"
	"strings"
)

// PCGMultiplier is the 64-bit LCG multiplier constant (Knuth / Numerical Recipes).
const PCGMultiplier uint64 = 6364136223846793005

// HamurabiStreamID is the PCG32 stream sequence identifier for the Hamurabi simulation.
const HamurabiStreamID uint64 = 54

// DefaultRngInc is the canonical odd increment for Hamurabi PCG32: (54 << 1) | 1 = 109.
const DefaultRngInc uint64 = (HamurabiStreamID << 1) | 1

// Rating represents ruler performance evaluation awarded upon completing a 10-year reign.
type Rating int

const (
	// RatingCharlemagne indicates outstanding performance (<= 3% avg starvation and >= 10 acres/person).
	RatingCharlemagne Rating = iota
	// RatingAcre indicates reasonable performance (<= 10% avg starvation and >= 9 acres/person).
	RatingAcre
	// RatingNero indicates poor performance (<= 33% avg starvation and >= 7 acres/person).
	RatingNero
	// RatingDisaster indicates atrocious performance (> 33% avg starvation or < 7 acres/person).
	RatingDisaster
)

// String returns the canonical name of the rating.
func (r Rating) String() string {
	switch r {
	case RatingCharlemagne:
		return "Charlemagne"
	case RatingAcre:
		return "Acre"
	case RatingNero:
		return "Nero"
	case RatingDisaster:
		return "Disaster"
	default:
		return fmt.Sprintf("Rating(%d)", r)
	}
}

// GameStatusKind identifies the lifecycle stage of a simulation.
type GameStatusKind int

const (
	// StatusPlaying indicates an active, playable year (years 1..=10).
	StatusPlaying GameStatusKind = iota
	// StatusWon indicates successful completion of the 10-year term (terminal year 11).
	StatusWon
	// StatusImpeached indicates expulsion due to extreme starvation (> 45% in a single year).
	StatusImpeached
	// StatusGameOverExtinction indicates all citizens perished.
	StatusGameOverExtinction
)

// GameStatus captures the simulation lifecycle status, including terminal ratings or impeachment details.
type GameStatus struct {
	Kind            GameStatusKind
	Rating          Rating
	StarvedThisTurn uint32
	Percent         uint32
}

// NewPlayingStatus constructs an active playing status.
func NewPlayingStatus() GameStatus {
	return GameStatus{Kind: StatusPlaying}
}

// NewWonStatus constructs a winning status with the given performance rating.
func NewWonStatus(r Rating) GameStatus {
	return GameStatus{Kind: StatusWon, Rating: r}
}

// NewImpeachedStatus constructs an impeachment status with starvation details.
func NewImpeachedStatus(starvedThisTurn, percent uint32) GameStatus {
	return GameStatus{
		Kind:            StatusImpeached,
		StarvedThisTurn: starvedThisTurn,
		Percent:         percent,
	}
}

// NewExtinctionStatus constructs a game-over extinction status.
func NewExtinctionStatus() GameStatus {
	return GameStatus{Kind: StatusGameOverExtinction}
}

// String returns the canonical text representation matching conformance fixtures.
func (s GameStatus) String() string {
	switch s.Kind {
	case StatusPlaying:
		return "Playing"
	case StatusWon:
		return fmt.Sprintf("Won(%s)", s.Rating)
	case StatusImpeached:
		return fmt.Sprintf("Impeached(starved=%d, percent=%d)", s.StarvedThisTurn, s.Percent)
	case StatusGameOverExtinction:
		return "GameOverExtinction"
	default:
		return fmt.Sprintf("UnknownStatus(%d)", s.Kind)
	}
}

// ParseGameStatus parses a status string from JSON fixtures.
func ParseGameStatus(str string) (GameStatus, error) {
	if str == "Playing" {
		return GameStatus{Kind: StatusPlaying}, nil
	}
	if str == "GameOverExtinction" {
		return GameStatus{Kind: StatusGameOverExtinction}, nil
	}
	if strings.HasPrefix(str, "Won(") && strings.HasSuffix(str, ")") {
		inner := str[4 : len(str)-1]
		switch inner {
		case "Charlemagne":
			return GameStatus{Kind: StatusWon, Rating: RatingCharlemagne}, nil
		case "Acre":
			return GameStatus{Kind: StatusWon, Rating: RatingAcre}, nil
		case "Nero":
			return GameStatus{Kind: StatusWon, Rating: RatingNero}, nil
		case "Disaster":
			return GameStatus{Kind: StatusWon, Rating: RatingDisaster}, nil
		default:
			return GameStatus{}, fmt.Errorf("unknown rating in status: %s", inner)
		}
	}
	if strings.HasPrefix(str, "Impeached(") && strings.HasSuffix(str, ")") {
		inner := str[10 : len(str)-1]
		var starved, percent uint32
		n, err := fmt.Sscanf(inner, "starved=%d, percent=%d", &starved, &percent)
		if err != nil || n != 2 {
			return GameStatus{}, fmt.Errorf("invalid impeached status format: %s", inner)
		}
		return GameStatus{
			Kind:            StatusImpeached,
			StarvedThisTurn: starved,
			Percent:         percent,
		}, nil
	}
	return GameStatus{}, fmt.Errorf("unrecognized game status: %s", str)
}

// MarshalJSON serializes GameStatus to JSON string format.
func (s GameStatus) MarshalJSON() ([]byte, error) {
	return json.Marshal(s.String())
}

// UnmarshalJSON deserializes GameStatus from JSON string format.
func (s *GameStatus) UnmarshalJSON(data []byte) error {
	var str string
	if err := json.Unmarshal(data, &str); err != nil {
		return err
	}
	parsed, err := ParseGameStatus(str)
	if err != nil {
		return err
	}
	*s = parsed
	return nil
}

// HamurabiError represents an exhaustive enumeration of order rejection errors.
type HamurabiError string

const (
	// ErrGameAlreadyEnded indicates an order was submitted when status is not StatusPlaying.
	ErrGameAlreadyEnded HamurabiError = "GameAlreadyEnded"
	// ErrInvalidOrderBounds indicates order parameters violate numerical bounds or invariant constraints.
	ErrInvalidOrderBounds HamurabiError = "InvalidOrderBounds"
	// ErrInsufficientBushelsForLand indicates lack of grain to purchase ordered acreage.
	ErrInsufficientBushelsForLand HamurabiError = "InsufficientBushelsForLand"
	// ErrInsufficientAcresToSell indicates ordered land sale exceeds owned acreage.
	ErrInsufficientAcresToSell HamurabiError = "InsufficientAcresToSell"
	// ErrInsufficientBushelsToFeed indicates ordered food grain exceeds grain remaining after land trade.
	ErrInsufficientBushelsToFeed HamurabiError = "InsufficientBushelsToFeed"
	// ErrInsufficientAcresToPlant indicates ordered planting acreage exceeds owned land after trade.
	ErrInsufficientAcresToPlant HamurabiError = "InsufficientAcresToPlant"
	// ErrInsufficientPopulationToPlant indicates ordered acreage exceeds labor capacity (1 citizen tends 10 acres).
	ErrInsufficientPopulationToPlant HamurabiError = "InsufficientPopulationToPlant"
	// ErrInsufficientGrainForSeed indicates lack of seed grain (1 bushel seeds 2 acres).
	ErrInsufficientGrainForSeed HamurabiError = "InsufficientGrainForSeed"
)

// Error implements the error interface, returning the canonical error name.
func (e HamurabiError) Error() string {
	return string(e)
}

// HamurabiState captures the complete immutable state of the city-state of Lagash.
type HamurabiState struct {
	// Year is current reign year: 1..=10 are playable; 11 is terminal summary state.
	Year uint8 `json:"year"`
	// Population is current city-state population (initial: 100).
	Population uint32 `json:"population"`
	// Bushels is grain in storage (initial: 2800).
	Bushels uint32 `json:"bushels"`
	// Acres is farmland owned by the city (initial: 1000).
	Acres uint32 `json:"acres"`
	// LandPrice is current market price in bushels per acre (17..=26).
	LandPrice uint32 `json:"land_price"`
	// HarvestPerAcre is yield from previous harvest in bushels per acre (1..=5).
	HarvestPerAcre uint32 `json:"harvest_per_acre"`
	// RatsEaten is bushels destroyed by rats in previous year (initial: 200).
	RatsEaten uint32 `json:"rats_eaten"`
	// StarvedLastYear is citizens starved in previous year (initial: 0).
	StarvedLastYear uint32 `json:"starved_last_year"`
	// StarvedTotal is cumulative citizens starved across entire reign.
	StarvedTotal uint32 `json:"starved_total"`
	// PercentStarvedSum is sum of yearly starvation percentages for 10-year average.
	PercentStarvedSum uint32 `json:"percent_starved_sum"`
	// ImmigrantsLastYear is newcomers arriving in previous year (initial: 5).
	ImmigrantsLastYear uint32 `json:"immigrants_last_year"`
	// PlagueStruck indicates whether plague halved population in previous year.
	PlagueStruck bool `json:"plague_struck"`
	// Status is current simulation lifecycle status.
	Status GameStatus `json:"status"`
	// RngState is internal 64-bit state of PCG32 generator.
	RngState uint64 `json:"rng_state"`
	// RngInc is internal 64-bit sequence increment of PCG32 (must have bit 0 set).
	RngInc uint64 `json:"rng_inc"`
}

// TurnOrder encapsulates player decisions for a single annual turn.
type TurnOrder struct {
	// AcresDelta is acres of land to buy (positive) or sell (negative); 0 means no trade.
	AcresDelta int32 `json:"acres_delta"`
	// BushelsToFeed is grain allocated to feed the population (20 bushels per person).
	BushelsToFeed uint32 `json:"bushels_to_feed"`
	// AcresToPlant is owned land to sow with grain (1 bushel seeds 2 acres).
	AcresToPlant uint32 `json:"acres_to_plant"`
}

// ── PRNG Internal Primitives (PCG32 XSH RR 64/32) ──────────────────────────

func pcg32Step(state *uint64, inc uint64) uint32 {
	oldState := *state
	*state = oldState*PCGMultiplier + inc
	xorshifted := uint32(((oldState >> 18) ^ oldState) >> 27)
	rot := int(oldState >> 59)
	return bits.RotateLeft32(xorshifted, -rot)
}

func pcg32NextIntInRange(state *uint64, inc uint64, minVal, maxVal uint32) uint32 {
	span := uint64(maxVal-minVal) + 1
	threshold := ((uint64(1) << 32) - span) % span
	for {
		r := uint64(pcg32Step(state, inc))
		if r >= threshold {
			return minVal + uint32(r%span)
		}
	}
}

// ── Simulation Engine API ───────────────────────────────────────────────────

// NewGame initializes a deterministic 10-year reign for ruler Hamurabi.
//
// Seeds PCG32 with (seed, stream_id = 54) using the standard PCG initseq warm-up,
// and performs the initial draw for Year 1 land price in [17, 26].
func NewGame(seed uint64) HamurabiState {
	inc := DefaultRngInc
	var rngState uint64 = 0
	rngState = rngState*PCGMultiplier + inc
	rngState += seed
	rngState = rngState*PCGMultiplier + inc

	landPrice := pcg32NextIntInRange(&rngState, inc, 17, 26)

	return HamurabiState{
		Year:               1,
		Population:         100,
		Bushels:            2800,
		Acres:              1000,
		LandPrice:          landPrice,
		HarvestPerAcre:     3,
		RatsEaten:          200,
		StarvedLastYear:    0,
		StarvedTotal:       0,
		PercentStarvedSum:  0,
		ImmigrantsLastYear: 5,
		PlagueStruck:       false,
		Status:             GameStatus{Kind: StatusPlaying},
		RngState:           rngState,
		RngInc:             inc,
	}
}

// ValidateOrder checks player orders against current state constraints without advancing state.
//
// Checks are evaluated in strict canonical order:
//  1. Lifecycle Status check (StatusPlaying)
//  2. State Invariant checks (odd rng_inc, year in [1, 10], land_price in [17, 26], harvest in [1, 5])
//  3. Land Trade resource availability and arithmetic overflow guards
//  4. Food allocation feasibility
//  5. Planting land and labor capacity limits (1 citizen tends 10 acres)
//  6. Seed grain availability (ceil(acres / 2))
//  7. Maximum harvest overflow guard (CWE-190)
func ValidateOrder(state HamurabiState, order TurnOrder) error {
	// 1. Status Check
	if state.Status.Kind != StatusPlaying {
		return ErrGameAlreadyEnded
	}

	// State invariant checks (VER04m state_inv & CWE-400 / CWE-835)
	// - Enforce odd rng_inc to prevent infinite loop / short period in PCG32 LCG
	if (state.RngInc & 1) != 1 {
		return ErrInvalidOrderBounds
	}
	// - Enforce reign year in valid turn bounds [1, 10]
	if state.Year < 1 || state.Year > 10 {
		return ErrInvalidOrderBounds
	}
	// - Enforce land price within canonical range [17, 26]
	if state.LandPrice < 17 || state.LandPrice > 26 {
		return ErrInvalidOrderBounds
	}
	// - Enforce harvest per acre within canonical range [1, 5]
	if state.HarvestPerAcre < 1 || state.HarvestPerAcre > 5 {
		return ErrInvalidOrderBounds
	}

	// 2. Bounds & Overflow Guard on Land Trade
	if order.AcresDelta > 0 {
		buyCount := uint64(order.AcresDelta)
		cost := buyCount * uint64(state.LandPrice)
		if cost > uint64(state.Bushels) {
			return ErrInsufficientBushelsForLand
		}
	} else if order.AcresDelta < 0 {
		// Guard against -math.MinInt32 32-bit overflow by promoting to int64 first
		sellCount64 := -int64(order.AcresDelta)
		if sellCount64 < 0 {
			return ErrInvalidOrderBounds
		}
		sellCount := uint64(sellCount64)
		if sellCount > uint64(state.Acres) {
			return ErrInsufficientAcresToSell
		}
	}

	// 3. Land Trade Balances
	var acresAfterTrade uint32
	if order.AcresDelta >= 0 {
		a := uint64(state.Acres) + uint64(order.AcresDelta)
		if a > math.MaxUint32 {
			return ErrInvalidOrderBounds
		}
		acresAfterTrade = uint32(a)
	} else {
		sellCount := uint64(-int64(order.AcresDelta))
		acresAfterTrade = state.Acres - uint32(sellCount)
	}

	var bushelsAfterTrade uint32
	if order.AcresDelta > 0 {
		cost := uint64(order.AcresDelta) * uint64(state.LandPrice)
		bushelsAfterTrade = state.Bushels - uint32(cost)
	} else if order.AcresDelta < 0 {
		sellCount := uint64(-int64(order.AcresDelta))
		revenue := sellCount * uint64(state.LandPrice)
		if uint64(state.Bushels)+revenue > math.MaxUint32 {
			return ErrInvalidOrderBounds
		}
		bushelsAfterTrade = state.Bushels + uint32(revenue)
	} else {
		bushelsAfterTrade = state.Bushels
	}

	// 4. Food Allocation Check
	if order.BushelsToFeed > bushelsAfterTrade {
		return ErrInsufficientBushelsToFeed
	}
	bushelsAfterFeed := bushelsAfterTrade - order.BushelsToFeed

	// 5. Planting Land & Labor Checks
	if order.AcresToPlant > acresAfterTrade {
		return ErrInsufficientAcresToPlant
	}
	maxLaborAcres := uint64(state.Population) * 10
	if uint64(order.AcresToPlant) > maxLaborAcres {
		return ErrInsufficientPopulationToPlant
	}

	// 6. Planting Seed Check: each bushel seeds 2 acres (ceil(acres / 2))
	seedNeeded := (uint64(order.AcresToPlant) + 1) / 2
	if seedNeeded > uint64(bushelsAfterFeed) {
		return ErrInsufficientGrainForSeed
	}
	bushelsAfterSeed := bushelsAfterFeed - uint32(seedNeeded)

	// 7. Maximum Harvest Overflow Guard (CWE-190)
	// Maximum harvest yield is 5 bushels per acre. Ensure bushels_after_seed + acres_to_plant * 5 <= math.MaxUint32.
	maxHarvest := uint64(order.AcresToPlant) * 5
	maxStored := uint64(bushelsAfterSeed) + maxHarvest
	if maxStored > math.MaxUint32 {
		return ErrInvalidOrderBounds
	}

	return nil
}

// StepYear advances the simulation by exactly one year according to player orders.
//
// Executes in two atomic phases:
//   - Phase A: Validation and constraint checks in canonical order.
//   - Phase B: Strictly locked PRNG draw sequence (harvest, rats, plague, starvation, immigration, next land price).
//
// Returns the updated immutable HamurabiState on success, or an error on refusal.
// The original simulation state is never modified on error.
func StepYear(state HamurabiState, order TurnOrder) (HamurabiState, error) {
	if err := ValidateOrder(state, order); err != nil {
		return state, err
	}

	pcgState := state.RngState
	pcgInc := state.RngInc

	// Intermediate balances from validated trade
	var acresAfterTrade uint32
	if order.AcresDelta >= 0 {
		acresAfterTrade = state.Acres + uint32(order.AcresDelta)
	} else {
		sellCount := uint32(-int64(order.AcresDelta))
		acresAfterTrade = state.Acres - sellCount
	}

	var bushelsAfterTrade uint32
	if order.AcresDelta > 0 {
		cost := uint32(uint64(order.AcresDelta) * uint64(state.LandPrice))
		bushelsAfterTrade = state.Bushels - cost
	} else if order.AcresDelta < 0 {
		sellCount := uint64(-int64(order.AcresDelta))
		revenue := uint32(sellCount * uint64(state.LandPrice))
		bushelsAfterTrade = state.Bushels + revenue
	} else {
		bushelsAfterTrade = state.Bushels
	}

	bushelsAfterFeed := bushelsAfterTrade - order.BushelsToFeed
	seedNeeded := uint32((uint64(order.AcresToPlant) + 1) / 2)
	bushelsAfterSeed := bushelsAfterFeed - seedNeeded

	// Step 1: Harvest Yield Draw
	// Draw 1: pcg.next_int_in_range(1, 5) -> harvest_per_acre (uint32)
	harvestPerAcre := pcg32NextIntInRange(&pcgState, pcgInc, 1, 5)
	totalHarvest := uint64(order.AcresToPlant) * uint64(harvestPerAcre)
	bushelsStored := uint64(bushelsAfterSeed) + totalHarvest

	// Step 2: Rat Infestation Draws
	// Draw 2: pcg.next_int_in_range(1, 100) -> rat_roll (uint32)
	ratRoll := pcg32NextIntInRange(&pcgState, pcgInc, 1, 100)
	var ratsEaten uint32 = 0
	if ratRoll <= 40 {
		// Draw 3: pcg.next_int_in_range(1, 5) -> rat_factor (uint32)
		ratFactor := pcg32NextIntInRange(&pcgState, pcgInc, 1, 5)
		eaten := bushelsStored / uint64(ratFactor)
		if eaten > math.MaxUint32 {
			ratsEaten = math.MaxUint32
		} else {
			ratsEaten = uint32(eaten)
		}
	}
	var bushelsFinal uint32
	if bushelsStored > uint64(ratsEaten) {
		rem := bushelsStored - uint64(ratsEaten)
		if rem > math.MaxUint32 {
			bushelsFinal = math.MaxUint32
		} else {
			bushelsFinal = uint32(rem)
		}
	} else {
		bushelsFinal = 0
	}

	// Step 3: Plague Draw
	// Draw 4: pcg.next_int_in_range(1, 100) -> plague_roll (uint32)
	plagueRoll := pcg32NextIntInRange(&pcgState, pcgInc, 1, 100)
	plagueStruck := (plagueRoll <= 15)

	// Step 4: Starvation & Impeachment Evaluation
	peopleFed := order.BushelsToFeed / 20
	var starved, survivors uint32
	if peopleFed >= state.Population {
		starved = 0
		survivors = state.Population
	} else {
		starved = state.Population - peopleFed
		survivors = peopleFed
	}

	var percentStarved uint32 = 0
	if state.Population > 0 {
		pct := (uint64(starved) * 100) / uint64(state.Population)
		if pct > 100 {
			percentStarved = 100
		} else {
			percentStarved = uint32(pct)
		}
	}

	// Saturating addition for cumulative starvation metrics (CWE-190)
	var newStarvedTotal uint32
	if uint64(state.StarvedTotal)+uint64(starved) > math.MaxUint32 {
		newStarvedTotal = math.MaxUint32
	} else {
		newStarvedTotal = state.StarvedTotal + starved
	}

	var newPercentSum uint32
	if uint64(state.PercentStarvedSum)+uint64(percentStarved) > math.MaxUint32 {
		newPercentSum = math.MaxUint32
	} else {
		newPercentSum = state.PercentStarvedSum + percentStarved
	}

	var newStatus GameStatus
	if uint64(starved)*100 > uint64(state.Population)*45 {
		newStatus = GameStatus{
			Kind:            StatusImpeached,
			StarvedThisTurn: starved,
			Percent:         percentStarved,
		}
	} else if survivors == 0 {
		newStatus = GameStatus{Kind: StatusGameOverExtinction}
	} else {
		newStatus = GameStatus{Kind: StatusPlaying}
	}

	// Step 5: Plague Resolution
	if plagueStruck && newStatus.Kind == StatusPlaying {
		survivors /= 2
	}

	// Step 6: Immigration Draw
	// Draw 5: pcg.next_int_in_range(1, 5) -> imm_factor (uint32)
	immFactor := pcg32NextIntInRange(&pcgState, pcgInc, 1, 5)
	var immigrants uint32 = 0
	if starved == 0 && newStatus.Kind == StatusPlaying {
		acresTerm := uint64(acresAfterTrade) * 20
		inner := acresTerm + uint64(bushelsFinal)
		numerator := uint64(immFactor) * inner
		denominator := 100 * uint64(state.Population)
		if denominator > 0 {
			q := (numerator / denominator) + 1
			if q > math.MaxUint32 {
				immigrants = math.MaxUint32
			} else {
				immigrants = uint32(q)
			}
		}
	}

	var finalPopulation uint32
	if uint64(survivors)+uint64(immigrants) > math.MaxUint32 {
		finalPopulation = math.MaxUint32
	} else {
		finalPopulation = survivors + immigrants
	}

	// Step 7: Next Year Land Price Draw
	// Draw 6: pcg.next_int_in_range(17, 26) -> next_land_price (uint32)
	nextLandPrice := pcg32NextIntInRange(&pcgState, pcgInc, 17, 26)

	// Step 8: Turn Advance & Rating
	var nextYear uint8
	if state.Year < 255 {
		nextYear = state.Year + 1
	} else {
		nextYear = 255
	}

	if nextYear == 11 && newStatus.Kind == StatusPlaying {
		avgStarved := newPercentSum / 10
		var acresPerPerson uint32 = 0
		if finalPopulation > 0 {
			acresPerPerson = acresAfterTrade / finalPopulation
		}
		var rating Rating
		if avgStarved <= 3 && acresPerPerson >= 10 {
			rating = RatingCharlemagne
		} else if avgStarved <= 10 && acresPerPerson >= 9 {
			rating = RatingAcre
		} else if avgStarved <= 33 && acresPerPerson >= 7 {
			rating = RatingNero
		} else {
			rating = RatingDisaster
		}
		newStatus = GameStatus{Kind: StatusWon, Rating: rating}
	}

	return HamurabiState{
		Year:               nextYear,
		Population:         finalPopulation,
		Bushels:            bushelsFinal,
		Acres:              acresAfterTrade,
		LandPrice:          nextLandPrice,
		HarvestPerAcre:     harvestPerAcre,
		RatsEaten:          ratsEaten,
		StarvedLastYear:    starved,
		StarvedTotal:       newStarvedTotal,
		PercentStarvedSum:  newPercentSum,
		ImmigrantsLastYear: immigrants,
		PlagueStruck:       plagueStruck,
		Status:             newStatus,
		RngState:           pcgState,
		RngInc:             pcgInc,
	}, nil
}

// FormatReport generates the annual teletype report or final evaluation in authentic 1970s teletype styling.
func FormatReport(state HamurabiState) string {
	switch state.Status.Kind {
	case StatusImpeached:
		return fmt.Sprintf(
			"YOU STARVED %d PEOPLE IN ONE YEAR!!!\n"+
				"DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n",
			state.Status.StarvedThisTurn,
		)
	case StatusGameOverExtinction:
		return "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n"
	case StatusWon:
		avgStarved := state.PercentStarvedSum / 10
		var acresPerPerson uint32 = 0
		if state.Population > 0 {
			acresPerPerson = state.Acres / state.Population
		}
		var ratingMsg string
		switch state.Status.Rating {
		case RatingCharlemagne:
			ratingMsg = "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!"
		case RatingAcre:
			ratingMsg = "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE."
		case RatingNero:
			ratingMsg = "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU."
		case RatingDisaster:
			ratingMsg = "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!"
		}
		return fmt.Sprintf(
			"IN YOUR 10-YEAR TERM OF OFFICE, %d PERCENT OF THE\n"+
				"POPULATION STARVED PER YEAR ON AVERAGE.\n"+
				"A TOTAL OF %d PEOPLE DIED!!\n"+
				"YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n"+
				"%d ACRES PER PERSON.\n\n"+
				"%s\n",
			avgStarved, state.StarvedTotal, acresPerPerson, ratingMsg,
		)
	case StatusPlaying:
		var b strings.Builder
		fmt.Fprintf(&b,
			"HAMURABI: I BEG TO REPORT TO YOU,\n"+
				"IN YEAR %d, %d PEOPLE STARVED, %d CAME TO THE CITY.\n",
			state.Year, state.StarvedLastYear, state.ImmigrantsLastYear,
		)
		if state.PlagueStruck {
			b.WriteString("A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n")
		}
		fmt.Fprintf(&b,
			"POPULATION IS NOW %d.\n"+
				"THE CITY NOW OWNS %d ACRES.\n"+
				"YOU HARVESTED %d BUSHELS PER ACRE.\n"+
				"RATS ATE %d BUSHELS.\n"+
				"YOU NOW HAVE %d BUSHELS IN STORE.\n"+
				"LAND IS TRADING AT %d BUSHELS PER ACRE.\n",
			state.Population,
			state.Acres,
			state.HarvestPerAcre,
			state.RatsEaten,
			state.Bushels,
			state.LandPrice,
		)
		return b.String()
	default:
		return ""
	}
}

// FormatReport returns the annual teletype report formatted from the state.
func (s HamurabiState) FormatReport() string {
	return FormatReport(s)
}
