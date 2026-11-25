import com.codingadventures.kotlin.microbenchmark.Analysis;
import com.codingadventures.kotlin.microbenchmark.Microbenchmark;
import com.codingadventures.kotlin.microbenchmark.Observation;
import java.util.List;

/** Java consumer compiled only against the installed Kotlin distribution jars. */
public final class Consumer {
    public static void main(String[] args) {
        List<Observation> pairs = List.of(
                new Observation(100, 80, "baseline-first"), new Observation(100, 80, "baseline-first"),
                new Observation(100, 80, "baseline-first"), new Observation(100, 80, "candidate-first"),
                new Observation(100, 80, "candidate-first"), new Observation(100, 80, "candidate-first"));
        Analysis result = Microbenchmark.INSTANCE.analyze(pairs, 95, 1.05);
        if (!"faster".equals(result.getVerdict())) throw new AssertionError(result);
    }
}
