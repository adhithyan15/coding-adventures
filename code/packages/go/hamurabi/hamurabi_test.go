package hamurabi

import (
	"math"
	"strings"
	"sync"
	"testing"
)

func TestInitialState(t *testing.T) {
	state := NewGame(1337)
	if state.Year != 1 {
		t.Errorf("Year = %d; want 1", state.Year)
	}
	if state.Population != 100 {
		t.Errorf("Population = %d; want 100", state.Population)
	}
	if state.Bushels != 2800 {
		t.Errorf("Bushels = %d; want 2800", state.Bushels)
	}
	if state.Acres != 1000 {
		t.Errorf("Acres = %d; want 1000", state.Acres)
	}
	if state.LandPrice < 17 || state.LandPrice > 26 {
		t.Errorf("LandPrice = %d; want in [17, 26]", state.LandPrice)
	}
	if state.HarvestPerAcre != 3 {
		t.Errorf("HarvestPerAcre = %d; want 3", state.HarvestPerAcre)
	}
	if state.RatsEaten != 200 {
		t.Errorf("RatsEaten = %d; want 200", state.RatsEaten)
	}
	if state.StarvedLastYear != 0 {
		t.Errorf("StarvedLastYear = %d; want 0", state.StarvedLastYear)
	}
	if state.StarvedTotal != 0 {
		t.Errorf("StarvedTotal = %d; want 0", state.StarvedTotal)
	}
	if state.PercentStarvedSum != 0 {
		t.Errorf("PercentStarvedSum = %d; want 0", state.PercentStarvedSum)
	}
	if state.ImmigrantsLastYear != 5 {
		t.Errorf("ImmigrantsLastYear = %d; want 5", state.ImmigrantsLastYear)
	}
	if state.PlagueStruck {
		t.Errorf("PlagueStruck = true; want false")
	}
	if state.Status.Kind != StatusPlaying {
		t.Errorf("Status.Kind = %v; want StatusPlaying", state.Status.Kind)
	}
	if (state.RngInc & 1) != 1 {
		t.Errorf("RngInc & 1 = %d; want 1", state.RngInc&1)
	}
}

func TestAllEightErrorArms(t *testing.T) {
	state := NewGame(1337)
	normalOrder := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: 2000,
		AcresToPlant:  100,
	}

	// Arm 1: GameAlreadyEnded
	endedState := state
	endedState.Status = GameStatus{Kind: StatusWon, Rating: RatingCharlemagne}
	if err := ValidateOrder(endedState, normalOrder); err != ErrGameAlreadyEnded {
		t.Errorf("expected ErrGameAlreadyEnded, got %v", err)
	}
	if _, err := StepYear(endedState, normalOrder); err != ErrGameAlreadyEnded {
		t.Errorf("StepYear expected ErrGameAlreadyEnded, got %v", err)
	}

	// Arm 2: InvalidOrderBounds (corrupted land price or arithmetic overflow on trade)
	badState := state
	badState.LandPrice = 10 // valid is 17..=26
	if err := ValidateOrder(badState, normalOrder); err != ErrInvalidOrderBounds {
		t.Errorf("expected ErrInvalidOrderBounds, got %v", err)
	}

	richLandState := state
	richLandState.Acres = 2_000_000_000
	richLandState.Bushels = 4_000_000_000
	overflowSell := TurnOrder{
		AcresDelta:    -1_000_000_000,
		BushelsToFeed: 0,
		AcresToPlant:  0,
	}
	if err := ValidateOrder(richLandState, overflowSell); err != ErrInvalidOrderBounds {
		t.Errorf("expected ErrInvalidOrderBounds, got %v", err)
	}

	// Arm 3: InsufficientBushelsForLand (buying more than bushels allow)
	buyTooMuch := TurnOrder{
		AcresDelta:    200, // 200 * >=17 = 3400 > 2800
		BushelsToFeed: 0,
		AcresToPlant:  0,
	}
	if err := ValidateOrder(state, buyTooMuch); err != ErrInsufficientBushelsForLand {
		t.Errorf("expected ErrInsufficientBushelsForLand, got %v", err)
	}

	// Arm 4: InsufficientAcresToSell (selling more than owned)
	sellTooMuch := TurnOrder{
		AcresDelta:    -1001, // owns 1000
		BushelsToFeed: 0,
		AcresToPlant:  0,
	}
	if err := ValidateOrder(state, sellTooMuch); err != ErrInsufficientAcresToSell {
		t.Errorf("expected ErrInsufficientAcresToSell, got %v", err)
	}

	// Arm 5: InsufficientBushelsToFeed (feeding more than available)
	feedTooMuch := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: 2801, // 2800 in store
		AcresToPlant:  0,
	}
	if err := ValidateOrder(state, feedTooMuch); err != ErrInsufficientBushelsToFeed {
		t.Errorf("expected ErrInsufficientBushelsToFeed, got %v", err)
	}

	// Arm 6: InsufficientAcresToPlant (planting more than owned)
	plantTooMuchAcres := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: 1000,
		AcresToPlant:  1001, // 1000 owned
	}
	if err := ValidateOrder(state, plantTooMuchAcres); err != ErrInsufficientAcresToPlant {
		t.Errorf("expected ErrInsufficientAcresToPlant, got %v", err)
	}

	// Arm 7: InsufficientPopulationToPlant (1 person tends 10 acres)
	smallPopState := state
	smallPopState.Population = 50 // can only tend 500
	plantTooMuchPop := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: 1000,
		AcresToPlant:  501,
	}
	if err := ValidateOrder(smallPopState, plantTooMuchPop); err != ErrInsufficientPopulationToPlant {
		t.Errorf("expected ErrInsufficientPopulationToPlant, got %v", err)
	}

	// Arm 8: InsufficientGrainForSeed (1 bushel seeds 2 acres)
	// 800 acres requires 400 bushels seed. Left after feed: 2800 - 2500 = 300
	seedStarvedOrder := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: 2500,
		AcresToPlant:  800,
	}
	if err := ValidateOrder(state, seedStarvedOrder); err != ErrInsufficientGrainForSeed {
		t.Errorf("expected ErrInsufficientGrainForSeed, got %v", err)
	}
}

func TestAdversarialEvenRngIncRefusal(t *testing.T) {
	state := NewGame(1)
	order := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: 2000,
		AcresToPlant:  1000,
	}

	state.RngInc = 0
	if err := ValidateOrder(state, order); err != ErrInvalidOrderBounds {
		t.Errorf("RngInc=0 expected ErrInvalidOrderBounds, got %v", err)
	}
	if _, err := StepYear(state, order); err != ErrInvalidOrderBounds {
		t.Errorf("RngInc=0 StepYear expected ErrInvalidOrderBounds, got %v", err)
	}

	state.RngInc = 2
	if err := ValidateOrder(state, order); err != ErrInvalidOrderBounds {
		t.Errorf("RngInc=2 expected ErrInvalidOrderBounds, got %v", err)
	}
}

func TestAdversarialStateInvBounds(t *testing.T) {
	validOrder := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: 2000,
		AcresToPlant:  1000,
	}

	// Hostile year = 255
	s := NewGame(1)
	s.Year = 255
	if err := ValidateOrder(s, validOrder); err != ErrInvalidOrderBounds {
		t.Errorf("Year=255 expected ErrInvalidOrderBounds, got %v", err)
	}

	// Hostile year = 0
	s = NewGame(1)
	s.Year = 0
	if err := ValidateOrder(s, validOrder); err != ErrInvalidOrderBounds {
		t.Errorf("Year=0 expected ErrInvalidOrderBounds, got %v", err)
	}

	// Hostile year = 11 (valid playing reign is 1..=10)
	s = NewGame(1)
	s.Year = 11
	if err := ValidateOrder(s, validOrder); err != ErrInvalidOrderBounds {
		t.Errorf("Year=11 expected ErrInvalidOrderBounds, got %v", err)
	}

	// Hostile land_price = 0
	s = NewGame(1)
	s.LandPrice = 0
	if err := ValidateOrder(s, validOrder); err != ErrInvalidOrderBounds {
		t.Errorf("LandPrice=0 expected ErrInvalidOrderBounds, got %v", err)
	}

	// Hostile land_price = 100 (valid is 17..=26)
	s = NewGame(1)
	s.LandPrice = 100
	if err := ValidateOrder(s, validOrder); err != ErrInvalidOrderBounds {
		t.Errorf("LandPrice=100 expected ErrInvalidOrderBounds, got %v", err)
	}

	// Hostile harvest_per_acre = 99 (valid is 1..=5)
	s = NewGame(1)
	s.HarvestPerAcre = 99
	if err := ValidateOrder(s, validOrder); err != ErrInvalidOrderBounds {
		t.Errorf("HarvestPerAcre=99 expected ErrInvalidOrderBounds, got %v", err)
	}

	// Hostile harvest_per_acre = 0
	s = NewGame(1)
	s.HarvestPerAcre = 0
	if err := ValidateOrder(s, validOrder); err != ErrInvalidOrderBounds {
		t.Errorf("HarvestPerAcre=0 expected ErrInvalidOrderBounds, got %v", err)
	}
}

func TestAdversarialStarvedTotalOverflow(t *testing.T) {
	state := NewGame(1)
	state.StarvedTotal = math.MaxUint32
	state.PercentStarvedSum = math.MaxUint32 - 10
	order := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: 1000, // Starves 50 people
		AcresToPlant:  0,
	}
	next, err := StepYear(state, order)
	if err != nil {
		t.Fatalf("unexpected step error: %v", err)
	}
	if next.StarvedTotal != math.MaxUint32 {
		t.Errorf("StarvedTotal = %d; want %d", next.StarvedTotal, uint32(math.MaxUint32))
	}
	if next.PercentStarvedSum != math.MaxUint32 {
		t.Errorf("PercentStarvedSum = %d; want %d", next.PercentStarvedSum, uint32(math.MaxUint32))
	}
}

func TestAdversarialHarvestOverflowRefusal(t *testing.T) {
	state := NewGame(1)
	state.Population = 500_000_000
	state.Acres = 1_000_000_000
	state.Bushels = 2_000_000_000
	order := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: 0,
		AcresToPlant:  900_000_000, // 900M * 5 = 4.5B > math.MaxUint32
	}
	if err := ValidateOrder(state, order); err != ErrInvalidOrderBounds {
		t.Errorf("expected ErrInvalidOrderBounds, got %v", err)
	}
	if _, err := StepYear(state, order); err != ErrInvalidOrderBounds {
		t.Errorf("StepYear expected ErrInvalidOrderBounds, got %v", err)
	}
}

func TestAdversarialAcresDeltaMinInt32(t *testing.T) {
	state := NewGame(1)
	order := TurnOrder{
		AcresDelta:    math.MinInt32, // -2147483648
		BushelsToFeed: 0,
		AcresToPlant:  0,
	}
	// City owns 1000 acres, so attempting to sell 2147483648 acres should fail with InsufficientAcresToSell
	if err := ValidateOrder(state, order); err != ErrInsufficientAcresToSell {
		t.Errorf("AcresDelta=MinInt32 expected ErrInsufficientAcresToSell, got %v", err)
	}

	// Even if city had 3B acres, 2.14B * 17 bushels would overflow math.MaxUint32
	richState := state
	richState.Acres = 3_000_000_000
	if err := ValidateOrder(richState, order); err != ErrInvalidOrderBounds {
		t.Errorf("AcresDelta=MinInt32 revenue overflow expected ErrInvalidOrderBounds, got %v", err)
	}
}

func TestAdversarialBushelsToFeedMaxUint32(t *testing.T) {
	state := NewGame(1)
	order := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: math.MaxUint32,
		AcresToPlant:  0,
	}
	if err := ValidateOrder(state, order); err != ErrInsufficientBushelsToFeed {
		t.Errorf("expected ErrInsufficientBushelsToFeed, got %v", err)
	}
}

func TestAdversarialAcresToPlantMaxUint32(t *testing.T) {
	state := NewGame(1)
	order := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: 0,
		AcresToPlant:  math.MaxUint32,
	}
	if err := ValidateOrder(state, order); err != ErrInsufficientAcresToPlant {
		t.Errorf("expected ErrInsufficientAcresToPlant, got %v", err)
	}
}

func TestRatingsEvaluation(t *testing.T) {
	// Charlemagne
	c := NewGame(1)
	c.Year = 11
	c.Status = NewWonStatus(RatingCharlemagne)
	repC := FormatReport(c)
	if !strings.Contains(repC, "A FANTASTIC PERFORMANCE!") {
		t.Errorf("RatingCharlemagne report missing message: %s", repC)
	}

	// Acre
	a := NewGame(1)
	a.Year = 11
	a.Status = NewWonStatus(RatingAcre)
	repA := FormatReport(a)
	if !strings.Contains(repA, "YOUR PERFORMANCE COULD HAVE BEEN WORSE") {
		t.Errorf("RatingAcre report missing message: %s", repA)
	}

	// Nero
	n := NewGame(1)
	n.Year = 11
	n.Status = NewWonStatus(RatingNero)
	repN := FormatReport(n)
	if !strings.Contains(repN, "NERO WOULD HAVE BEEN PROUD OF YOU") {
		t.Errorf("RatingNero report missing message: %s", repN)
	}

	// Disaster
	d := NewGame(1)
	d.Year = 11
	d.Status = NewWonStatus(RatingDisaster)
	repD := FormatReport(d)
	if !strings.Contains(repD, "OFFICIAL NATIONAL DISASTER") {
		t.Errorf("RatingDisaster report missing message: %s", repD)
	}

	// Impeached
	imp := NewGame(1)
	imp.Status = NewImpeachedStatus(48, 48)
	repImp := FormatReport(imp)
	if !strings.Contains(repImp, "YOU STARVED 48 PEOPLE IN ONE YEAR!!!") {
		t.Errorf("Impeached report missing message: %s", repImp)
	}

	// Extinction
	ext := NewGame(1)
	ext.Status = NewExtinctionStatus()
	repExt := FormatReport(ext)
	if !strings.Contains(repExt, "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!") {
		t.Errorf("Extinction report missing message: %s", repExt)
	}
}

func TestPlaguePopulationReduction(t *testing.T) {
	foundPlague := false
	for seed := uint64(0); seed < 1000; seed++ {
		st := NewGame(seed)
		order := TurnOrder{
			AcresDelta:    0,
			BushelsToFeed: 2000,
			AcresToPlant:  0,
		}
		if next, err := StepYear(st, order); err == nil {
			if next.PlagueStruck {
				foundPlague = true
				if next.Population < 50 {
					t.Errorf("Population = %d after plague; want >= 50", next.Population)
				}
				rep := FormatReport(next)
				if !strings.Contains(rep, "A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.") {
					t.Errorf("Plague report missing strike notice: %s", rep)
				}
				break
			}
		}
	}
	if !foundPlague {
		t.Errorf("should find a plague occurrence in 1000 seeds")
	}
}

func TestFull10YearDeterministicGame(t *testing.T) {
	state := NewGame(4242)

	for expectedYear := uint8(1); expectedYear <= 10; expectedYear++ {
		if state.Year != expectedYear {
			t.Fatalf("Year = %d, want %d", state.Year, expectedYear)
		}
		if state.Status.Kind != StatusPlaying {
			t.Fatalf("Status not playing at year %d", expectedYear)
		}

		feedNeeded := state.Population * 20
		var acresDelta int32
		if state.Bushels < feedNeeded+500 {
			deficit := (feedNeeded + 500) - state.Bushels
			acresToSell := (deficit + state.LandPrice - 1) / state.LandPrice
			if maxSell := state.Acres / 3; acresToSell > maxSell {
				acresToSell = maxSell
			}
			acresDelta = -int32(acresToSell)
		} else if state.Bushels > feedNeeded+2000 {
			surplus := state.Bushels - (feedNeeded + 1000)
			acresToBuy := surplus / state.LandPrice
			if acresToBuy > 50 {
				acresToBuy = 50
			}
			acresDelta = int32(acresToBuy)
		} else {
			acresDelta = 0
		}

		var acresAfterTrade uint32
		if acresDelta >= 0 {
			acresAfterTrade = state.Acres + uint32(acresDelta)
		} else {
			acresAfterTrade = state.Acres - uint32(-acresDelta)
		}

		var bushelsAfterTrade uint32
		if acresDelta > 0 {
			bushelsAfterTrade = state.Bushels - uint32(acresDelta)*state.LandPrice
		} else if acresDelta < 0 {
			bushelsAfterTrade = state.Bushels + uint32(-acresDelta)*state.LandPrice
		} else {
			bushelsAfterTrade = state.Bushels
		}

		bushelsToFeed := feedNeeded
		if bushelsToFeed > bushelsAfterTrade {
			bushelsToFeed = bushelsAfterTrade
		}
		bushelsRemaining := bushelsAfterTrade - bushelsToFeed

		maxPlantable := acresAfterTrade
		if laborLimit := state.Population * 10; maxPlantable > laborLimit {
			maxPlantable = laborLimit
		}
		maxFromSeed := bushelsRemaining * 2
		acresToPlant := maxPlantable
		if acresToPlant > maxFromSeed {
			acresToPlant = maxFromSeed
		}

		order := TurnOrder{
			AcresDelta:    acresDelta,
			BushelsToFeed: bushelsToFeed,
			AcresToPlant:  acresToPlant,
		}

		next, err := StepYear(state, order)
		if err != nil {
			t.Fatalf("year %d step failed: %v", expectedYear, err)
		}
		state = next
	}

	if state.Year != 11 {
		t.Fatalf("Year = %d at end; want 11", state.Year)
	}
	if state.Status.Kind != StatusWon {
		t.Fatalf("Status = %v at end; want StatusWon", state.Status)
	}
	rep := FormatReport(state)
	if !strings.Contains(rep, "IN YOUR 10-YEAR TERM OF OFFICE") {
		t.Errorf("final report missing 10-year term intro: %s", rep)
	}
}

func TestZeroAllocation(t *testing.T) {
	state := NewGame(1337)
	order := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: 2000,
		AcresToPlant:  800,
	}

	allocsValidate := testing.AllocsPerRun(100, func() {
		if err := ValidateOrder(state, order); err != nil {
			t.Fatalf("unexpected validation error: %v", err)
		}
	})
	if allocsValidate != 0 {
		t.Errorf("ValidateOrder allocated %f times; want 0", allocsValidate)
	}

	allocsStep := testing.AllocsPerRun(100, func() {
		if _, err := StepYear(state, order); err != nil {
			t.Fatalf("unexpected step error: %v", err)
		}
	})
	if allocsStep != 0 {
		t.Errorf("StepYear allocated %f times; want 0", allocsStep)
	}
}

func BenchmarkStepYear(b *testing.B) {
	state := NewGame(1337)
	order := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: 2000,
		AcresToPlant:  800,
	}
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = StepYear(state, order)
	}
}

func BenchmarkValidateOrder(b *testing.B) {
	state := NewGame(1337)
	order := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: 2000,
		AcresToPlant:  800,
	}
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = ValidateOrder(state, order)
	}
}

func TestConcurrencyAndRaceSafety(t *testing.T) {
	state := NewGame(42)
	order := TurnOrder{
		AcresDelta:    0,
		BushelsToFeed: 2000,
		AcresToPlant:  800,
	}

	const goroutines = 32
	const iterations = 50
	var wg sync.WaitGroup
	wg.Add(goroutines)

	for g := 0; g < goroutines; g++ {
		go func() {
			defer wg.Done()
			for i := 0; i < iterations; i++ {
				next, err := StepYear(state, order)
				if err != nil {
					t.Errorf("concurrent StepYear failed: %v", err)
					return
				}
				if next.Year != 2 {
					t.Errorf("concurrent StepYear Year = %d, want 2", next.Year)
					return
				}
			}
		}()
	}
	wg.Wait()
}
