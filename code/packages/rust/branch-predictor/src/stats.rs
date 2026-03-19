/// Prediction statistics -- measuring how well a branch predictor performs.
///
/// Every branch predictor needs a scorecard. When a CPU designer evaluates a
/// predictor, the first question is always: "What's the accuracy?" A predictor
/// that's 95% accurate causes a pipeline flush on only 5% of branches, while
/// a 70% accurate predictor flushes on 30% -- potentially halving throughput
/// on a deeply pipelined machine.
///
/// Real-world context:
/// - Intel's Pentium Pro achieved ~90% accuracy with a two-level adaptive predictor
/// - Modern CPUs (since ~2015) achieve 95-99% accuracy using TAGE or perceptron predictors
/// - Even a 1% improvement in accuracy can yield measurable speedups on branch-heavy code

/// Tracks prediction accuracy for a branch predictor.
///
/// # Example
/// ```
/// use branch_predictor::PredictionStats;
///
/// let mut stats = PredictionStats::new();
/// stats.record(true);   // predictor got it right
/// stats.record(true);   // right again
/// stats.record(false);  // wrong this time
/// assert!((stats.accuracy() - 66.666_666_666_666_66).abs() < 0.01);
/// ```
///
/// The stats object is usually owned by a predictor and exposed via its
/// `stats()` method. The CPU core never creates PredictionStats directly --
/// it just reads the predictor's stats after running a benchmark.
#[derive(Debug, Clone, Default)]
pub struct PredictionStats {
    /// Total number of predictions made.
    pub predictions: u64,
    /// Number of correct predictions.
    pub correct: u64,
    /// Number of incorrect predictions (mispredictions).
    pub incorrect: u64,
}

impl PredictionStats {
    /// Create a new, zeroed-out statistics tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Prediction accuracy as a percentage (0.0 to 100.0).
    ///
    /// Returns 0.0 if no predictions have been made yet, because we can't
    /// divide by zero, and "no data" is semantically closer to "0% accurate"
    /// than "100% accurate" in a benchmarking context.
    pub fn accuracy(&self) -> f64 {
        if self.predictions == 0 {
            return 0.0;
        }
        (self.correct as f64 / self.predictions as f64) * 100.0
    }

    /// Misprediction rate as a percentage (0.0 to 100.0).
    ///
    /// This is the complement of accuracy: `misprediction_rate = 100 - accuracy`.
    /// CPU architects often think in terms of misprediction rate because each
    /// misprediction causes a pipeline flush -- a concrete, measurable cost.
    pub fn misprediction_rate(&self) -> f64 {
        if self.predictions == 0 {
            return 0.0;
        }
        (self.incorrect as f64 / self.predictions as f64) * 100.0
    }

    /// Record the outcome of a single prediction.
    ///
    /// # Arguments
    /// * `correct` - True if the predictor guessed correctly, false otherwise.
    pub fn record(&mut self, correct: bool) {
        self.predictions += 1;
        if correct {
            self.correct += 1;
        } else {
            self.incorrect += 1;
        }
    }

    /// Reset all counters to zero.
    ///
    /// Called when starting a new benchmark or program execution. Without
    /// this, stats from a previous run would contaminate the new measurement.
    pub fn reset(&mut self) {
        self.predictions = 0;
        self.correct = 0;
        self.incorrect = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_stats_are_zero() {
        let stats = PredictionStats::new();
        assert_eq!(stats.predictions, 0);
        assert_eq!(stats.correct, 0);
        assert_eq!(stats.incorrect, 0);
        assert!((stats.accuracy() - 0.0).abs() < f64::EPSILON);
        assert!((stats.misprediction_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_record_correct() {
        let mut stats = PredictionStats::new();
        stats.record(true);
        assert_eq!(stats.predictions, 1);
        assert_eq!(stats.correct, 1);
        assert_eq!(stats.incorrect, 0);
        assert!((stats.accuracy() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_record_incorrect() {
        let mut stats = PredictionStats::new();
        stats.record(false);
        assert_eq!(stats.predictions, 1);
        assert_eq!(stats.correct, 0);
        assert_eq!(stats.incorrect, 1);
        assert!((stats.misprediction_rate() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_mixed_predictions() {
        let mut stats = PredictionStats::new();
        stats.record(true);
        stats.record(true);
        stats.record(false);
        assert_eq!(stats.predictions, 3);
        assert!((stats.accuracy() - 66.666_666_666_666_66).abs() < 0.01);
        assert!((stats.misprediction_rate() - 33.333_333_333_333_33).abs() < 0.01);
    }

    #[test]
    fn test_reset() {
        let mut stats = PredictionStats::new();
        stats.record(true);
        stats.record(false);
        stats.reset();
        assert_eq!(stats.predictions, 0);
        assert_eq!(stats.correct, 0);
        assert_eq!(stats.incorrect, 0);
        assert!((stats.accuracy() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_perfect_accuracy() {
        let mut stats = PredictionStats::new();
        for _ in 0..100 {
            stats.record(true);
        }
        assert!((stats.accuracy() - 100.0).abs() < f64::EPSILON);
        assert!((stats.misprediction_rate() - 0.0).abs() < f64::EPSILON);
    }
}
