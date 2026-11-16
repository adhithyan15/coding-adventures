// hamurabi.dart — Deterministic Hamurabi simulation core in Dart
// ===============================================================
//
// Pure, deterministic simulation core for the seminal 1968 mainframe resource
// allocation game Hamurabi (GAME01 / HA01).
//
// In accordance with repository verification architecture:
// - Zero ambient system authority: capabilities: [] in required_capabilities.json
// - Pure state transitions: stepYear(state, order) -> Result<HamurabiState, HamurabiError>
// - Explicit pseudorandomness: PCG32 state stored directly inside HamurabiState
// - CWE-190 / CWE-681 mitigation: On native Dart VM, int is 64-bit signed two's-complement.
//   We use BigInt for 64-bit unsigned PCG32 state/inc transitions and checked int
//   for bounded u32/i32 domain math.

// ─── Constants ───────────────────────────────────────────────────────────────

final BigInt _pcgMultiplier = BigInt.parse('6364136223846793005');
final BigInt _mask32 = BigInt.parse('0xFFFFFFFF');
const int _streamId = 54;

// ─── Domain Models ──────────────────────────────────────────────────────────

/// Rating classification awarded to the ruler upon completing a 10-year term.
enum Rating {
  /// Average starvation <= 3% AND acres per person >= 10.
  charlemagne,

  /// Average starvation <= 10% AND acres per person >= 9.
  acre,

  /// Average starvation <= 33% AND acres per person >= 7.
  nero,

  /// Average starvation > 33% OR acres per person < 7.
  disaster;

  String get displayName {
    switch (this) {
      case Rating.charlemagne:
        return 'Charlemagne';
      case Rating.acre:
        return 'Acre';
      case Rating.nero:
        return 'Nero';
      case Rating.disaster:
        return 'Disaster';
    }
  }
}

/// Simulation lifecycle status.
sealed class GameStatus {
  const GameStatus();
  const factory GameStatus.playing() = Playing;
  const factory GameStatus.won(Rating rating) = Won;
  const factory GameStatus.impeached(int starvedThisTurn, int percent) = Impeached;
  const factory GameStatus.gameOverExtinction() = GameOverExtinction;

  static GameStatus parse(String s) {
    if (s == 'Playing') return const GameStatus.playing();
    if (s == 'GameOverExtinction') return const GameStatus.gameOverExtinction();
    if (s.startsWith('Won(')) {
      final name = s.substring(4, s.length - 1);
      final rating = Rating.values.firstWhere(
        (r) => r.displayName.toLowerCase() == name.toLowerCase(),
        orElse: () => throw ArgumentError('Unknown rating: $name'),
      );
      return GameStatus.won(rating);
    }
    if (s.startsWith('Impeached(')) {
      final match = RegExp(r'Impeached\(starved=(\d+),\s*percent=(\d+)\)').firstMatch(s);
      if (match == null) throw ArgumentError('Invalid Impeached format: $s');
      final starved = int.parse(match.group(1)!);
      final percent = int.parse(match.group(2)!);
      return GameStatus.impeached(starved, percent);
    }
    throw ArgumentError('Unknown GameStatus: $s');
  }
}

/// Active, playable year (years 1..=10).
final class Playing extends GameStatus {
  const Playing();
  @override
  bool operator ==(Object other) => other is Playing;
  @override
  int get hashCode => 1;
  @override
  String toString() => 'Playing';
}

/// Completed 10-year term successfully (transitioned to terminal year 11).
final class Won extends GameStatus {
  final Rating rating;
  const Won(this.rating);
  @override
  bool operator ==(Object other) => other is Won && other.rating == rating;
  @override
  int get hashCode => rating.hashCode;
  @override
  String toString() => 'Won(${rating.displayName})';
}

/// Impeached and expelled from office due to extreme starvation (> 45%).
final class Impeached extends GameStatus {
  final int starvedThisTurn;
  final int percent;
  const Impeached(this.starvedThisTurn, this.percent);
  @override
  bool operator ==(Object other) =>
      other is Impeached &&
      other.starvedThisTurn == starvedThisTurn &&
      other.percent == percent;
  @override
  int get hashCode => Object.hash(starvedThisTurn, percent);
  @override
  String toString() => 'Impeached(starved=$starvedThisTurn, percent=$percent)';
}

/// Entire population died of starvation or disaster.
final class GameOverExtinction extends GameStatus {
  const GameOverExtinction();
  @override
  bool operator ==(Object other) => other is GameOverExtinction;
  @override
  int get hashCode => 2;
  @override
  String toString() => 'GameOverExtinction';
}

/// Closed enumeration of order rejection errors.
enum HamurabiError implements Exception {
  /// Attempted to execute an order when status is not Playing.
  gameAlreadyEnded('Game has already ended'),

  /// Arithmetic overflow or invalid bounds on input parameters.
  invalidOrderBounds('Order parameters exceed valid numerical bounds or overflow'),

  /// Player ordered to buy land but lacks sufficient bushels.
  insufficientBushelsForLand('Insufficient bushels to buy specified acreage'),

  /// Player ordered to sell more land than the city owns.
  insufficientAcresToSell('Cannot sell more acres than the city owns'),

  /// Player ordered to feed more bushels than available after land trade.
  insufficientBushelsToFeed('Insufficient bushels in store to feed population'),

  /// Player ordered to plant more acres than owned after land trade.
  insufficientAcresToPlant('Cannot plant more acres than owned after land trade'),

  /// Player ordered to plant more acres than population can tend (1 person tends 10 acres).
  insufficientPopulationToPlant('Insufficient population to tend planted acreage'),

  /// Player lacks grain to seed fields (1 bushel seeds 2 acres).
  insufficientGrainForSeed('Insufficient grain to seed planted acreage');

  final String message;
  const HamurabiError(this.message);

  String get code {
    switch (this) {
      case HamurabiError.gameAlreadyEnded:
        return 'GameAlreadyEnded';
      case HamurabiError.invalidOrderBounds:
        return 'InvalidOrderBounds';
      case HamurabiError.insufficientBushelsForLand:
        return 'InsufficientBushelsForLand';
      case HamurabiError.insufficientAcresToSell:
        return 'InsufficientAcresToSell';
      case HamurabiError.insufficientBushelsToFeed:
        return 'InsufficientBushelsToFeed';
      case HamurabiError.insufficientAcresToPlant:
        return 'InsufficientAcresToPlant';
      case HamurabiError.insufficientPopulationToPlant:
        return 'InsufficientPopulationToPlant';
      case HamurabiError.insufficientGrainForSeed:
        return 'InsufficientGrainForSeed';
    }
  }

  static HamurabiError fromCode(String code) {
    return HamurabiError.values.firstWhere(
      (e) => e.code == code,
      orElse: () => throw ArgumentError('Unknown error code: $code'),
    );
  }
}

/// Player orders for a single year.
class TurnOrder {
  /// Acres of land to buy (positive) or sell (negative). 0 indicates no trade.
  final int acresDelta;

  /// Bushels of grain allocated to feed the population (20 bushels per person).
  final int bushelsToFeed;

  /// Acres of owned land to sow with grain (1 bushel seeds 2 acres).
  final int acresToPlant;

  const TurnOrder({
    required this.acresDelta,
    required this.bushelsToFeed,
    required this.acresToPlant,
  });

  factory TurnOrder.fromJson(Map<String, dynamic> json) {
    return TurnOrder(
      acresDelta: (json['acres_delta'] as num).toInt(),
      bushelsToFeed: (json['bushels_to_feed'] as num).toInt(),
      acresToPlant: (json['acres_to_plant'] as num).toInt(),
    );
  }

  Map<String, dynamic> toJson() => {
        'acres_delta': acresDelta,
        'bushels_to_feed': bushelsToFeed,
        'acres_to_plant': acresToPlant,
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TurnOrder &&
          runtimeType == other.runtimeType &&
          acresDelta == other.acresDelta &&
          bushelsToFeed == other.bushelsToFeed &&
          acresToPlant == other.acresToPlant;

  @override
  int get hashCode => Object.hash(acresDelta, bushelsToFeed, acresToPlant);

  @override
  String toString() =>
      'TurnOrder(acresDelta: $acresDelta, bushelsToFeed: $bushelsToFeed, acresToPlant: $acresToPlant)';
}

/// Immutable value record capturing the complete simulation state.
class HamurabiState {
  /// Current year of reign: 1..=10 are playable; 11 is terminal summary state.
  final int year;

  /// Current population of the city-state (initial: 100).
  final int population;

  /// Bushels of grain in storage (initial: 2800).
  final int bushels;

  /// Acres of farmland owned by the city (initial: 1000).
  final int acres;

  /// Current market price of land in bushels per acre (17..=26).
  final int landPrice;

  /// Yield from the previous harvest in bushels per acre (1..=5).
  final int harvestPerAcre;

  /// Bushels consumed by rats during previous year (initial: 200).
  final int ratsEaten;

  /// Citizens starved during previous year (initial: 0).
  final int starvedLastYear;

  /// Cumulative citizens starved across the entire reign.
  final int starvedTotal;

  /// Sum of yearly starvation percentages (for 10-year average computation).
  final int percentStarvedSum;

  /// New citizens arriving during previous year (initial: 5).
  final int immigrantsLastYear;

  /// Whether a plague occurred in the previous year.
  final bool plagueStruck;

  /// Current game lifecycle status.
  final GameStatus status;

  /// Internal 64-bit unsigned state of the PCG32 generator.
  final BigInt rngState;

  /// Internal 64-bit unsigned sequence increment of PCG32 (always odd).
  final BigInt rngInc;

  const HamurabiState({
    required this.year,
    required this.population,
    required this.bushels,
    required this.acres,
    required this.landPrice,
    required this.harvestPerAcre,
    required this.ratsEaten,
    required this.starvedLastYear,
    required this.starvedTotal,
    required this.percentStarvedSum,
    required this.immigrantsLastYear,
    required this.plagueStruck,
    required this.status,
    required this.rngState,
    required this.rngInc,
  });

  factory HamurabiState.fromJson(Map<String, dynamic> json) {
    return HamurabiState(
      year: (json['year'] as num).toInt(),
      population: (json['population'] as num).toInt(),
      bushels: (json['bushels'] as num).toInt(),
      acres: (json['acres'] as num).toInt(),
      landPrice: (json['land_price'] as num).toInt(),
      harvestPerAcre: (json['harvest_per_acre'] as num).toInt(),
      ratsEaten: (json['rats_eaten'] as num).toInt(),
      starvedLastYear: (json['starved_last_year'] as num).toInt(),
      starvedTotal: (json['starved_total'] as num).toInt(),
      percentStarvedSum: (json['percent_starved_sum'] as num).toInt(),
      immigrantsLastYear: (json['immigrants_last_year'] as num).toInt(),
      plagueStruck: json['plague_struck'] as bool,
      status: GameStatus.parse(json['status'] as String),
      rngState: BigInt.parse(json['rng_state'].toString()).toUnsigned(64),
      rngInc: BigInt.parse(json['rng_inc'].toString()).toUnsigned(64),
    );
  }

  Map<String, dynamic> toJson() => {
        'year': year,
        'population': population,
        'bushels': bushels,
        'acres': acres,
        'land_price': landPrice,
        'harvest_per_acre': harvestPerAcre,
        'rats_eaten': ratsEaten,
        'starved_last_year': starvedLastYear,
        'starved_total': starvedTotal,
        'percent_starved_sum': percentStarvedSum,
        'immigrants_last_year': immigrantsLastYear,
        'plague_struck': plagueStruck,
        'status': status.toString(),
        'rng_state': rngState.toString(),
        'rng_inc': rngInc.toString(),
      };

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is HamurabiState &&
          runtimeType == other.runtimeType &&
          year == other.year &&
          population == other.population &&
          bushels == other.bushels &&
          acres == other.acres &&
          landPrice == other.landPrice &&
          harvestPerAcre == other.harvestPerAcre &&
          ratsEaten == other.ratsEaten &&
          starvedLastYear == other.starvedLastYear &&
          starvedTotal == other.starvedTotal &&
          percentStarvedSum == other.percentStarvedSum &&
          immigrantsLastYear == other.immigrantsLastYear &&
          plagueStruck == other.plagueStruck &&
          status == other.status &&
          rngState == other.rngState &&
          rngInc == other.rngInc;

  @override
  int get hashCode => Object.hashAll([
        year,
        population,
        bushels,
        acres,
        landPrice,
        harvestPerAcre,
        ratsEaten,
        starvedLastYear,
        starvedTotal,
        percentStarvedSum,
        immigrantsLastYear,
        plagueStruck,
        status,
        rngState,
        rngInc,
      ]);

  @override
  String toString() =>
      'HamurabiState(year: $year, pop: $population, bushels: $bushels, acres: $acres, price: $landPrice, yield: $harvestPerAcre, rats: $ratsEaten, starvedLast: $starvedLastYear, starvedTotal: $starvedTotal, status: $status, rngState: $rngState)';
}

// ─── Result Algebraic Type ──────────────────────────────────────────────────

sealed class Result<T, E> {
  const Result();
  bool get isOk => this is Ok<T, E>;
  bool get isErr => this is Err<T, E>;
  T get unwrap;
  E get unwrapErr;
}

final class Ok<T, E> extends Result<T, E> {
  final T value;
  const Ok(this.value);
  @override
  T get unwrap => value;
  @override
  E get unwrapErr => throw StateError('Called unwrapErr on Ok($value)');
  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is Ok<T, E> && other.value == value);
  @override
  int get hashCode => value.hashCode;
  @override
  String toString() => 'Ok($value)';
}

final class Err<T, E> extends Result<T, E> {
  final E error;
  const Err(this.error);
  @override
  T get unwrap => throw StateError('Called unwrap on Err($error)');
  @override
  E get unwrapErr => error;
  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is Err<T, E> && other.error == error);
  @override
  int get hashCode => error.hashCode;
  @override
  String toString() => 'Err($error)';
}

// ─── PRNG Internal Primitives (PCG32 XSH RR 64/32) ──────────────────────────

class _PcgStepResult {
  final int value;
  final BigInt nextState;
  const _PcgStepResult(this.value, this.nextState);
}

_PcgStepResult _pcg32Step(BigInt state, BigInt inc) {
  final oldState = state;
  final nextState = ((oldState * _pcgMultiplier) + inc).toUnsigned(64);
  final xorshifted = (((oldState >> 18) ^ oldState) >> 27) & _mask32;
  final rot = (oldState >> 59).toInt();
  final x = xorshifted.toInt();
  final value = rot == 0 ? x : (((x >>> rot) | (x << (32 - rot))) & 0xFFFFFFFF);
  return _PcgStepResult(value, nextState);
}

_PcgStepResult _pcg32NextIntInRange(BigInt state, BigInt inc, int min, int max) {
  final span = max - min + 1;
  final threshold = ((1 << 32) - span) % span;
  var currentState = state;
  while (true) {
    final step = _pcg32Step(currentState, inc);
    currentState = step.nextState;
    final r = step.value;
    if (r >= threshold) {
      return _PcgStepResult(min + (r % span), currentState);
    }
  }
}

// ─── Simulation Engine API ──────────────────────────────────────────────────

/// Initializes a new, deterministic 10-year reign for ruler Hamurabi.
HamurabiState newGame(BigInt seed) {
  final unsignedSeed = seed.toUnsigned(64);
  final inc = BigInt.from((_streamId << 1) | 1); // 109
  var rngState = BigInt.zero;
  rngState = ((rngState * _pcgMultiplier) + inc).toUnsigned(64);
  rngState = (rngState + unsignedSeed).toUnsigned(64);
  rngState = ((rngState * _pcgMultiplier) + inc).toUnsigned(64);

  final step = _pcg32NextIntInRange(rngState, inc, 17, 26);
  final landPrice = step.value;
  rngState = step.nextState;

  return HamurabiState(
    year: 1,
    population: 100,
    bushels: 2800,
    acres: 1000,
    landPrice: landPrice,
    harvestPerAcre: 3,
    ratsEaten: 200,
    starvedLastYear: 0,
    starvedTotal: 0,
    percentStarvedSum: 0,
    immigrantsLastYear: 5,
    plagueStruck: false,
    status: const GameStatus.playing(),
    rngState: rngState,
    rngInc: inc,
  );
}

/// Convenience overload of [newGame] taking an integer seed.
HamurabiState newGameFromInt(int seed) => newGame(BigInt.from(seed).toUnsigned(64));

/// Validates player orders against current state constraints without advancing state.
///
/// Evaluates checks in canonical order:
/// 1. Status check (GameStatus.playing)
/// 2. Bounds and overflow checks
/// 3. Land trade resource availability
/// 4. Food allocation feasibility
/// 5. Planting acreage and labor limits
/// 6. Seed grain availability
HamurabiError? validateOrder(HamurabiState state, TurnOrder order) {
  // 1. Status Check
  if (state.status is! Playing) {
    return HamurabiError.gameAlreadyEnded;
  }

  // State invariant checks (VER04m state_inv & CWE-400 / CWE-835)
  if ((state.rngInc & BigInt.one) != BigInt.one) {
    return HamurabiError.invalidOrderBounds;
  }
  if (state.year < 1 || state.year > 10) {
    return HamurabiError.invalidOrderBounds;
  }
  if (state.landPrice < 17 || state.landPrice > 26) {
    return HamurabiError.invalidOrderBounds;
  }
  if (state.harvestPerAcre < 1 || state.harvestPerAcre > 5) {
    return HamurabiError.invalidOrderBounds;
  }

  // Hostile bounds checks (CWE-190 / CWE-681)
  if (order.acresDelta < -2147483648 || order.acresDelta > 2147483647) {
    return HamurabiError.invalidOrderBounds;
  }
  if (order.bushelsToFeed < 0 || order.bushelsToFeed > 0xFFFFFFFF) {
    return HamurabiError.invalidOrderBounds;
  }
  if (order.acresToPlant < 0 || order.acresToPlant > 0xFFFFFFFF) {
    return HamurabiError.invalidOrderBounds;
  }
  if (state.population < 0 || state.population > 0xFFFFFFFF) {
    return HamurabiError.invalidOrderBounds;
  }
  if (state.bushels < 0 || state.bushels > 0xFFFFFFFF) {
    return HamurabiError.invalidOrderBounds;
  }
  if (state.acres < 0 || state.acres > 0xFFFFFFFF) {
    return HamurabiError.invalidOrderBounds;
  }

  // 2. Bounds & Overflow Guard on Land Trade
  if (order.acresDelta > 0) {
    final buyCount = order.acresDelta;
    final cost = buyCount * state.landPrice;
    if (cost > state.bushels) {
      return HamurabiError.insufficientBushelsForLand;
    }
  } else if (order.acresDelta < 0) {
    final sellCount = -order.acresDelta;
    if (sellCount > state.acres) {
      return HamurabiError.insufficientAcresToSell;
    }
  }

  // 3. Land Trade Balances
  int acresAfterTrade;
  int bushelsAfterTrade;
  if (order.acresDelta > 0) {
    acresAfterTrade = state.acres + order.acresDelta;
    if (acresAfterTrade > 0xFFFFFFFF) {
      return HamurabiError.invalidOrderBounds;
    }
    final cost = order.acresDelta * state.landPrice;
    bushelsAfterTrade = state.bushels - cost;
  } else if (order.acresDelta < 0) {
    final sellCount = -order.acresDelta;
    acresAfterTrade = state.acres - sellCount;
    final revenue = sellCount * state.landPrice;
    bushelsAfterTrade = state.bushels + revenue;
    if (bushelsAfterTrade > 0xFFFFFFFF) {
      return HamurabiError.invalidOrderBounds;
    }
  } else {
    acresAfterTrade = state.acres;
    bushelsAfterTrade = state.bushels;
  }

  // 4. Food Allocation Check
  if (order.bushelsToFeed > bushelsAfterTrade) {
    return HamurabiError.insufficientBushelsToFeed;
  }
  final bushelsAfterFeed = bushelsAfterTrade - order.bushelsToFeed;

  // 5. Planting Land & Labor Checks
  if (order.acresToPlant > acresAfterTrade) {
    return HamurabiError.insufficientAcresToPlant;
  }
  final maxLaborAcres = state.population * 10;
  if (maxLaborAcres > 0xFFFFFFFF) {
    return HamurabiError.invalidOrderBounds;
  }
  if (order.acresToPlant > maxLaborAcres) {
    return HamurabiError.insufficientPopulationToPlant;
  }

  // 6. Planting Seed Check: each bushel seeds 2 acres (ceil(acres / 2))
  final seedNeeded = (order.acresToPlant + 1) ~/ 2;
  if (seedNeeded > bushelsAfterFeed) {
    return HamurabiError.insufficientGrainForSeed;
  }
  final bushelsAfterSeed = bushelsAfterFeed - seedNeeded;

  // 7. Maximum Harvest Overflow Guard (CWE-190)
  final maxHarvest = order.acresToPlant * 5;
  final maxStored = bushelsAfterSeed + maxHarvest;
  if (maxStored > 0xFFFFFFFF) {
    return HamurabiError.invalidOrderBounds;
  }

  return null;
}

/// Advances the simulation by exactly one year according to player orders.
Result<HamurabiState, HamurabiError> stepYear(HamurabiState state, TurnOrder order) {
  final err = validateOrder(state, order);
  if (err != null) {
    return Err(err);
  }

  var pcgState = state.rngState;
  final pcgInc = state.rngInc;

  final int acresAfterTrade;
  final int bushelsAfterTrade;
  if (order.acresDelta > 0) {
    acresAfterTrade = state.acres + order.acresDelta;
    final cost = order.acresDelta * state.landPrice;
    bushelsAfterTrade = state.bushels - cost;
  } else if (order.acresDelta < 0) {
    final sellCount = -order.acresDelta;
    acresAfterTrade = state.acres - sellCount;
    final revenue = sellCount * state.landPrice;
    bushelsAfterTrade = state.bushels + revenue;
  } else {
    acresAfterTrade = state.acres;
    bushelsAfterTrade = state.bushels;
  }

  final bushelsAfterFeed = bushelsAfterTrade - order.bushelsToFeed;
  final seedNeeded = (order.acresToPlant + 1) ~/ 2;
  final bushelsAfterSeed = bushelsAfterFeed - seedNeeded;

  // Step 1: Harvest Yield Draw [1, 5]
  var step = _pcg32NextIntInRange(pcgState, pcgInc, 1, 5);
  final harvestPerAcre = step.value;
  pcgState = step.nextState;

  final totalHarvest = order.acresToPlant * harvestPerAcre;
  final bushelsStored = bushelsAfterSeed + totalHarvest;

  // Step 2: Rat Infestation Draws [1, 100]
  step = _pcg32NextIntInRange(pcgState, pcgInc, 1, 100);
  final ratRoll = step.value;
  pcgState = step.nextState;

  final int ratsEaten;
  if (ratRoll <= 40) {
    step = _pcg32NextIntInRange(pcgState, pcgInc, 1, 5);
    final ratFactor = step.value;
    pcgState = step.nextState;
    final eaten = bushelsStored ~/ ratFactor;
    ratsEaten = eaten > 0xFFFFFFFF ? 0xFFFFFFFF : eaten;
  } else {
    ratsEaten = 0;
  }

  final int bushelsRemaining = bushelsStored > ratsEaten ? (bushelsStored - ratsEaten) : 0;
  final int bushelsFinal = bushelsRemaining > 0xFFFFFFFF ? 0xFFFFFFFF : bushelsRemaining;

  // Step 3: Plague Draw [1, 100]
  step = _pcg32NextIntInRange(pcgState, pcgInc, 1, 100);
  final plagueRoll = step.value;
  pcgState = step.nextState;
  final plagueStruck = plagueRoll <= 15;

  // Step 4: Starvation & Impeachment Evaluation
  final peopleFed = order.bushelsToFeed ~/ 20;
  final int starved;
  int survivors;
  if (peopleFed >= state.population) {
    starved = 0;
    survivors = state.population;
  } else {
    starved = state.population - peopleFed;
    survivors = peopleFed;
  }

  final int percentStarved = state.population > 0
      ? ((starved * 100) ~/ state.population).clamp(0, 100)
      : 0;

  final newStarvedTotal = (state.starvedTotal + starved).clamp(0, 0xFFFFFFFF);
  final newPercentSum = (state.percentStarvedSum + percentStarved).clamp(0, 0xFFFFFFFF);

  GameStatus newStatus;
  if (starved * 100 > state.population * 45) {
    newStatus = GameStatus.impeached(starved, percentStarved);
  } else if (survivors == 0) {
    newStatus = const GameStatus.gameOverExtinction();
  } else {
    newStatus = const GameStatus.playing();
  }

  // Step 5: Plague Resolution
  if (plagueStruck && newStatus is Playing) {
    survivors ~/= 2;
  }

  // Step 6: Immigration Draw [1, 5]
  step = _pcg32NextIntInRange(pcgState, pcgInc, 1, 5);
  final immFactor = step.value;
  pcgState = step.nextState;

  final int immigrants;
  if (starved == 0 && newStatus is Playing) {
    final acresTerm = acresAfterTrade * 20;
    final inner = acresTerm + bushelsFinal;
    final numerator = immFactor * inner;
    final denominator = 100 * state.population;
    if (denominator == 0) {
      immigrants = 0;
    } else {
      final q = numerator ~/ denominator;
      immigrants = (q + 1).clamp(0, 0xFFFFFFFF);
    }
  } else {
    immigrants = 0;
  }

  final finalPopulation = (survivors + immigrants).clamp(0, 0xFFFFFFFF);

  // Step 7: Next Year Land Price Draw [17, 26]
  step = _pcg32NextIntInRange(pcgState, pcgInc, 17, 26);
  final nextLandPrice = step.value;
  pcgState = step.nextState;

  // Step 8: Turn Advance & Rating
  final nextYear = (state.year + 1).clamp(0, 255);
  if (nextYear == 11 && newStatus is Playing) {
    final avgStarved = newPercentSum ~/ 10;
    final acresPerPerson = finalPopulation == 0 ? 0 : (acresAfterTrade ~/ finalPopulation);
    final Rating rating;
    if (avgStarved <= 3 && acresPerPerson >= 10) {
      rating = Rating.charlemagne;
    } else if (avgStarved <= 10 && acresPerPerson >= 9) {
      rating = Rating.acre;
    } else if (avgStarved <= 33 && acresPerPerson >= 7) {
      rating = Rating.nero;
    } else {
      rating = Rating.disaster;
    }
    newStatus = GameStatus.won(rating);
  }

  return Ok(HamurabiState(
    year: nextYear,
    population: finalPopulation,
    bushels: bushelsFinal,
    acres: acresAfterTrade,
    landPrice: nextLandPrice,
    harvestPerAcre: harvestPerAcre,
    ratsEaten: ratsEaten,
    starvedLastYear: starved,
    starvedTotal: newStarvedTotal,
    percentStarvedSum: newPercentSum,
    immigrantsLastYear: immigrants,
    plagueStruck: plagueStruck,
    status: newStatus,
    rngState: pcgState,
    rngInc: pcgInc,
  ));
}

/// Formats the annual teletype report or final evaluation in authentic 1970s teletype styling.
String formatReport(HamurabiState state) {
  final status = state.status;
  if (status is Impeached) {
    return 'YOU STARVED ${status.starvedThisTurn} PEOPLE IN ONE YEAR!!!\n'
        'DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n';
  } else if (status is GameOverExtinction) {
    return 'EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n';
  } else if (status is Won) {
    final avgStarved = state.percentStarvedSum ~/ 10;
    final acresPerPerson =
        state.population == 0 ? 0 : (state.acres ~/ state.population);
    final String ratingMsg;
    switch (status.rating) {
      case Rating.charlemagne:
        ratingMsg =
            'A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!';
      case Rating.acre:
        ratingMsg =
            'YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE.';
      case Rating.nero:
        ratingMsg =
            'YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU.';
      case Rating.disaster:
        ratingMsg =
            'YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!';
    }
    return 'IN YOUR 10-YEAR TERM OF OFFICE, $avgStarved PERCENT OF THE\n'
        'POPULATION STARVED PER YEAR ON AVERAGE.\n'
        'A TOTAL OF ${state.starvedTotal} PEOPLE DIED!!\n'
        'YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n'
        '$acresPerPerson ACRES PER PERSON.\n\n'
        '$ratingMsg\n';
  } else {
    final buffer = StringBuffer();
    buffer.write('HAMURABI: I BEG TO REPORT TO YOU,\n'
        'IN YEAR ${state.year}, ${state.starvedLastYear} PEOPLE STARVED, ${state.immigrantsLastYear} CAME TO THE CITY.\n');
    if (state.plagueStruck) {
      buffer.write('A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n');
    }
    buffer.write('POPULATION IS NOW ${state.population}.\n'
        'THE CITY NOW OWNS ${state.acres} ACRES.\n'
        'YOU HARVESTED ${state.harvestPerAcre} BUSHELS PER ACRE.\n'
        'RATS ATE ${state.ratsEaten} BUSHELS.\n'
        'YOU NOW HAVE ${state.bushels} BUSHELS IN STORE.\n'
        'LAND IS TRADING AT ${state.landPrice} BUSHELS PER ACRE.\n');
    return buffer.toString();
  }
}
