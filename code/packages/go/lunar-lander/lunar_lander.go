// Package lunarlander provides a pure, deterministic simulation core and ASCII
// telemetry for the classic discrete lunar landing flight simulation (GAME02).
//
// # Historical Lineage
//
//   - 1969 — Jim Storer's Lunar Landing Game: Written in autumn 1969 by Jim Storer
//     on a DEC PDP-8 in FOCAL-69, modelling the physics of the Apollo Lunar
//     Module (LM) descent stage.
//   - 1973 — 101 BASIC Computer Games: Adapted to Dartmouth BASIC by David H.
//     Ahl and published under the title Rocket (and later Lunar), becoming an
//     enduring computing classic.
//
// # Architectural Guarantees
//
// In accordance with the repository's verification architecture (VER00, VER04m,
// VER07, VER08a, VER15):
//   - Zero ambient system authority: "capabilities": [] in required_capabilities.json.
//   - Pure integer fixed-point physics: 64-bit and 128-bit integer arithmetic with
//     zero floating-point operations in the simulation core.
//   - Zero heap allocation: all core simulation functions (NewFlight, ValidateBurn,
//     StepBurn, IsqrtU128, DivEuclid) execute with zero dynamic memory allocations
//     on the simulation path (VER07).
//   - Declarative 2D telemetry: generates intermediate representation PaintScene
//     rendered through a deterministic character-grid rasterizer.
//   - Full CWE weakness closure: CWE-190 (Integer Overflow) and CWE-369 (Divide by Zero).
package lunarlander

import (
	"encoding/json"
	"fmt"
	"math"
	"math/bits"
	"strconv"
	"strings"
	"unicode/utf8"
)

// ============================================================================
// Physical Constants & Apollo LM Parameters
// ============================================================================

// GLunarMftS2 is lunar surface gravitational acceleration in millifeet/s^2 (5.31 ft/s^2).
const GLunarMftS2 int64 = 5310

// DryMassLbs is the constant dry mass of the Apollo Lunar Module descent stage in pounds.
const DryMassLbs uint32 = 16500

// InitialFuelLbs is the initial usable propellant mass in pounds.
const InitialFuelLbs uint32 = 16500

// TotalInitialMassLbs is total initial gross mass (dry mass + propellant) in pounds.
const TotalInitialMassLbs uint32 = 33000

// InitialAltitudeMft is initial mission altitude in millifeet (120,000 ft).
const InitialAltitudeMft int64 = 120000000

// InitialVelocityMftS is initial descent velocity in millifeet/s (-3,000 ft/s, downward).
const InitialVelocityMftS int64 = -3000000

// ExhaustVelocityMftS is descent engine exhaust velocity in millifeet/s (8,000 ft/s).
const ExhaustVelocityMftS int64 = 8000000

// MaxBurnRateLbsS is maximum pilot-commanded engine burn rate in pounds per second.
const MaxBurnRateLbsS uint32 = 200

// MinDurationS is minimum turn duration in seconds.
const MinDurationS uint32 = 1

// MaxDurationS is maximum turn duration in seconds.
const MaxDurationS uint32 = 10

// MaxTimeSeconds is maximum allowable mission elapsed time in seconds (24 hours).
const MaxTimeSeconds uint32 = 86400

// MaxAltitudeMft is maximum plausible simulation altitude boundary in millifeet (200,000 ft).
const MaxAltitudeMft int64 = 200000000

// MinVelocityMftS is minimum valid velocity boundary in millifeet/s (-10,000 ft/s).
const MinVelocityMftS int64 = -10000000

// MaxVelocityMftS is maximum valid velocity boundary in millifeet/s (+10,000 ft/s).
const MaxVelocityMftS int64 = 10000000

// ============================================================================
// Domain Types & State Enums
// ============================================================================

// LandingOutcome classifies terminal landing outcome by touchdown impact speed magnitude.
type LandingOutcome int

const (
	// OutcomePerfect indicates 0 to 5 ft/s (0 to 5,000 mft/s): flawless touchdown.
	OutcomePerfect LandingOutcome = iota
	// OutcomeHard indicates 5.001 to 15 ft/s (5,001 to 15,000 mft/s): minor gear stress.
	OutcomeHard
	// OutcomeCrash indicates 15.001 to 30 ft/s (15,001 to 30,000 mft/s): craft crippled, crew survives.
	OutcomeCrash
	// OutcomeFatal indicates 30.001 to 60 ft/s (30,001 to 60,000 mft/s): total structural destruction.
	OutcomeFatal
	// OutcomeCrater indicates > 60 ft/s (> 60,000 mft/s): blasted a crater into lunar regolith.
	OutcomeCrater
)

func (o LandingOutcome) String() string {
	switch o {
	case OutcomePerfect:
		return "Perfect"
	case OutcomeHard:
		return "Hard"
	case OutcomeCrash:
		return "Crash"
	case OutcomeFatal:
		return "Fatal"
	case OutcomeCrater:
		return "Crater"
	default:
		return fmt.Sprintf("LandingOutcome(%d)", o)
	}
}

// FlightStatusKind identifies the lifecycle stage of a descent mission.
type FlightStatusKind int

const (
	// StatusInFlight indicates active in-flight descent trajectory.
	StatusInFlight FlightStatusKind = iota
	// StatusLanded indicates craft has reached the lunar surface (Y = 0) with a classified outcome.
	StatusLanded
)

// FlightStatus captures lifecycle flight status of the Lunar Module.
type FlightStatus struct {
	Kind    FlightStatusKind
	Outcome LandingOutcome
}

// InFlight represents the active descent flight status.
var InFlight = FlightStatus{Kind: StatusInFlight}

// Landed constructs a landed flight status with the given landing outcome.
func Landed(outcome LandingOutcome) FlightStatus {
	return FlightStatus{Kind: StatusLanded, Outcome: outcome}
}

func (s FlightStatus) String() string {
	if s.Kind == StatusInFlight {
		return "InFlight"
	}
	return fmt.Sprintf("Landed(%s)", s.Outcome.String())
}

// ParseFlightStatus parses a flight status from its canonical string representation.
func ParseFlightStatus(str string) (FlightStatus, error) {
	switch str {
	case "InFlight":
		return InFlight, nil
	case "Landed(Perfect)":
		return Landed(OutcomePerfect), nil
	case "Landed(Hard)":
		return Landed(OutcomeHard), nil
	case "Landed(Crash)":
		return Landed(OutcomeCrash), nil
	case "Landed(Fatal)":
		return Landed(OutcomeFatal), nil
	case "Landed(Crater)":
		return Landed(OutcomeCrater), nil
	default:
		return FlightStatus{}, fmt.Errorf("unrecognized flight status: %s", str)
	}
}

func (s FlightStatus) MarshalJSON() ([]byte, error) {
	return json.Marshal(s.String())
}

func (s *FlightStatus) UnmarshalJSON(data []byte) error {
	var str string
	if err := json.Unmarshal(data, &str); err != nil {
		return err
	}
	parsed, err := ParseFlightStatus(str)
	if err != nil {
		return err
	}
	*s = parsed
	return nil
}

// LanderError is the closed enumeration of burn command rejection errors.
type LanderError string

const (
	// ErrFlightAlreadyEnded indicates attempted command when flight has already terminated.
	ErrFlightAlreadyEnded LanderError = "FlightAlreadyEnded"
	// ErrExcessiveBurnRate indicates requested burn rate exceeds engine maximum (B > 200 lbs/s).
	ErrExcessiveBurnRate LanderError = "ExcessiveBurnRate"
	// ErrInvalidDuration indicates duration is 0 or exceeds turn limit (duration == 0 || duration > 10).
	ErrInvalidDuration LanderError = "InvalidDuration"
	// ErrArithmeticOverflow indicates arithmetic overflow or state coordinate boundary violation.
	ErrArithmeticOverflow LanderError = "ArithmeticOverflow"
)

func (e LanderError) Error() string {
	return string(e)
}

// LanderState is the immutable state record capturing the complete flight simulation state.
// To guarantee zero heap allocations (VER07), impact velocity is represented as value fields.
type LanderState struct {
	TimeSeconds        uint32       `json:"time_seconds"`
	AltitudeMft        int64        `json:"altitude_mft"`
	VelocityMftS       int64        `json:"velocity_mft_s"`
	FuelLbs            uint32       `json:"fuel_lbs"`
	DryMassLbs         uint32       `json:"dry_mass_lbs"`
	Status             FlightStatus `json:"status"`
	HasImpactVelocity  bool         `json:"-"`
	ImpactVelocityMftS int64        `json:"impact_velocity_mft_s"`
}

type landerStateJSON struct {
	TimeSeconds        uint32       `json:"time_seconds"`
	AltitudeMft        int64        `json:"altitude_mft"`
	VelocityMftS       int64        `json:"velocity_mft_s"`
	FuelLbs            uint32       `json:"fuel_lbs"`
	DryMassLbs         uint32       `json:"dry_mass_lbs"`
	Status             FlightStatus `json:"status"`
	ImpactVelocityMftS *int64       `json:"impact_velocity_mft_s"`
}

func (s LanderState) MarshalJSON() ([]byte, error) {
	var imp *int64
	if s.HasImpactVelocity {
		v := s.ImpactVelocityMftS
		imp = &v
	}
	return json.Marshal(landerStateJSON{
		TimeSeconds:        s.TimeSeconds,
		AltitudeMft:        s.AltitudeMft,
		VelocityMftS:       s.VelocityMftS,
		FuelLbs:            s.FuelLbs,
		DryMassLbs:         s.DryMassLbs,
		Status:             s.Status,
		ImpactVelocityMftS: imp,
	})
}

func (s *LanderState) UnmarshalJSON(data []byte) error {
	var raw landerStateJSON
	if err := json.Unmarshal(data, &raw); err != nil {
		return err
	}
	s.TimeSeconds = raw.TimeSeconds
	s.AltitudeMft = raw.AltitudeMft
	s.VelocityMftS = raw.VelocityMftS
	s.FuelLbs = raw.FuelLbs
	s.DryMassLbs = raw.DryMassLbs
	s.Status = raw.Status
	if raw.ImpactVelocityMftS != nil {
		s.HasImpactVelocity = true
		s.ImpactVelocityMftS = *raw.ImpactVelocityMftS
	} else {
		s.HasImpactVelocity = false
		s.ImpactVelocityMftS = 0
	}
	return nil
}

// BurnCommand is the pilot burn order for a discrete simulation turn.
type BurnCommand struct {
	BurnRateLbsS    uint32 `json:"burn_rate_lbs_s"`
	DurationSeconds uint32 `json:"duration_seconds"`
}

// ============================================================================
// Core Pure Simulation Functions
// ============================================================================

// NewFlight initializes a new Lunar Module descent mission at standard initial conditions.
// Guaranteed zero heap allocations (VER07).
func NewFlight() LanderState {
	return LanderState{
		TimeSeconds:        0,
		AltitudeMft:        InitialAltitudeMft,
		VelocityMftS:       InitialVelocityMftS,
		FuelLbs:            InitialFuelLbs,
		DryMassLbs:         DryMassLbs,
		Status:             InFlight,
		HasImpactVelocity:  false,
		ImpactVelocityMftS: 0,
	}
}

func newFlight() LanderState {
	return NewFlight()
}

// DivEuclid performs Euclidean integer division where remainder r is always non-negative: r in [0, |b|).
func DivEuclid(a, b int64) int64 {
	if b == 0 {
		panic("division by zero in DivEuclid")
	}
	q := a / b
	r := a % b
	if r < 0 {
		if b > 0 {
			return q - 1
		}
		return q + 1
	}
	return q
}

func divEuclid(a, b int64) int64 {
	return DivEuclid(a, b)
}

// Uint128 represents an unsigned 128-bit integer composed of two 64-bit words.
type Uint128 struct {
	Hi uint64
	Lo uint64
}

// NewUint128 constructs a Uint128 from high and low 64-bit words.
func NewUint128(hi, lo uint64) Uint128 {
	return Uint128{Hi: hi, Lo: lo}
}

// Uint128FromU64 constructs a Uint128 from a single 64-bit word.
func Uint128FromU64(v uint64) Uint128 {
	return Uint128{Hi: 0, Lo: v}
}

// Cmp compares a and b, returning -1 if a < b, 1 if a > b, and 0 if a == b.
func (a Uint128) Cmp(b Uint128) int {
	if a.Hi < b.Hi {
		return -1
	}
	if a.Hi > b.Hi {
		return 1
	}
	if a.Lo < b.Lo {
		return -1
	}
	if a.Lo > b.Lo {
		return 1
	}
	return 0
}

// Add returns a + b using math/bits carry arithmetic.
func (a Uint128) Add(b Uint128) Uint128 {
	lo, c := bits.Add64(a.Lo, b.Lo, 0)
	hi, _ := bits.Add64(a.Hi, b.Hi, c)
	return Uint128{Hi: hi, Lo: lo}
}

// Sub returns a - b using math/bits borrow arithmetic.
func (a Uint128) Sub(b Uint128) Uint128 {
	lo, borrow := bits.Sub64(a.Lo, b.Lo, 0)
	hi, _ := bits.Sub64(a.Hi, b.Hi, borrow)
	return Uint128{Hi: hi, Lo: lo}
}

// Rsh returns a >> shift.
func (a Uint128) Rsh(shift uint) Uint128 {
	if shift == 0 {
		return a
	}
	if shift >= 128 {
		return Uint128{}
	}
	if shift >= 64 {
		return Uint128{Hi: 0, Lo: a.Hi >> (shift - 64)}
	}
	lo := (a.Lo >> shift) | (a.Hi << (64 - shift))
	hi := a.Hi >> shift
	return Uint128{Hi: hi, Lo: lo}
}

// IsZero reports whether a is zero.
func (a Uint128) IsZero() bool {
	return a.Hi == 0 && a.Lo == 0
}

// IsqrtU128 computes floor(sqrt(n)) using bitwise binary square root.
// Division-free to eliminate any potential division-by-zero hazards (CWE-369).
// Guaranteed zero heap allocations (VER07).
func IsqrtU128(n Uint128) Uint128 {
	if n.IsZero() {
		return Uint128{}
	}
	res := Uint128{}
	bit := Uint128{Hi: 1 << 62, Lo: 0}
	for bit.Cmp(n) > 0 {
		bit = bit.Rsh(2)
	}
	num := n
	for !bit.IsZero() {
		sum := res.Add(bit)
		if num.Cmp(sum) >= 0 {
			num = num.Sub(sum)
			res = res.Rsh(1).Add(bit)
		} else {
			res = res.Rsh(1)
		}
		bit = bit.Rsh(2)
	}
	return res
}

func isqrtU128(n Uint128) Uint128 {
	return IsqrtU128(n)
}

// IsqrtU64 is a convenience helper computing floor(sqrt(n)) for 64-bit unsigned integers.
func IsqrtU64(n uint64) uint64 {
	return IsqrtU128(Uint128FromU64(n)).Lo
}

func clamp(val, low, high int64) int64 {
	if val < low {
		return low
	}
	if val > high {
		return high
	}
	return val
}

// ValidateBurn validates a candidate burn command against current state and physical invariants.
// Enforces canonical validation precedence:
//  1. Lifecycle Check: craft must be InFlight.
//  2. State Invariants (VER04m, CWE-190, CWE-369): dry mass integrity, fuel capacity,
//     in-flight altitude positivity, velocity bounds, mission time limits, and absence of impact velocity.
//  3. Command Bounds: maximum burn rate (<= 200), turn duration (1..=10), and mission time non-overflow.
//
// Guaranteed zero heap allocations (VER07).
func ValidateBurn(state LanderState, cmd BurnCommand) error {
	// 1. Lifecycle Check
	if state.Status != InFlight {
		return ErrFlightAlreadyEnded
	}

	// 2. State Invariants
	if state.DryMassLbs != DryMassLbs {
		return ErrArithmeticOverflow
	}
	if state.FuelLbs > InitialFuelLbs {
		return ErrArithmeticOverflow
	}
	if state.AltitudeMft <= 0 || state.AltitudeMft > MaxAltitudeMft {
		return ErrArithmeticOverflow
	}
	if state.VelocityMftS < MinVelocityMftS || state.VelocityMftS > MaxVelocityMftS {
		return ErrArithmeticOverflow
	}
	if state.TimeSeconds > MaxTimeSeconds {
		return ErrArithmeticOverflow
	}
	if state.HasImpactVelocity {
		return ErrArithmeticOverflow
	}

	// 3. Command Bounds
	if cmd.BurnRateLbsS > MaxBurnRateLbsS {
		return ErrExcessiveBurnRate
	}
	if cmd.DurationSeconds < MinDurationS || cmd.DurationSeconds > MaxDurationS {
		return ErrInvalidDuration
	}
	tNext := uint64(state.TimeSeconds) + uint64(cmd.DurationSeconds)
	if tNext > uint64(MaxTimeSeconds) {
		return ErrArithmeticOverflow
	}

	return nil
}

func validateBurn(state LanderState, cmd BurnCommand) error {
	return ValidateBurn(state, cmd)
}

// StepBurn integrates a single discrete flight step under the commanded burn rate and duration.
// Implements second-order Euler-Verlet numerical kinematics in pure fixed-point millifeet arithmetic.
// When the projected altitude penetrates or reaches lunar surface (Y <= 0),
// solves for exact touchdown contact time delta_t in [1, dt] and impact velocity,
// automatically refunding unburned propellant for any unused interval fraction.
// Guaranteed zero heap allocations (VER07).
func StepBurn(state LanderState, cmd BurnCommand) (LanderState, error) {
	if err := ValidateBurn(state, cmd); err != nil {
		return LanderState{}, err
	}

	dt := int64(cmd.DurationSeconds)
	fuelReq := uint64(cmd.BurnRateLbsS) * uint64(cmd.DurationSeconds)
	fuelAvail := uint64(state.FuelLbs)
	var deltaF uint32
	if fuelAvail < fuelReq {
		deltaF = uint32(fuelAvail)
	} else {
		deltaF = uint32(fuelReq)
	}
	fuelNext := state.FuelLbs - deltaF

	avgMass := int64(state.DryMassLbs) + int64(state.FuelLbs) - (int64(deltaF) / 2)
	if avgMass <= 0 {
		return LanderState{}, ErrArithmeticOverflow
	}
	impulse := int64(deltaF) * ExhaustVelocityMftS
	aThrust := impulse / (dt * avgMass)
	aNet := aThrust - GLunarMftS2

	vNext := state.VelocityMftS + aNet*dt
	yNext := state.AltitudeMft + state.VelocityMftS*dt + divEuclid(aNet*dt*dt, 2)
	tNext := state.TimeSeconds + cmd.DurationSeconds

	if yNext > MaxAltitudeMft || vNext < MinVelocityMftS || vNext > MaxVelocityMftS {
		return LanderState{}, ErrArithmeticOverflow
	}

	if yNext > 0 {
		return LanderState{
			TimeSeconds:        tNext,
			AltitudeMft:        yNext,
			VelocityMftS:       vNext,
			FuelLbs:            fuelNext,
			DryMassLbs:         state.DryMassLbs,
			Status:             InFlight,
			HasImpactVelocity:  false,
			ImpactVelocityMftS: 0,
		}, nil
	}

	// Ground touchdown solver
	aHalf := divEuclid(aNet, 2)
	var deltaTRaw int64
	if aHalf == 0 {
		if state.VelocityMftS == 0 {
			return LanderState{}, ErrArithmeticOverflow
		}
		deltaTRaw = divEuclid(-state.AltitudeMft, state.VelocityMftS)
	} else {
		v := state.VelocityMftS
		y := state.AltitudeMft
		disc := v*v - 4*aHalf*y
		var root int64
		if disc > 0 {
			u := Uint128FromU64(uint64(disc))
			root = int64(IsqrtU128(u).Lo)
		}
		deltaTRaw = divEuclid(-state.VelocityMftS-root, 2*aHalf)
	}

	deltaT := clamp(deltaTRaw, 1, dt)
	deltaTU32 := uint32(deltaT)

	vTouchdown := state.VelocityMftS + aNet*deltaT
	tFinal := state.TimeSeconds + deltaTU32

	burnReqTouchdown := uint64(deltaTU32) * uint64(cmd.BurnRateLbsS)
	var fuelBurnedAtTouchdown uint32
	if uint64(state.FuelLbs) < burnReqTouchdown {
		fuelBurnedAtTouchdown = state.FuelLbs
	} else {
		fuelBurnedAtTouchdown = uint32(burnReqTouchdown)
	}
	fuelFinal := state.FuelLbs - fuelBurnedAtTouchdown

	speedMftS := vTouchdown
	if speedMftS < 0 {
		speedMftS = -speedMftS
	}

	var outcome LandingOutcome
	switch {
	case speedMftS <= 5000:
		outcome = OutcomePerfect
	case speedMftS <= 15000:
		outcome = OutcomeHard
	case speedMftS <= 30000:
		outcome = OutcomeCrash
	case speedMftS <= 60000:
		outcome = OutcomeFatal
	default:
		outcome = OutcomeCrater
	}

	return LanderState{
		TimeSeconds:        tFinal,
		AltitudeMft:        0,
		VelocityMftS:       vTouchdown,
		FuelLbs:            fuelFinal,
		DryMassLbs:         state.DryMassLbs,
		Status:             Landed(outcome),
		HasImpactVelocity:  true,
		ImpactVelocityMftS: vTouchdown,
	}, nil
}

func stepBurn(state LanderState, cmd BurnCommand) (LanderState, error) {
	return StepBurn(state, cmd)
}

// ============================================================================
// ASCII Telemetry & PaintScene Presentation Bridge
// ============================================================================

func formatWithCommas(n int64) string {
	negative := n < 0
	abs := n
	if negative {
		abs = -abs
	}
	s := strconv.FormatInt(abs, 10)
	var sb strings.Builder
	if negative {
		sb.WriteByte('-')
	}
	nDigits := len(s)
	for i := 0; i < nDigits; i++ {
		if i > 0 && (nDigits-i)%3 == 0 {
			sb.WriteByte(',')
		}
		sb.WriteByte(s[i])
	}
	return sb.String()
}

func padRight(s string, width int) string {
	n := utf8.RuneCountInString(s)
	if n >= width {
		return s
	}
	return s + strings.Repeat(" ", width-n)
}

// PaintInstruction represents a 2D drawing instruction.
type PaintInstruction interface {
	InstructionKind() string
}

// PaintText represents text drawing instruction.
type PaintText struct {
	X        float64
	Y        float64
	Text     string
	FontSize float64
	Fill     string
}

// InstructionKind returns "text".
func (PaintText) InstructionKind() string { return "text" }

// PaintRect represents rectangle drawing instruction.
type PaintRect struct {
	X      float64
	Y      float64
	Width  float64
	Height float64
	Fill   string
}

// InstructionKind returns "rect".
func (PaintRect) InstructionKind() string { return "rect" }

// PaintScene represents a declarative 2D scene model.
type PaintScene struct {
	Width        float64
	Height       float64
	Background   string
	Instructions []PaintInstruction
}

// ToPaintScene translates flight state into a declarative 76x5 PaintScene.
func ToPaintScene(state LanderState, lastBurnRateLbsS uint32) PaintScene {
	altClamped := uint64(clamp(state.AltitudeMft, 0, InitialAltitudeMft))
	altPct := (altClamped * 100) / uint64(InitialAltitudeMft)
	altBars := uint32((altClamped * 20) / uint64(InitialAltitudeMft))

	fuelClamped := uint64(state.FuelLbs)
	if fuelClamped > uint64(InitialFuelLbs) {
		fuelClamped = uint64(InitialFuelLbs)
	}
	fuelPct := (fuelClamped * 100) / uint64(InitialFuelLbs)
	fuelBars := uint32((fuelClamped * 20) / uint64(InitialFuelLbs))

	altFt := state.AltitudeMft / 1000
	velFps := state.VelocityMftS / 1000
	fuelLbs := state.FuelLbs

	altStr := formatWithCommas(altFt)
	fuelStr := formatWithCommas(int64(fuelLbs))

	var rateLabel, rateText, rateSymbol string
	if state.Status.Kind == StatusInFlight {
		rateLabel = "RATE:"
		if state.VelocityMftS < 0 {
			rateText = "DESCENT"
			rateSymbol = "▼"
		} else if state.VelocityMftS > 0 {
			rateText = "ASCENT"
			rateSymbol = "▲"
		} else {
			rateText = "HOVER"
			rateSymbol = "◆"
		}
	} else {
		rateLabel = "STATUS:"
		switch state.Status.Outcome {
		case OutcomePerfect:
			rateText = "PERFECT"
		case OutcomeHard:
			rateText = "HARD"
		case OutcomeCrash:
			rateText = "CRASH"
		case OutcomeFatal:
			rateText = "FATAL"
		case OutcomeCrater:
			rateText = "CRATER"
		}
		rateSymbol = "★"
	}

	row0 := "┌───────────────────────── LUNAR LANDER TELEMETRY ─────────────────────────┐"
	left1 := fmt.Sprintf(" TIME: %6d SEC    ", state.TimeSeconds)
	right1 := fmt.Sprintf(" ALTITUDE: %7s FT [░░░░░░░░░░░░░░░░░░░░] %3d%%   ", altStr, altPct)
	row1 := fmt.Sprintf("│%s│%s│", left1, right1)

	left2 := fmt.Sprintf(" VELOCITY: %5d FT/S", velFps)
	right2 := fmt.Sprintf(" FUEL:    %7s LBS [░░░░░░░░░░░░░░░░░░░░] %3d%%   ", fuelStr, fuelPct)
	row2 := fmt.Sprintf("│%s│%s│", left2, right2)

	left3 := fmt.Sprintf(" BURN:     %4d LB/S ", lastBurnRateLbsS)
	rateCol := padRight(rateLabel, 7) + " " + padRight(rateText, 8) + " " + rateSymbol
	row3 := fmt.Sprintf("│%s│ %s│", left3, padRight(rateCol, 51))
	row4 := "└──────────────────────────────────────────────────────────────────────────┘"

	fullText := fmt.Sprintf("%s\n%s\n%s\n%s\n%s", row0, row1, row2, row3, row4)

	instructions := make([]PaintInstruction, 0, 3)
	instructions = append(instructions, PaintText{
		X:        0.0,
		Y:        0.0,
		Text:     fullText,
		FontSize: 1.0,
		Fill:     "#ffffff",
	})

	if altBars > 0 {
		instructions = append(instructions, PaintRect{
			X:      46.0,
			Y:      1.0,
			Width:  float64(altBars - 1),
			Height: 0.0,
			Fill:   "#00ff00",
		})
	}

	if fuelBars > 0 {
		instructions = append(instructions, PaintRect{
			X:      46.0,
			Y:      2.0,
			Width:  float64(fuelBars - 1),
			Height: 0.0,
			Fill:   "#00ff00",
		})
	}

	return PaintScene{
		Width:        76.0,
		Height:       5.0,
		Background:   "#000000",
		Instructions: instructions,
	}
}

func toPaintScene(state LanderState, lastBurnRateLbsS uint32) PaintScene {
	return ToPaintScene(state, lastBurnRateLbsS)
}

// TelemetryScene is convenience helper translating flight state into declarative PaintScene with zero burn.
func TelemetryScene(state LanderState) PaintScene {
	return ToPaintScene(state, 0)
}

func telemetryScene(state LanderState) PaintScene {
	return TelemetryScene(state)
}

// RenderSceneASCII renders PaintScene to a terminal-friendly UTF-8 string with scale_x=1, scale_y=1.
func RenderSceneASCII(scene PaintScene) string {
	cols := int(math.Ceil(scene.Width))
	rows := int(math.Ceil(scene.Height))
	if cols <= 0 || rows <= 0 {
		return ""
	}

	grid := make([][]rune, rows)
	for r := range grid {
		grid[r] = make([]rune, cols)
		for c := range grid[r] {
			grid[r][c] = ' '
		}
	}

	for _, inst := range scene.Instructions {
		switch it := inst.(type) {
		case PaintText:
			if it.Fill == "transparent" || it.Fill == "none" {
				continue
			}
			anchorCol := int(math.Round(it.X))
			firstRow := int(math.Round(it.Y))
			lines := strings.Split(it.Text, "\n")
			for lineIdx, line := range lines {
				r := firstRow + lineIdx
				if r < 0 || r >= rows {
					continue
				}
				colOffset := 0
				for _, ch := range line {
					if ch == '\r' {
						continue
					}
					c := anchorCol + colOffset
					colOffset++
					if c >= 0 && c < cols {
						grid[r][c] = ch
					}
				}
			}
		case PaintRect:
			if it.Fill == "transparent" || it.Fill == "none" || it.Fill == "" {
				continue
			}
			c1 := int(math.Round(it.X))
			r1 := int(math.Round(it.Y))
			c2 := int(math.Round(it.X + it.Width))
			r2 := int(math.Round(it.Y + it.Height))
			for r := r1; r <= r2; r++ {
				if r < 0 || r >= rows {
					continue
				}
				for c := c1; c <= c2; c++ {
					if c >= 0 && c < cols {
						grid[r][c] = '█'
					}
				}
			}
		}
	}

	lines := make([]string, rows)
	for r := 0; r < rows; r++ {
		line := strings.TrimRight(string(grid[r]), " ")
		lines[r] = line
	}
	return strings.TrimRight(strings.Join(lines, "\n"), "\n")
}

// RenderTelemetryASCII renders flight state telemetry dashboard into a deterministic ASCII string.
func RenderTelemetryASCII(state LanderState, lastBurnRateLbsS uint32) string {
	scene := ToPaintScene(state, lastBurnRateLbsS)
	return RenderSceneASCII(scene)
}

func renderTelemetryAscii(state LanderState, lastBurnRateLbsS uint32) string {
	return RenderTelemetryASCII(state, lastBurnRateLbsS)
}

// RenderTelemetry renders flight state telemetry dashboard with zero burn rate.
func RenderTelemetry(state LanderState) string {
	return RenderTelemetryASCII(state, 0)
}

func renderTelemetry(state LanderState) string {
	return RenderTelemetry(state)
}
