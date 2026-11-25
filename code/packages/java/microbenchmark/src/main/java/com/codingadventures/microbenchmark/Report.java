package com.codingadventures.microbenchmark;

import java.util.List;

/** Complete collection result. The pair list is an immutable snapshot. */
public record Report(Config config, long batchSize, List<Pair> pairs, Analysis analysis) {
    public Report { pairs = List.copyOf(pairs); }
}
