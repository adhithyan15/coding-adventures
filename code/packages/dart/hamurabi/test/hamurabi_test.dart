// hamurabi_test.dart — Unit and security tests for Hamurabi
import 'package:coding_adventures_hamurabi/coding_adventures_hamurabi.dart';
import 'package:test/test.dart';

void main() {
  group('Hamurabi Unit & Security Tests', () {
    test('initial state invariants (newGame)', () {
      final state = newGameFromInt(1337);
      expect(state.year, equals(1));
      expect(state.population, equals(100));
      expect(state.bushels, equals(2800));
      expect(state.acres, equals(1000));
      expect(state.landPrice, inInclusiveRange(17, 26));
      expect(state.harvestPerAcre, equals(3));
      expect(state.ratsEaten, equals(200));
      expect(state.starvedLastYear, equals(0));
      expect(state.starvedTotal, equals(0));
      expect(state.percentStarvedSum, equals(0));
      expect(state.immigrantsLastYear, equals(5));
      expect(state.plagueStruck, isFalse);
      expect(state.status, equals(const GameStatus.playing()));
      expect((state.rngInc & BigInt.one), equals(BigInt.one));
    });

    test('all eight error arms', () {
      final state = newGameFromInt(1337);
      const normalOrder = TurnOrder(acresDelta: 0, bushelsToFeed: 2000, acresToPlant: 100);

      // 1. GameAlreadyEnded
      final wonState = HamurabiState(
        year: 11,
        population: state.population,
        bushels: state.bushels,
        acres: state.acres,
        landPrice: state.landPrice,
        harvestPerAcre: state.harvestPerAcre,
        ratsEaten: state.ratsEaten,
        starvedLastYear: state.starvedLastYear,
        starvedTotal: state.starvedTotal,
        percentStarvedSum: state.percentStarvedSum,
        immigrantsLastYear: state.immigrantsLastYear,
        plagueStruck: state.plagueStruck,
        status: const GameStatus.won(Rating.charlemagne),
        rngState: state.rngState,
        rngInc: state.rngInc,
      );
      expect(validateOrder(wonState, normalOrder), equals(HamurabiError.gameAlreadyEnded));
      expect(stepYear(wonState, normalOrder).unwrapErr, equals(HamurabiError.gameAlreadyEnded));

      // 2. InvalidOrderBounds (corrupted land price or overflow)
      final badPriceState = HamurabiState(
        year: state.year,
        population: state.population,
        bushels: state.bushels,
        acres: state.acres,
        landPrice: 10, // valid is 17..=26
        harvestPerAcre: state.harvestPerAcre,
        ratsEaten: state.ratsEaten,
        starvedLastYear: state.starvedLastYear,
        starvedTotal: state.starvedTotal,
        percentStarvedSum: state.percentStarvedSum,
        immigrantsLastYear: state.immigrantsLastYear,
        plagueStruck: state.plagueStruck,
        status: state.status,
        rngState: state.rngState,
        rngInc: state.rngInc,
      );
      expect(validateOrder(badPriceState, normalOrder), equals(HamurabiError.invalidOrderBounds));

      // 3. InsufficientBushelsForLand
      const buyTooMuch = TurnOrder(acresDelta: 200, bushelsToFeed: 0, acresToPlant: 0);
      expect(validateOrder(state, buyTooMuch), equals(HamurabiError.insufficientBushelsForLand));

      // 4. InsufficientAcresToSell
      const sellTooMuch = TurnOrder(acresDelta: -1001, bushelsToFeed: 0, acresToPlant: 0);
      expect(validateOrder(state, sellTooMuch), equals(HamurabiError.insufficientAcresToSell));

      // 5. InsufficientBushelsToFeed
      const feedTooMuch = TurnOrder(acresDelta: 0, bushelsToFeed: 2801, acresToPlant: 0);
      expect(validateOrder(state, feedTooMuch), equals(HamurabiError.insufficientBushelsToFeed));

      // 6. InsufficientAcresToPlant
      const plantTooMuchAcres = TurnOrder(acresDelta: 0, bushelsToFeed: 1000, acresToPlant: 1001);
      expect(validateOrder(state, plantTooMuchAcres), equals(HamurabiError.insufficientAcresToPlant));

      // 7. InsufficientPopulationToPlant (1 person tends 10 acres)
      final smallPopState = HamurabiState(
        year: state.year,
        population: 50, // can only tend 500
        bushels: state.bushels,
        acres: state.acres,
        landPrice: state.landPrice,
        harvestPerAcre: state.harvestPerAcre,
        ratsEaten: state.ratsEaten,
        starvedLastYear: state.starvedLastYear,
        starvedTotal: state.starvedTotal,
        percentStarvedSum: state.percentStarvedSum,
        immigrantsLastYear: state.immigrantsLastYear,
        plagueStruck: state.plagueStruck,
        status: state.status,
        rngState: state.rngState,
        rngInc: state.rngInc,
      );
      const plantTooMuchPop = TurnOrder(acresDelta: 0, bushelsToFeed: 1000, acresToPlant: 501);
      expect(validateOrder(smallPopState, plantTooMuchPop), equals(HamurabiError.insufficientPopulationToPlant));

      // 8. InsufficientGrainForSeed (1 bushel seeds 2 acres)
      // 800 acres needs 400 bushels seed. Bushels left after feed: 2800 - 2500 = 300.
      const seedStarvedOrder = TurnOrder(acresDelta: 0, bushelsToFeed: 2500, acresToPlant: 800);
      expect(validateOrder(state, seedStarvedOrder), equals(HamurabiError.insufficientGrainForSeed));
    });

    test('CWE-190 overflow guards and bounds', () {
      final state = newGameFromInt(42);

      // Huge buy
      const hugeBuy = TurnOrder(acresDelta: 2147483647, bushelsToFeed: 0, acresToPlant: 0);
      expect(validateOrder(state, hugeBuy), equals(HamurabiError.insufficientBushelsForLand));

      // Int32.min negation guard: acresDelta = -2147483648
      const int32MinSell = TurnOrder(acresDelta: -2147483648, bushelsToFeed: 0, acresToPlant: 0);
      expect(validateOrder(state, int32MinSell), equals(HamurabiError.insufficientAcresToSell));

      // Huge plant
      const hugePlant = TurnOrder(acresDelta: 0, bushelsToFeed: 0, acresToPlant: 4294967295);
      expect(validateOrder(state, hugePlant), equals(HamurabiError.insufficientAcresToPlant));

      // Max harvest overflow guard (900M acres * 5 > 4.29B)
      final bigState = HamurabiState(
        year: 1,
        population: 500000000,
        bushels: 2000000000,
        acres: 1000000000,
        landPrice: 20,
        harvestPerAcre: 3,
        ratsEaten: 0,
        starvedLastYear: 0,
        starvedTotal: 0,
        percentStarvedSum: 0,
        immigrantsLastYear: 0,
        plagueStruck: false,
        status: const GameStatus.playing(),
        rngState: state.rngState,
        rngInc: state.rngInc,
      );
      const harvestOverflowOrder = TurnOrder(acresDelta: 0, bushelsToFeed: 0, acresToPlant: 900000000);
      expect(validateOrder(bigState, harvestOverflowOrder), equals(HamurabiError.invalidOrderBounds));
    });

    test('adversarial state invariants refusal', () {
      final validOrder = const TurnOrder(acresDelta: 0, bushelsToFeed: 2000, acresToPlant: 1000);

      // Even rng_inc
      final evenIncState = HamurabiState(
        year: 1,
        population: 100,
        bushels: 2800,
        acres: 1000,
        landPrice: 20,
        harvestPerAcre: 3,
        ratsEaten: 0,
        starvedLastYear: 0,
        starvedTotal: 0,
        percentStarvedSum: 0,
        immigrantsLastYear: 0,
        plagueStruck: false,
        status: const GameStatus.playing(),
        rngState: BigInt.one,
        rngInc: BigInt.from(2), // even!
      );
      expect(validateOrder(evenIncState, validOrder), equals(HamurabiError.invalidOrderBounds));

      // Year bounds [1, 10]
      final year0 = HamurabiState(
        year: 0,
        population: 100,
        bushels: 2800,
        acres: 1000,
        landPrice: 20,
        harvestPerAcre: 3,
        ratsEaten: 0,
        starvedLastYear: 0,
        starvedTotal: 0,
        percentStarvedSum: 0,
        immigrantsLastYear: 0,
        plagueStruck: false,
        status: const GameStatus.playing(),
        rngState: BigInt.one,
        rngInc: BigInt.from(109),
      );
      expect(validateOrder(year0, validOrder), equals(HamurabiError.invalidOrderBounds));
    });

    test('impeachment and teletype report format', () {
      final state = newGameFromInt(100);
      const starveOrder = TurnOrder(acresDelta: 0, bushelsToFeed: 0, acresToPlant: 0);
      final nextState = stepYear(state, starveOrder).unwrap;

      expect(nextState.status, isA<Impeached>());
      final imp = nextState.status as Impeached;
      expect(imp.starvedThisTurn, equals(100));
      expect(imp.percent, equals(100));

      final report = formatReport(nextState);
      expect(report, contains('YOU STARVED 100 PEOPLE IN ONE YEAR!!!'));
      expect(report, contains('IMPEACHED AND THROWN OUT OF OFFICE'));
    });

    test('terminal evaluation ratings in year 11', () {
      // Charlemagne: avg starved <= 3% AND acres per person >= 10
      final wonCharlemagne = HamurabiState(
        year: 11,
        population: 100,
        bushels: 3000,
        acres: 1000,
        landPrice: 20,
        harvestPerAcre: 3,
        ratsEaten: 0,
        starvedLastYear: 0,
        starvedTotal: 0,
        percentStarvedSum: 20,
        immigrantsLastYear: 0,
        plagueStruck: false,
        status: const GameStatus.won(Rating.charlemagne),
        rngState: BigInt.one,
        rngInc: BigInt.from(109),
      );
      final report = formatReport(wonCharlemagne);
      expect(report, contains('CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!'));
    });
  });
}
