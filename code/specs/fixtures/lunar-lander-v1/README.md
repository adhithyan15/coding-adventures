# Lunar Lander Conformance Corpus (lunar-lander-v1)

Canonical deterministic test corpus for the Lunar Lander fixed-point discrete flight simulation (`GAME02`, `LL01`).

## Contents

- `cases.json`: 39 deterministic test cases comprising:
  - **Multi-Turn Trajectories & Terminal Landing Tiers (9 cases)**:
    - `freefall_crater_impact`: Apollo LM ballistic fall from 120,000 ft with zero thrust until surface impact (`LandingOutcome::Crater`).
    - `controlled_perfect_landing`: Controlled two-stage descent with terminal braking achieving flawless touchdown ($|V| \le 5\text{ ft/s}$, `LandingOutcome::Perfect`) with mid-turn propellant refund.
    - `hard_landing_trajectory`: Terminal descent ending in gear stress ($5\text{ ft/s} < |V| \le 15\text{ ft/s}$, `LandingOutcome::Hard`).
    - `crash_landing_trajectory`: Terminal descent resulting in craft wreckage with crew survival ($15\text{ ft/s} < |V| \le 30\text{ ft/s}$, `LandingOutcome::Crash`).
    - `fatal_landing_trajectory`: High-speed descent resulting in total vehicle destruction ($30\text{ ft/s} < |V| \le 60\text{ ft/s}$, `LandingOutcome::Fatal`).
    - `mid_flight_fuel_exhaustion`: Fuel exhaustion mid-burn at altitude followed by zero-thrust ballistic fall and touchdown, verifying the non-phantom fuel refund cap.
    - `hover_near_equilibrium`: Equilibrium hover step where upward thrust acceleration exactly balances lunar gravity ($a_{\text{thrust}} = 5,310\text{ mft/s}^2$, $a_{\text{net}} = 0$).
    - `constant_velocity_touchdown_a_half_zero`: Touchdown under net zero acceleration exercising the linear $\delta t = \lfloor -Y / V \rfloor$ contact time branch.
    - `high_altitude_full_power_braking`: Multi-turn maximum continuous burn ($B = 200\text{ lbs/s}$) reversing vertical velocity from initial descent.
  - **Single Step Physics Dynamics (3 cases)**:
    - `single_step_sub_second_interval`: 1-second burn interval at standard flight condition.
    - `single_step_zero_burn_rate`: 5-second zero thrust ballistic step in lunar gravity.
    - `single_step_max_burn_rate`: 10-second maximum burn rate step ($B = 200\text{ lbs/s}$).
  - **Lifecycle Refusals (5 cases)**:
    - Refusals on subsequent burns after touchdown across landing outcomes (`Perfect`, `Hard`, `Crash`, `Fatal`, `Crater`) returning `LanderError::FlightAlreadyEnded`.
  - **Command Boundary Refusals (5 cases)**:
    - Burn rate bounds: $B = 201\text{ lbs/s}$ and $u32::\text{MAX}$ returning `LanderError::ExcessiveBurnRate`.
    - Interval duration bounds: $\Delta t = 0$, $\Delta t = 11$, and $u32::\text{MAX}$ returning `LanderError::InvalidDuration`.
  - **Mission Time & Addition Overflow Guards (2 cases)**:
    - Mission elapsed time limit: step attempting to advance beyond 86,400s returning `LanderError::ArithmeticOverflow`.
    - Addition overflow guard on elapsed mission time.
  - **CWE-369 Division-by-Zero Guards (2 cases)**:
    - Division-by-zero prevention when `dry_mass_lbs = 0` and `fuel_lbs = 0`.
    - State invariant rejection when `dry_mass_lbs != 16500` (tampered craft dry mass).
  - **CWE-190 Integer Overflow & Hostile State Bounds (13 cases)**:
    - Hostile fuel values: `fuel_lbs = 16501` and `u32::MAX`.
    - Hostile altitude values: `altitude_mft = 0` (while `InFlight`), `-1000`, `200,000,001`, `i64::MAX`, `i64::MIN`.
    - Hostile velocity values: `velocity_mft_s = -10,000,001`, `10,000,001`, `i64::MAX`, `i64::MIN`.
    - Hostile mission time: `time_seconds = 86,401`.
    - Hostile touchdown state: `impact_velocity_mft_s` set while still marked `InFlight`.
