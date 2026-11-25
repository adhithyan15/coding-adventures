import com.codingadventures.microbenchmark.Analysis;
import com.codingadventures.microbenchmark.Microbenchmark;
import com.codingadventures.microbenchmark.Pair;
import java.util.List;

/** Compiled outside the Gradle project against the installed distribution jars. */
public final class Consumer {
    public static void main(String[] args) {
        List<Pair> pairs = List.of(
                new Pair(100, 80, "baseline-first"), new Pair(100, 80, "baseline-first"),
                new Pair(100, 80, "baseline-first"), new Pair(100, 80, "candidate-first"),
                new Pair(100, 80, "candidate-first"), new Pair(100, 80, "candidate-first"));
        Analysis result = Microbenchmark.analyze(pairs, 95, 1.05);
        if (!"faster".equals(result.verdict())) throw new AssertionError(result);
    }
}
