package com.codingadventures.microbenchmark;

import java.io.IOException;
import java.io.OutputStream;
import java.io.PrintStream;
import java.nio.file.Path;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/** Native Java command-line driver for the indexed-array VER07 workload. */
public final class ArrayMicrobenchmark {
    private ArrayMicrobenchmark() {}
    private static final int[] VALUES = new int[4096];
    private static final long ONE_PASS;
    static {
        long sum = 0;
        for (int i = 0; i < VALUES.length; i++) { VALUES[i] = (17 * i + 3) % 251; sum += VALUES[i]; }
        ONE_PASS = sum;
    }

    public static void main(String[] args) {
        int status = run(args, System.out, System.err, Microbenchmark.monotonicClock());
        if (status != 0) System.exit(status);
    }

    static int run(String[] args, OutputStream stdout, PrintStream stderr, Clock clock) {
        try {
            Config config = Config.defaults();
            String revision = "unknown";
            String replayPath = null;
            for (int i = 0; i < args.length; i++) {
                String option = args[i];
                if (option.equals("--help")) { stderr.println("Usage: array-microbenchmark [--pairs N] [--confidence N] [--factor F] [--seed I64] [--target-ns N] [--max-batch N] [--revision REV] | --replay FILE"); return 0; }
                if (i + 1 >= args.length) throw new IllegalArgumentException("missing value for " + option);
                String value = args[++i];
                switch (option) {
                    case "--pairs" -> config = new Config(Integer.parseInt(value), config.confidencePercent(), config.minimumFactor(), config.seed(), config.targetNs(), config.maxBatch());
                    case "--confidence" -> config = new Config(config.pairs(), Integer.parseInt(value), config.minimumFactor(), config.seed(), config.targetNs(), config.maxBatch());
                    case "--factor" -> config = new Config(config.pairs(), config.confidencePercent(), Double.parseDouble(value), config.seed(), config.targetNs(), config.maxBatch());
                    case "--seed" -> config = new Config(config.pairs(), config.confidencePercent(), config.minimumFactor(), Long.parseLong(value), config.targetNs(), config.maxBatch());
                    case "--target-ns" -> config = new Config(config.pairs(), config.confidencePercent(), config.minimumFactor(), config.seed(), Long.parseLong(value), config.maxBatch());
                    case "--max-batch" -> config = new Config(config.pairs(), config.confidencePercent(), config.minimumFactor(), config.seed(), config.targetNs(), Long.parseLong(value));
                    case "--revision" -> revision = value;
                    case "--replay" -> replayPath = value;
                    default -> throw new IllegalArgumentException("unknown option: " + option);
                }
            }
            if (replayPath != null) {
                Report report = ReportCodec.replay(ReportCodec.readFile(Path.of(replayPath)));
                write(stdout, ("replay valid: " + report.pairs().size() + " pairs, " + report.analysis().verdict() + "\n").getBytes(StandardCharsets.UTF_8));
                return 0;
            }
            config.validate();
            Batch indexed = batch -> {
                long sum = 0;
                for (long pass = 0; pass < batch; pass++) for (int i = 0; i < VALUES.length; i++) sum += VALUES[i];
                return sum;
            };
            Batch iterated = batch -> {
                long sum = 0;
                for (long pass = 0; pass < batch; pass++) for (int value : VALUES) sum += value;
                return sum;
            };
            Report report = Microbenchmark.collect(config, ONE_PASS, indexed, iterated, clock);
            List<String> invocation = new ArrayList<>();
            invocation.add("array-microbenchmark"); invocation.addAll(Arrays.asList(args));
            String json = ReportCodec.toJson(report, invocation, revision, Runtime.version().toString(),
                    System.getProperty("os.name", "unknown"), System.getProperty("os.arch", "unknown"));
            ReportCodec.replay(json); // Do not emit a malformed or non-replayable report.
            write(stdout, json.getBytes(StandardCharsets.UTF_8));
            return 0;
        } catch (Exception error) {
            stderr.println("benchmark failed: " + error.getMessage());
            return 1;
        }
    }

    private static void write(OutputStream out, byte[] bytes) throws IOException {
        out.write(bytes);
        out.flush();
        if (out instanceof PrintStream ps && ps.checkError()) throw new IOException("stdout write failed");
    }
}
