package lunarlander

import (
	"math"
	"strings"
	"sync"
	"testing"
	"unicode/utf8"
)

func TestInitialFlightInvariants(t *testing.T) {
	flight := NewFlight()
	if flight.TimeSeconds != 0 {
		t.Errorf("expected TimeSeconds == 0, got %d", flight.TimeSeconds)
	}
	if flight.AltitudeMft != InitialAltitudeMft {
		t.Errorf("expected AltitudeMft == %d, got %d", InitialAltitudeMft, flight.AltitudeMft)
	}
	if flight.VelocityMftS != InitialVelocityMftS {
		t.Errorf("expected VelocityMftS == %d, got %d", InitialVelocityMftS, flight.VelocityMftS)
	}
	if flight.FuelLbs != InitialFuelLbs {
		t.Errorf("expected FuelLbs == %d, got %d", InitialFuelLbs, flight.FuelLbs)
	}
	if flight.DryMassLbs != DryMassLbs {
		t.Errorf("expected DryMassLbs == %d, got %d", DryMassLbs, flight.DryMassLbs)
	}
	if flight.Status != InFlight {
		t.Errorf("expected Status == InFlight, got %v", flight.Status)
	}
	if flight.HasImpactVelocity {
		t.Errorf("expected HasImpactVelocity == false, got true")
	}
}

func TestDivEuclid(t *testing.T) {
	tests := []struct {
		a, b int64
		want int64
	}{
		{7, 3, 2},
		{-7, 3, -3},
		{7, -3, -2},
		{-7, -3, 3},
		{0, 5, 0},
		{0, -5, 0},
		{-46070, -5310, 9}, // Critical fixture case (cases.json Turn 4)
	}

	for _, tc := range tests {
		got := DivEuclid(tc.a, tc.b)
		if got != tc.want {
			t.Errorf("DivEuclid(%d, %d) = %d, want %d", tc.a, tc.b, got, tc.want)
		}
		// Confirm remainder r = a - got * b is in [0, |b|)
		r := tc.a - got*tc.b
		absB := tc.b
		if absB < 0 {
			absB = -absB
		}
		if r < 0 || r >= absB {
			t.Errorf("DivEuclid remainder %d not in [0, %d)", r, absB)
		}
	}
}

func TestIsqrtU128Exactness(t *testing.T) {
	tests := []struct {
		hi, lo uint64
		want   uint64
	}{
		{0, 0, 0},
		{0, 1, 1},
		{0, 2, 1},
		{0, 3, 1},
		{0, 4, 2},
		{0, 15, 3},
		{0, 16, 4},
		{0, 10274400000000, 3205370},
		// (2^64 - 1)^2 = 2^128 - 2^65 + 1
		// Hi = 0xFFFFFFFFFFFFFFFE, Lo = 1
		{0xFFFFFFFFFFFFFFFE, 1, math.MaxUint64},
		// 2^128 - 1 -> floor(sqrt(n)) = 2^64 - 1
		{math.MaxUint64, math.MaxUint64, math.MaxUint64},
	}

	for _, tc := range tests {
		input := NewUint128(tc.hi, tc.lo)
		got := IsqrtU128(input)
		if got.Hi != 0 || got.Lo != tc.want {
			t.Errorf("IsqrtU128(%d, %d) = (%d, %d), want (0, %d)", tc.hi, tc.lo, got.Hi, got.Lo, tc.want)
		}
	}

	// Test IsqrtU64 convenience wrapper
	if got := IsqrtU64(10274400000000); got != 3205370 {
		t.Errorf("IsqrtU64 = %d, want 3205370", got)
	}
}

func TestUint128Operations(t *testing.T) {
	a := NewUint128(1, 2)
	b := NewUint128(3, 4)
	sum := a.Add(b)
	if sum.Hi != 4 || sum.Lo != 6 {
		t.Errorf("Add failed: %v", sum)
	}
	diff := sum.Sub(a)
	if diff.Hi != 3 || diff.Lo != 4 {
		t.Errorf("Sub failed: %v", diff)
	}

	// Sub with borrow
	c := NewUint128(1, 0)
	d := NewUint128(0, 1)
	borrowDiff := c.Sub(d)
	if borrowDiff.Hi != 0 || borrowDiff.Lo != math.MaxUint64 {
		t.Errorf("Sub with borrow failed: %v", borrowDiff)
	}

	// Rsh
	rsh1 := NewUint128(2, 0).Rsh(1)
	if rsh1.Hi != 1 || rsh1.Lo != 0 {
		t.Errorf("Rsh 1 failed: %v", rsh1)
	}
	rsh64 := NewUint128(5, 10).Rsh(64)
	if rsh64.Hi != 0 || rsh64.Lo != 5 {
		t.Errorf("Rsh 64 failed: %v", rsh64)
	}
	rsh128 := NewUint128(5, 10).Rsh(128)
	if !rsh128.IsZero() {
		t.Errorf("Rsh 128 failed: %v", rsh128)
	}

	// Cmp
	if a.Cmp(b) >= 0 {
		t.Errorf("Cmp a < b failed")
	}
	if b.Cmp(a) <= 0 {
		t.Errorf("Cmp b > a failed")
	}
	if a.Cmp(a) != 0 {
		t.Errorf("Cmp a == a failed")
	}
}

func TestAdversarialCWE369DivisionByZero(t *testing.T) {
	hostileState := LanderState{
		TimeSeconds:        0,
		AltitudeMft:        100000000,
		VelocityMftS:       -2000000,
		FuelLbs:            0,
		DryMassLbs:         0, // Corrupted dry mass!
		Status:             InFlight,
		HasImpactVelocity:  false,
		ImpactVelocityMftS: 0,
	}
	cmd := BurnCommand{
		BurnRateLbsS:    100,
		DurationSeconds: 5,
	}
	if err := ValidateBurn(hostileState, cmd); err != ErrArithmeticOverflow {
		t.Errorf("expected ErrArithmeticOverflow, got %v", err)
	}
	if _, err := StepBurn(hostileState, cmd); err != ErrArithmeticOverflow {
		t.Errorf("expected ErrArithmeticOverflow, got %v", err)
	}
}

func TestAdversarialCWE190IntegerOverflow(t *testing.T) {
	state := NewFlight()
	validCmd := BurnCommand{
		BurnRateLbsS:    100,
		DurationSeconds: 5,
	}

	// Hostile altitude boundaries
	state.AltitudeMft = math.MaxInt64
	if err := ValidateBurn(state, validCmd); err != ErrArithmeticOverflow {
		t.Errorf("MaxInt64 Altitude: expected ErrArithmeticOverflow, got %v", err)
	}
	state.AltitudeMft = math.MinInt64
	if err := ValidateBurn(state, validCmd); err != ErrArithmeticOverflow {
		t.Errorf("MinInt64 Altitude: expected ErrArithmeticOverflow, got %v", err)
	}
	state.AltitudeMft = MaxAltitudeMft + 1
	if err := ValidateBurn(state, validCmd); err != ErrArithmeticOverflow {
		t.Errorf("> Max Altitude: expected ErrArithmeticOverflow, got %v", err)
	}
	state.AltitudeMft = 0
	if err := ValidateBurn(state, validCmd); err != ErrArithmeticOverflow {
		t.Errorf("0 Altitude in flight: expected ErrArithmeticOverflow, got %v", err)
	}

	// Hostile velocity boundaries
	state = NewFlight()
	state.VelocityMftS = math.MaxInt64
	if err := ValidateBurn(state, validCmd); err != ErrArithmeticOverflow {
		t.Errorf("MaxInt64 Velocity: expected ErrArithmeticOverflow, got %v", err)
	}
	state.VelocityMftS = math.MinInt64
	if err := ValidateBurn(state, validCmd); err != ErrArithmeticOverflow {
		t.Errorf("MinInt64 Velocity: expected ErrArithmeticOverflow, got %v", err)
	}
	state.VelocityMftS = MinVelocityMftS - 1
	if err := ValidateBurn(state, validCmd); err != ErrArithmeticOverflow {
		t.Errorf("< Min Velocity: expected ErrArithmeticOverflow, got %v", err)
	}
	state.VelocityMftS = MaxVelocityMftS + 1
	if err := ValidateBurn(state, validCmd); err != ErrArithmeticOverflow {
		t.Errorf("> Max Velocity: expected ErrArithmeticOverflow, got %v", err)
	}

	// Hostile fuel boundaries
	state = NewFlight()
	state.FuelLbs = InitialFuelLbs + 1
	if err := ValidateBurn(state, validCmd); err != ErrArithmeticOverflow {
		t.Errorf("> Initial Fuel: expected ErrArithmeticOverflow, got %v", err)
	}
	state.FuelLbs = math.MaxUint32
	if err := ValidateBurn(state, validCmd); err != ErrArithmeticOverflow {
		t.Errorf("MaxUint32 Fuel: expected ErrArithmeticOverflow, got %v", err)
	}

	// Hostile time boundaries
	state = NewFlight()
	state.TimeSeconds = MaxTimeSeconds + 1
	if err := ValidateBurn(state, validCmd); err != ErrArithmeticOverflow {
		t.Errorf("> Max Time: expected ErrArithmeticOverflow, got %v", err)
	}
	state.TimeSeconds = MaxTimeSeconds
	addOneS := BurnCommand{BurnRateLbsS: 0, DurationSeconds: 1}
	if err := ValidateBurn(state, addOneS); err != ErrArithmeticOverflow {
		t.Errorf("Time overflow: expected ErrArithmeticOverflow, got %v", err)
	}

	// Hostile command bounds
	state = NewFlight()
	excessiveBurn := BurnCommand{BurnRateLbsS: MaxBurnRateLbsS + 1, DurationSeconds: 5}
	if err := ValidateBurn(state, excessiveBurn); err != ErrExcessiveBurnRate {
		t.Errorf("Excessive burn: expected ErrExcessiveBurnRate, got %v", err)
	}
	zeroDuration := BurnCommand{BurnRateLbsS: 50, DurationSeconds: 0}
	if err := ValidateBurn(state, zeroDuration); err != ErrInvalidDuration {
		t.Errorf("Zero duration: expected ErrInvalidDuration, got %v", err)
	}
	excessiveDuration := BurnCommand{BurnRateLbsS: 50, DurationSeconds: MaxDurationS + 1}
	if err := ValidateBurn(state, excessiveDuration); err != ErrInvalidDuration {
		t.Errorf("Excessive duration: expected ErrInvalidDuration, got %v", err)
	}
	u32MaxDuration := BurnCommand{BurnRateLbsS: 50, DurationSeconds: math.MaxUint32}
	if err := ValidateBurn(state, u32MaxDuration); err != ErrInvalidDuration {
		t.Errorf("MaxUint32 duration: expected ErrInvalidDuration, got %v", err)
	}

	// Hostile impact velocity set while in flight
	state = NewFlight()
	state.HasImpactVelocity = true
	state.ImpactVelocityMftS = -1000
	if err := ValidateBurn(state, validCmd); err != ErrArithmeticOverflow {
		t.Errorf("Impact velocity in flight: expected ErrArithmeticOverflow, got %v", err)
	}
}

func TestPaintVmAsciiTelemetryHUD(t *testing.T) {
	state := NewFlight()
	scene := ToPaintScene(state, 0)
	if scene.Width != 76.0 || scene.Height != 5.0 {
		t.Errorf("scene dimensions = %fx%f, want 76x5", scene.Width, scene.Height)
	}

	hud := RenderTelemetryASCII(state, 0)
	lines := strings.Split(hud, "\n")
	if len(lines) != 5 {
		t.Fatalf("Telemetry HUD must contain exactly 5 lines, got %d", len(lines))
	}

	for idx, line := range lines {
		runeCount := utf8.RuneCountInString(line)
		if runeCount != 76 {
			t.Errorf("Line %d rune count = %d, want 76: %q", idx, runeCount, line)
		}
	}

	keywords := []string{
		"LUNAR LANDER TELEMETRY",
		"TIME:",
		"ALTITUDE:",
		"VELOCITY:",
		"FUEL:",
		"BURN:",
		"RATE:",
		"DESCENT",
		"█",
	}
	for _, kw := range keywords {
		if !strings.Contains(hud, kw) {
			t.Errorf("HUD missing keyword %q", kw)
		}
	}

	// Partial gauge fill containing both █ and ░
	midState := state
	midState.FuelLbs = 8250
	midState.AltitudeMft = 60000000
	hudMid := RenderTelemetryASCII(midState, 100)
	if !strings.Contains(hudMid, "█") || !strings.Contains(hudMid, "░") {
		t.Errorf("Mid-flight HUD must contain both █ and ░")
	}

	// Check convenience helpers
	scene2 := TelemetryScene(state)
	if scene2.Width != 76.0 {
		t.Errorf("TelemetryScene width = %f, want 76.0", scene2.Width)
	}
	hud2 := RenderTelemetry(state)
	if hud2 != hud {
		t.Errorf("RenderTelemetry != RenderTelemetryASCII(state, 0)")
	}
}

func TestTelemetryStatesAndOutcomes(t *testing.T) {
	state := NewFlight()

	// InFlight ascent
	state.VelocityMftS = 50000
	hudAscent := RenderTelemetryASCII(state, 100)
	if !strings.Contains(hudAscent, "ASCENT") || !strings.Contains(hudAscent, "▲") {
		t.Errorf("Ascent HUD missing ASCENT or ▲: %s", hudAscent)
	}

	// InFlight hover
	state.VelocityMftS = 0
	hudHover := RenderTelemetryASCII(state, 50)
	if !strings.Contains(hudHover, "HOVER") || !strings.Contains(hudHover, "◆") {
		t.Errorf("Hover HUD missing HOVER or ◆: %s", hudHover)
	}

	// Landed outcomes
	outcomes := []struct {
		outcome LandingOutcome
		text    string
	}{
		{OutcomePerfect, "PERFECT"},
		{OutcomeHard, "HARD"},
		{OutcomeCrash, "CRASH"},
		{OutcomeFatal, "FATAL"},
		{OutcomeCrater, "CRATER"},
	}

	for _, tc := range outcomes {
		state.Status = Landed(tc.outcome)
		state.HasImpactVelocity = true
		state.ImpactVelocityMftS = -1000
		hudLanded := RenderTelemetryASCII(state, 0)
		if !strings.Contains(hudLanded, "STATUS:") {
			t.Errorf("Landed HUD missing STATUS: label for %s", tc.text)
		}
		if !strings.Contains(hudLanded, tc.text) {
			t.Errorf("Landed HUD missing outcome text %s", tc.text)
		}
		if !strings.Contains(hudLanded, "★") {
			t.Errorf("Landed HUD missing star symbol ★ for %s", tc.text)
		}
	}
}

func TestZeroAllocations(t *testing.T) {
	state := NewFlight()
	cmd := BurnCommand{BurnRateLbsS: 100, DurationSeconds: 5}

	allocsNewFlight := testing.AllocsPerRun(100, func() {
		_ = NewFlight()
	})
	if allocsNewFlight != 0 {
		t.Errorf("NewFlight allocated %f times, want 0", allocsNewFlight)
	}

	allocsValidate := testing.AllocsPerRun(100, func() {
		_ = ValidateBurn(state, cmd)
	})
	if allocsValidate != 0 {
		t.Errorf("ValidateBurn allocated %f times, want 0", allocsValidate)
	}

	allocsStepInFlight := testing.AllocsPerRun(100, func() {
		_, _ = StepBurn(state, cmd)
	})
	if allocsStepInFlight != 0 {
		t.Errorf("StepBurn (in flight) allocated %f times, want 0", allocsStepInFlight)
	}

	// Touchdown step
	touchdownState := LanderState{
		TimeSeconds:        55,
		AltitudeMft:        5725,
		VelocityMftS:       2290,
		FuelLbs:            1750,
		DryMassLbs:         DryMassLbs,
		Status:             InFlight,
		HasImpactVelocity:  false,
		ImpactVelocityMftS: 0,
	}
	touchdownCmd := BurnCommand{BurnRateLbsS: 10, DurationSeconds: 10}
	allocsStepTouchdown := testing.AllocsPerRun(100, func() {
		_, _ = StepBurn(touchdownState, touchdownCmd)
	})
	if allocsStepTouchdown != 0 {
		t.Errorf("StepBurn (touchdown) allocated %f times, want 0", allocsStepTouchdown)
	}
}

func TestConcurrentStepBurn(t *testing.T) {
	state := NewFlight()
	cmd := BurnCommand{BurnRateLbsS: 100, DurationSeconds: 5}
	const goroutines = 16
	const iterations = 1000

	var wg sync.WaitGroup
	wg.Add(goroutines)
	for i := 0; i < goroutines; i++ {
		go func() {
			defer wg.Done()
			for j := 0; j < iterations; j++ {
				next, err := StepBurn(state, cmd)
				if err != nil {
					t.Errorf("unexpected error in concurrent StepBurn: %v", err)
					return
				}
				if next.AltitudeMft <= 0 {
					t.Errorf("unexpected altitude: %d", next.AltitudeMft)
					return
				}
			}
		}()
	}
	wg.Wait()
}

func BenchmarkStepBurn(b *testing.B) {
	state := NewFlight()
	cmd := BurnCommand{BurnRateLbsS: 100, DurationSeconds: 5}
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = StepBurn(state, cmd)
	}
}

func BenchmarkStepBurnTouchdown(b *testing.B) {
	state := LanderState{
		TimeSeconds:        55,
		AltitudeMft:        5725,
		VelocityMftS:       2290,
		FuelLbs:            1750,
		DryMassLbs:         DryMassLbs,
		Status:             InFlight,
		HasImpactVelocity:  false,
		ImpactVelocityMftS: 0,
	}
	cmd := BurnCommand{BurnRateLbsS: 10, DurationSeconds: 10}
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = StepBurn(state, cmd)
	}
}

func BenchmarkValidateBurn(b *testing.B) {
	state := NewFlight()
	cmd := BurnCommand{BurnRateLbsS: 100, DurationSeconds: 5}
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = ValidateBurn(state, cmd)
	}
}

func BenchmarkIsqrtU128(b *testing.B) {
	n := NewUint128(100, 20000000000)
	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = IsqrtU128(n)
	}
}
