# frozen_string_literal: true

module CodingAdventures
  module LunarLander
    # ============================================================================
    # Physical Constants & Apollo LM Parameters
    # ============================================================================

    # Lunar surface gravitational acceleration in millifeet/s^2 (5.31 ft/s^2).
    G_LUNAR_MFT_S2 = 5_310

    # Constant dry mass of the Apollo Lunar Module descent stage in pounds.
    DRY_MASS_LBS = 16_500

    # Initial usable propellant mass in pounds.
    INITIAL_FUEL_LBS = 16_500

    # Total initial gross mass (dry mass + propellant) in pounds.
    TOTAL_INITIAL_MASS_LBS = 33_000

    # Initial mission altitude in millifeet (120,000 ft).
    INITIAL_ALTITUDE_MFT = 120_000_000

    # Initial descent velocity in millifeet/s (-3,000 ft/s, downward).
    INITIAL_VELOCITY_MFT_S = -3_000_000

    # Descent engine exhaust velocity in millifeet/s (8,000 ft/s).
    EXHAUST_VELOCITY_MFT_S = 8_000_000

    # Maximum pilot-commanded engine burn rate in pounds per second.
    MAX_BURN_RATE_LBS_S = 200

    # Minimum turn duration in seconds.
    MIN_DURATION_S = 1

    # Maximum turn duration in seconds.
    MAX_DURATION_S = 10

    # Maximum allowable mission elapsed time in seconds (24 hours).
    MAX_TIME_SECONDS = 86_400

    # Maximum plausible simulation altitude boundary in millifeet (200,000 ft).
    MAX_ALTITUDE_MFT = 200_000_000

    # Minimum valid velocity boundary in millifeet/s (-10,000 ft/s).
    MIN_VELOCITY_MFT_S = -10_000_000

    # Maximum valid velocity boundary in millifeet/s (+10,000 ft/s).
    MAX_VELOCITY_MFT_S = 10_000_000

    # ============================================================================
    # Pure Deterministic Math Helpers
    # ============================================================================

    # Truncating integer division toward zero.
    #
    # Unlike standard Ruby `/` (which floors negative quotients toward -infinity),
    # this helper strictly truncates towards zero matching C/Rust integer `/`.
    def trunc_div(a, b)
      raise ZeroDivisionError, "divided by 0" if b.zero?
      sign = ((a < 0) ^ (b < 0)) ? -1 : 1
      sign * (a.abs / b.abs)
    end

    # Euclidean division with non-negative remainder r in [0, |b|).
    #
    # Defined via trunc_div to be 100% immune to language-specific `%` / `divmod`
    # sign differences when b < 0.
    # Satisfies: div_euclid(-46070, -5310) == 9
    def div_euclid(a, b)
      q = trunc_div(a, b)
      r = a - q * b
      if r < 0
        b > 0 ? (q - 1) : (q + 1)
      else
        q
      end
    end

    # Computes the exact integer square root floor(sqrt(n)) for 128-bit unsigned integers.
    #
    # Uses an iterative digit-by-digit bitwise binary square root algorithm with bounded
    # step execution (at most 64 bitwise iterations) and zero recursive calls.
    # Division-free to eliminate any potential division-by-zero hazards (CWE-369).
    def isqrt_u128(n)
      return 0 if n <= 0
      res = 0
      bit = 1 << 126
      while bit > n
        bit >>= 2
      end
      num = n
      while bit != 0
        if num >= res + bit
          num -= res + bit
          res = (res >> 1) + bit
        else
          res >>= 1
        end
        bit >>= 2
      end
      res
    end

    # ============================================================================
    # Core Flight Simulation Functions
    # ============================================================================

    # Initializes a new Lunar Module descent mission at standard initial conditions.
    #
    # Guaranteed frozen state with fresh value semantics (VER04m).
    def new_flight
      LanderState.new(
        time_seconds: 0,
        altitude_mft: INITIAL_ALTITUDE_MFT,
        velocity_mft_s: INITIAL_VELOCITY_MFT_S,
        fuel_lbs: INITIAL_FUEL_LBS,
        dry_mass_lbs: DRY_MASS_LBS,
        status: FlightStatus.in_flight,
        impact_velocity_mft_s: nil
      ).freeze
    end

    # Validates a candidate burn command against current state and physical invariants.
    #
    # Canonical validation precedence:
    # 1. Lifecycle Check: craft must be InFlight.
    # 2. State Invariants (VER04m, CWE-190, CWE-369, CWE-843): dry mass integrity,
    #    fuel capacity, in-flight altitude positivity, velocity bounds, mission time limits,
    #    and absence of impact velocity.
    # 3. Command Bounds (CWE-843, CWE-190): maximum burn rate (<= 200), turn duration (1..=10),
    #    and mission time non-overflow.
    def validate_burn(state, cmd)
      state_obj = state.is_a?(LanderState) ? state : LanderState.from_h(state)
      cmd_obj = cmd.is_a?(BurnCommand) ? cmd : BurnCommand.from(cmd)

      # 1. Lifecycle Check
      unless state_obj.status.in_flight?
        raise FlightAlreadyEndedError, "Flight has already ended"
      end

      # 2. State Invariants
      unless state_obj.dry_mass_lbs.is_a?(Integer) && state_obj.dry_mass_lbs == DRY_MASS_LBS
        raise ArithmeticOverflowError, "Dry mass corrupted"
      end
      unless state_obj.fuel_lbs.is_a?(Integer) && state_obj.fuel_lbs >= 0 && state_obj.fuel_lbs <= INITIAL_FUEL_LBS
        raise ArithmeticOverflowError, "Fuel out of bounds"
      end
      unless state_obj.altitude_mft.is_a?(Integer) && state_obj.altitude_mft > 0 && state_obj.altitude_mft <= MAX_ALTITUDE_MFT
        raise ArithmeticOverflowError, "Altitude out of bounds"
      end
      unless state_obj.velocity_mft_s.is_a?(Integer) && state_obj.velocity_mft_s >= MIN_VELOCITY_MFT_S && state_obj.velocity_mft_s <= MAX_VELOCITY_MFT_S
        raise ArithmeticOverflowError, "Velocity out of bounds"
      end
      unless state_obj.time_seconds.is_a?(Integer) && state_obj.time_seconds >= 0 && state_obj.time_seconds <= MAX_TIME_SECONDS
        raise ArithmeticOverflowError, "Time out of bounds"
      end
      unless state_obj.impact_velocity_mft_s.nil?
        raise ArithmeticOverflowError, "Impact velocity must be nil while in flight"
      end

      # 3. Command Bounds
      unless cmd_obj.burn_rate_lbs_s.is_a?(Integer) && cmd_obj.burn_rate_lbs_s >= 0 && cmd_obj.burn_rate_lbs_s <= MAX_BURN_RATE_LBS_S
        raise ExcessiveBurnRateError, "Burn rate exceeds maximum engine capacity"
      end
      unless cmd_obj.duration_seconds.is_a?(Integer) && cmd_obj.duration_seconds >= MIN_DURATION_S && cmd_obj.duration_seconds <= MAX_DURATION_S
        raise InvalidDurationError, "Duration must be between 1 and 10 seconds"
      end

      t_next = state_obj.time_seconds + cmd_obj.duration_seconds
      if t_next > MAX_TIME_SECONDS
        raise ArithmeticOverflowError, "Total flight time exceeds 86,400 seconds"
      end

      true
    end

    # Integrates a single discrete flight step under the commanded burn rate and duration.
    #
    # Implements second-order Euler-Verlet numerical kinematics in pure fixed-point millifeet arithmetic.
    # When the projected altitude penetrates or reaches the lunar surface (Y_{t+dt} <= 0),
    # solves for the exact touchdown contact time delta_t in [1, dt] and impact velocity,
    # automatically refunding unburned propellant for any unused interval fraction.
    def step_burn(state, cmd)
      state_obj = state.is_a?(LanderState) ? state : LanderState.from_h(state)
      cmd_obj = cmd.is_a?(BurnCommand) ? cmd : BurnCommand.from(cmd)

      validate_burn(state_obj, cmd_obj)

      dt = cmd_obj.duration_seconds
      fuel_req = cmd_obj.burn_rate_lbs_s * cmd_obj.duration_seconds
      delta_f = [state_obj.fuel_lbs, fuel_req].min
      fuel_next = state_obj.fuel_lbs - delta_f

      avg_mass = state_obj.dry_mass_lbs + state_obj.fuel_lbs - trunc_div(delta_f, 2)
      impulse = delta_f * EXHAUST_VELOCITY_MFT_S
      a_thrust = trunc_div(impulse, dt * avg_mass)
      a_net = a_thrust - G_LUNAR_MFT_S2

      v_next = state_obj.velocity_mft_s + a_net * dt
      y_next = state_obj.altitude_mft + state_obj.velocity_mft_s * dt + div_euclid(a_net * dt * dt, 2)
      t_next = state_obj.time_seconds + cmd_obj.duration_seconds

      if y_next > MAX_ALTITUDE_MFT || v_next < MIN_VELOCITY_MFT_S || v_next > MAX_VELOCITY_MFT_S
        raise ArithmeticOverflowError, "Kinematic boundaries exceeded"
      end

      if y_next > 0
        LanderState.new(
          time_seconds: t_next,
          altitude_mft: y_next,
          velocity_mft_s: v_next,
          fuel_lbs: fuel_next,
          dry_mass_lbs: state_obj.dry_mass_lbs,
          status: FlightStatus.in_flight,
          impact_velocity_mft_s: nil
        ).freeze
      else
        # Ground touchdown solver
        a_half = div_euclid(a_net, 2)
        delta_t_raw = if a_half.zero?
          if state_obj.velocity_mft_s.zero?
            raise ArithmeticOverflowError, "Zero velocity descent at ground"
          end
          div_euclid(-state_obj.altitude_mft, state_obj.velocity_mft_s)
        else
          v128 = state_obj.velocity_mft_s
          y128 = state_obj.altitude_mft
          a128 = a_half
          disc = v128 * v128 - 4 * a128 * y128
          disc_pos = [disc, 0].max
          root = isqrt_u128(disc_pos)
          div_euclid(-state_obj.velocity_mft_s - root, 2 * a_half)
        end

        delta_t = delta_t_raw.clamp(1, dt)
        v_touchdown = state_obj.velocity_mft_s + a_net * delta_t
        t_final = state_obj.time_seconds + delta_t

        fuel_burned_at_touchdown = [state_obj.fuel_lbs, delta_t * cmd_obj.burn_rate_lbs_s].min
        fuel_final = state_obj.fuel_lbs - fuel_burned_at_touchdown

        outcome = LandingOutcome.from_speed(v_touchdown)

        LanderState.new(
          time_seconds: t_final,
          altitude_mft: 0,
          velocity_mft_s: v_touchdown,
          fuel_lbs: fuel_final,
          dry_mass_lbs: state_obj.dry_mass_lbs,
          status: FlightStatus.landed(outcome),
          impact_velocity_mft_s: v_touchdown
        ).freeze
      end
    end
  end
end
