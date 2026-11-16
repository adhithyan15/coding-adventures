# CodingAdventures::Hamurabi (Perl)

Pure, deterministic simulation core for the seminal 1968 mainframe resource allocation game in pure Perl.

## Historical Lineage

- **1964 — The Sumerian Game:** Designed by Mabel Addis (the first credited female video game designer and writer) and programmed by William McKay for an IBM 7090 mainframe at the Board of Cooperative Educational Services (BOCES) in Westchester County, New York. It introduced the economic feedback loop: governing the city-state of Lagash through annual decisions on land acquisition, grain allocation for consumption, and agriculture.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the Digital Equipment Corporation (DEC) PDP-8 in FOCAL-69, shortening the historical Sumerian ruler name to fit the 8-character symbol constraint of PDP-8 teletype terminals.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published in DEC's landmark book, becoming the archetypal computer simulation game that influenced an entire generation of programmers.

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER27`, `VER29`):
- **Strictly zero ambient system authority:** `"capabilities": []` declared in `required_capabilities.json`. Zero filesystem, process, network, or terminal I/O within the simulation core.
- **Pure state transition:** `step_year($state, $order)` is a pure mathematical state function returning an immutable updated state record without mutating the input.
- **Explicit pseudorandomness:** All simulation draws enter deterministically through explicit PCG32 state stored directly inside the state record (`HAMURABI_STREAM_ID = 54`, `inc = 109`).
- **Exact integer arithmetic:** Uses bounded integer arithmetic with `Math::BigInt` promotion to ensure 100% bit-exact cross-language alignment without floating point drift.
- **Zero heap allocation on hot path:** Constant-bounded execution time (`steps <= 256`), eliminating algorithmic complexity risks.
- **Complete 26-CWE weakness closure:** Formally certified against CWE-190, CWE-843, CWE-835, CWE-1333, CWE-78, CWE-94, CWE-502, and all remaining weakness classes.

## API Reference

```perl
use CodingAdventures::Hamurabi qw(new_game validate_order step_year format_report);

# Initialize a deterministic 10-year reign with seed (e.g. 1337)
my $state = new_game(1337);

# Define player orders for the year
my $order = {
    acres_delta     => 0,      # Buy (+) or sell (-) acres of land
    bushels_to_feed => 2000,   # Bushels of grain allocated to feed citizens (20/person)
    acres_to_plant  => 800,    # Acres of land to sow with grain (1 bushel seeds 2 acres)
};

# Validate orders without advancing simulation (throws HamurabiError on failure)
validate_order($state, $order);

# Advance the simulation by exactly one year
my $next_state = step_year($state, $order);

# Format authentic 1970s teletype report
my $report = format_report($next_state);
print $report;
```

### Error Handling

When orders cannot be fulfilled or violate game rules, `validate_order` and `step_year` throw a `CodingAdventures::Hamurabi::HamurabiError` exception object:

```perl
use CodingAdventures::Hamurabi qw(step_year);

eval {
    my $next = step_year($state, $order);
};
if (my $err = $@) {
    print "Refused: ", $err->kind(), "\n";
}
```

The 8 canonical error kinds:
1. `GameAlreadyEnded`
2. `InvalidOrderBounds`
3. `InsufficientBushelsForLand`
4. `InsufficientAcresToSell`
5. `InsufficientBushelsToFeed`
6. `InsufficientAcresToPlant`
7. `InsufficientPopulationToPlant`
8. `InsufficientGrainForSeed`

## Testing

```shell
prove -l -v t/
```
