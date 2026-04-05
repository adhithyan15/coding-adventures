package stats

import (
	"math"
	"testing"
)

// approxEqual checks if two floats are approximately equal within epsilon.
func approxEqual(a, b, epsilon float64) bool {
	return math.Abs(a-b) < epsilon
}

const eps = 1e-10

// ── Mean ───────────────────────────────────────────────────────────────

func TestMeanParity(t *testing.T) {
	// ST01 parity: mean([1,2,3,4,5]) -> 3.0
	got := Mean([]float64{1, 2, 3, 4, 5})
	if got != 3.0 {
		t.Errorf("Mean([1,2,3,4,5]) = %v, want 3.0", got)
	}
}

func TestMeanSingle(t *testing.T) {
	got := Mean([]float64{42.0})
	if got != 42.0 {
		t.Errorf("Mean([42]) = %v, want 42.0", got)
	}
}

func TestMeanNegative(t *testing.T) {
	got := Mean([]float64{-3, -1, 0, 1, 3})
	if got != 0.0 {
		t.Errorf("Mean([-3,-1,0,1,3]) = %v, want 0.0", got)
	}
}

func TestMeanEmptyPanics(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("Mean([]) did not panic")
		}
	}()
	Mean([]float64{})
}

// ── Median ─────────────────────────────────────────────────────────────

func TestMedianOdd(t *testing.T) {
	got := Median([]float64{1, 2, 3, 4, 5})
	if got != 3.0 {
		t.Errorf("Median([1,2,3,4,5]) = %v, want 3.0", got)
	}
}

func TestMedianEven(t *testing.T) {
	got := Median([]float64{1, 2, 3, 4})
	if got != 2.5 {
		t.Errorf("Median([1,2,3,4]) = %v, want 2.5", got)
	}
}

func TestMedianUnsorted(t *testing.T) {
	got := Median([]float64{5, 1, 3, 2, 4})
	if got != 3.0 {
		t.Errorf("Median([5,1,3,2,4]) = %v, want 3.0", got)
	}
}

func TestMedianSingle(t *testing.T) {
	got := Median([]float64{7.0})
	if got != 7.0 {
		t.Errorf("Median([7]) = %v, want 7.0", got)
	}
}

func TestMedianEmptyPanics(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("Median([]) did not panic")
		}
	}()
	Median([]float64{})
}

// ── Mode ───────────────────────────────────────────────────────────────

func TestModeParity(t *testing.T) {
	got := Mode([]float64{1, 2, 2, 3})
	if got != 2.0 {
		t.Errorf("Mode([1,2,2,3]) = %v, want 2.0", got)
	}
}

func TestModeTieFirstWins(t *testing.T) {
	got := Mode([]float64{1, 3, 1, 3})
	if got != 1.0 {
		t.Errorf("Mode([1,3,1,3]) = %v, want 1.0 (first wins)", got)
	}
}

func TestModeSingle(t *testing.T) {
	got := Mode([]float64{5.0})
	if got != 5.0 {
		t.Errorf("Mode([5]) = %v, want 5.0", got)
	}
}

func TestModeEmptyPanics(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("Mode([]) did not panic")
		}
	}()
	Mode([]float64{})
}

// ── Variance ───────────────────────────────────────────────────────────

func TestVarianceSampleParity(t *testing.T) {
	got := Variance([]float64{2, 4, 4, 4, 5, 5, 7, 9}, false)
	want := 4.571428571428571
	if !approxEqual(got, want, eps) {
		t.Errorf("Variance sample = %v, want %v", got, want)
	}
}

func TestVariancePopulationParity(t *testing.T) {
	got := Variance([]float64{2, 4, 4, 4, 5, 5, 7, 9}, true)
	if !approxEqual(got, 4.0, eps) {
		t.Errorf("Variance population = %v, want 4.0", got)
	}
}

func TestVarianceZero(t *testing.T) {
	got := Variance([]float64{5, 5, 5, 5}, true)
	if got != 0.0 {
		t.Errorf("Variance([5,5,5,5]) = %v, want 0.0", got)
	}
}

func TestVarianceSinglePopulation(t *testing.T) {
	got := Variance([]float64{42.0}, true)
	if got != 0.0 {
		t.Errorf("Variance([42], population) = %v, want 0.0", got)
	}
}

func TestVarianceSingleSamplePanics(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("Variance([42], sample) did not panic")
		}
	}()
	Variance([]float64{42.0}, false)
}

func TestVarianceEmptyPanics(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("Variance([], sample) did not panic")
		}
	}()
	Variance([]float64{}, false)
}

// ── Standard Deviation ─────────────────────────────────────────────────

func TestStandardDeviationSample(t *testing.T) {
	got := StandardDeviation([]float64{2, 4, 4, 4, 5, 5, 7, 9}, false)
	want := math.Sqrt(4.571428571428571)
	if !approxEqual(got, want, eps) {
		t.Errorf("StdDev sample = %v, want %v", got, want)
	}
}

func TestStandardDeviationPopulation(t *testing.T) {
	got := StandardDeviation([]float64{2, 4, 4, 4, 5, 5, 7, 9}, true)
	if !approxEqual(got, 2.0, eps) {
		t.Errorf("StdDev population = %v, want 2.0", got)
	}
}

// ── Min / Max / Range ──────────────────────────────────────────────────

func TestMin(t *testing.T) {
	got := Min([]float64{3, 1, 4, 1, 5})
	if got != 1.0 {
		t.Errorf("Min = %v, want 1.0", got)
	}
}

func TestMax(t *testing.T) {
	got := Max([]float64{3, 1, 4, 1, 5})
	if got != 5.0 {
		t.Errorf("Max = %v, want 5.0", got)
	}
}

func TestRange(t *testing.T) {
	got := Range([]float64{2, 4, 4, 4, 5, 5, 7, 9})
	if got != 7.0 {
		t.Errorf("Range = %v, want 7.0", got)
	}
}

func TestRangeSingle(t *testing.T) {
	got := Range([]float64{5.0})
	if got != 0.0 {
		t.Errorf("Range([5]) = %v, want 0.0", got)
	}
}

func TestMinNegative(t *testing.T) {
	got := Min([]float64{-5, -1, 0, 3})
	if got != -5.0 {
		t.Errorf("Min([-5,-1,0,3]) = %v, want -5.0", got)
	}
}

func TestMaxNegative(t *testing.T) {
	got := Max([]float64{-5, -1, 0, 3})
	if got != 3.0 {
		t.Errorf("Max([-5,-1,0,3]) = %v, want 3.0", got)
	}
}

func TestMinEmptyPanics(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("Min([]) did not panic")
		}
	}()
	Min([]float64{})
}

func TestMaxEmptyPanics(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("Max([]) did not panic")
		}
	}()
	Max([]float64{})
}

// ── Frequency Count ────────────────────────────────────────────────────

func TestFrequencyCountParity(t *testing.T) {
	got := FrequencyCount("Hello")
	expected := map[string]int{"H": 1, "E": 1, "L": 2, "O": 1}
	for k, v := range expected {
		if got[k] != v {
			t.Errorf("FrequencyCount('Hello')[%s] = %d, want %d", k, got[k], v)
		}
	}
	if len(got) != len(expected) {
		t.Errorf("FrequencyCount('Hello') has %d entries, want %d", len(got), len(expected))
	}
}

func TestFrequencyCountCaseInsensitive(t *testing.T) {
	got := FrequencyCount("AaA")
	if got["A"] != 3 {
		t.Errorf("FrequencyCount('AaA')[A] = %d, want 3", got["A"])
	}
}

func TestFrequencyCountIgnoresNonAlpha(t *testing.T) {
	got := FrequencyCount("A1 B! C?")
	if got["A"] != 1 || got["B"] != 1 || got["C"] != 1 || len(got) != 3 {
		t.Errorf("FrequencyCount('A1 B! C?') = %v, unexpected", got)
	}
}

func TestFrequencyCountEmpty(t *testing.T) {
	got := FrequencyCount("")
	if len(got) != 0 {
		t.Errorf("FrequencyCount('') has %d entries, want 0", len(got))
	}
}

// ── Frequency Distribution ─────────────────────────────────────────────

func TestFrequencyDistributionUniform(t *testing.T) {
	got := FrequencyDistribution("AABB")
	if !approxEqual(got["A"], 0.5, eps) || !approxEqual(got["B"], 0.5, eps) {
		t.Errorf("FrequencyDistribution('AABB') = %v, want A:0.5 B:0.5", got)
	}
}

func TestFrequencyDistributionEmpty(t *testing.T) {
	got := FrequencyDistribution("")
	if len(got) != 0 {
		t.Errorf("FrequencyDistribution('') has %d entries, want 0", len(got))
	}
}

func TestFrequencyDistributionSumsToOne(t *testing.T) {
	got := FrequencyDistribution("HELLO WORLD")
	total := 0.0
	for _, v := range got {
		total += v
	}
	if !approxEqual(total, 1.0, 1e-6) {
		t.Errorf("FrequencyDistribution sum = %v, want 1.0", total)
	}
}

// ── Chi-Squared ────────────────────────────────────────────────────────

func TestChiSquaredParity(t *testing.T) {
	got := ChiSquared([]float64{10, 20, 30}, []float64{20, 20, 20})
	if !approxEqual(got, 10.0, eps) {
		t.Errorf("ChiSquared = %v, want 10.0", got)
	}
}

func TestChiSquaredPerfectMatch(t *testing.T) {
	got := ChiSquared([]float64{10, 20, 30}, []float64{10, 20, 30})
	if !approxEqual(got, 0.0, eps) {
		t.Errorf("ChiSquared perfect match = %v, want 0.0", got)
	}
}

func TestChiSquaredLengthMismatchPanics(t *testing.T) {
	defer func() {
		if r := recover(); r == nil {
			t.Error("ChiSquared with mismatched lengths did not panic")
		}
	}()
	ChiSquared([]float64{1, 2}, []float64{1, 2, 3})
}

func TestChiSquaredSingle(t *testing.T) {
	got := ChiSquared([]float64{5}, []float64{10})
	if !approxEqual(got, 2.5, eps) {
		t.Errorf("ChiSquared([5],[10]) = %v, want 2.5", got)
	}
}

// ── Chi-Squared Text ───────────────────────────────────────────────────

func TestChiSquaredTextEmpty(t *testing.T) {
	got := ChiSquaredText("", map[string]float64{"A": 0.5})
	if got != 0.0 {
		t.Errorf("ChiSquaredText('') = %v, want 0.0", got)
	}
}

func TestChiSquaredTextPerfect(t *testing.T) {
	got := ChiSquaredText("AAAAAAAAAA", map[string]float64{"A": 1.0})
	if !approxEqual(got, 0.0, eps) {
		t.Errorf("ChiSquaredText perfect = %v, want 0.0", got)
	}
}

func TestChiSquaredTextEnglishBetterThanRandom(t *testing.T) {
	english := "THEQUICKBROWNFOXJUMPSOVERTHELAZYDOG"
	random := "ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ"
	chiEnglish := ChiSquaredText(english, EnglishFrequencies)
	chiRandom := ChiSquaredText(random, EnglishFrequencies)
	if chiEnglish >= chiRandom {
		t.Errorf("English chi-squared (%v) should be less than random (%v)", chiEnglish, chiRandom)
	}
}

// ── Index of Coincidence ───────────────────────────────────────────────

func TestIndexOfCoincidenceParity(t *testing.T) {
	got := IndexOfCoincidence("AABB")
	want := 1.0 / 3.0
	if !approxEqual(got, want, eps) {
		t.Errorf("IC('AABB') = %v, want %v", got, want)
	}
}

func TestIndexOfCoincidenceAllSame(t *testing.T) {
	got := IndexOfCoincidence("AAAA")
	if !approxEqual(got, 1.0, eps) {
		t.Errorf("IC('AAAA') = %v, want 1.0", got)
	}
}

func TestIndexOfCoincidenceAllDifferent(t *testing.T) {
	got := IndexOfCoincidence("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
	if !approxEqual(got, 0.0, eps) {
		t.Errorf("IC(alphabet) = %v, want 0.0", got)
	}
}

func TestIndexOfCoincidenceEmpty(t *testing.T) {
	got := IndexOfCoincidence("")
	if got != 0.0 {
		t.Errorf("IC('') = %v, want 0.0", got)
	}
}

func TestIndexOfCoincidenceSingle(t *testing.T) {
	got := IndexOfCoincidence("A")
	if got != 0.0 {
		t.Errorf("IC('A') = %v, want 0.0", got)
	}
}

func TestIndexOfCoincidenceCaseInsensitive(t *testing.T) {
	a := IndexOfCoincidence("aabb")
	b := IndexOfCoincidence("AABB")
	if !approxEqual(a, b, eps) {
		t.Errorf("IC('aabb') = %v != IC('AABB') = %v", a, b)
	}
}

func TestIndexOfCoincidenceIgnoresNonAlpha(t *testing.T) {
	a := IndexOfCoincidence("A A B B")
	b := IndexOfCoincidence("AABB")
	if !approxEqual(a, b, eps) {
		t.Errorf("IC('A A B B') = %v != IC('AABB') = %v", a, b)
	}
}

func TestIndexOfCoincidenceEnglishRange(t *testing.T) {
	text := "TOBEORNOTTOBETHATISTHEQUESTION"
	got := IndexOfCoincidence(text)
	if got <= 0.0 {
		t.Errorf("IC of English text = %v, expected > 0.0", got)
	}
}

// ── Entropy ────────────────────────────────────────────────────────────

func TestEntropySingleRepeated(t *testing.T) {
	got := Entropy("AAAA")
	if !approxEqual(got, 0.0, eps) {
		t.Errorf("Entropy('AAAA') = %v, want 0.0", got)
	}
}

func TestEntropyTwoEqual(t *testing.T) {
	got := Entropy("ABABABAB")
	if !approxEqual(got, 1.0, eps) {
		t.Errorf("Entropy('ABABABAB') = %v, want 1.0", got)
	}
}

func TestEntropyUniform26(t *testing.T) {
	got := Entropy("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
	want := math.Log2(26)
	if !approxEqual(got, want, 1e-6) {
		t.Errorf("Entropy(alphabet) = %v, want %v", got, want)
	}
}

func TestEntropyEmpty(t *testing.T) {
	got := Entropy("")
	if got != 0.0 {
		t.Errorf("Entropy('') = %v, want 0.0", got)
	}
}

func TestEntropyCaseInsensitive(t *testing.T) {
	a := Entropy("aabb")
	b := Entropy("AABB")
	if !approxEqual(a, b, eps) {
		t.Errorf("Entropy('aabb') = %v != Entropy('AABB') = %v", a, b)
	}
}

func TestEntropyIncreasesWithDiversity(t *testing.T) {
	low := Entropy("AAAB")
	high := Entropy("ABCD")
	if high <= low {
		t.Errorf("Entropy('ABCD')=%v should be > Entropy('AAAB')=%v", high, low)
	}
}

// ── English Frequencies ────────────────────────────────────────────────

func TestEnglishFrequenciesHas26(t *testing.T) {
	if len(EnglishFrequencies) != 26 {
		t.Errorf("EnglishFrequencies has %d entries, want 26", len(EnglishFrequencies))
	}
}

func TestEnglishFrequenciesSumsToOne(t *testing.T) {
	total := 0.0
	for _, v := range EnglishFrequencies {
		total += v
	}
	if !approxEqual(total, 1.0, 0.001) {
		t.Errorf("EnglishFrequencies sum = %v, want ~1.0", total)
	}
}

func TestEnglishFrequenciesEMostFrequent(t *testing.T) {
	maxLetter := ""
	maxVal := 0.0
	for k, v := range EnglishFrequencies {
		if v > maxVal {
			maxVal = v
			maxLetter = k
		}
	}
	if maxLetter != "E" {
		t.Errorf("Most frequent letter = %s, want E", maxLetter)
	}
}

func TestEnglishFrequenciesSpotCheck(t *testing.T) {
	if !approxEqual(EnglishFrequencies["A"], 0.08167, 1e-5) {
		t.Errorf("EnglishFrequencies[A] = %v, want 0.08167", EnglishFrequencies["A"])
	}
	if !approxEqual(EnglishFrequencies["Z"], 0.00074, 1e-5) {
		t.Errorf("EnglishFrequencies[Z] = %v, want 0.00074", EnglishFrequencies["Z"])
	}
}
