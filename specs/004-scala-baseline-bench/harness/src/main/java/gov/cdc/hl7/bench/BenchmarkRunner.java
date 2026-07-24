package gov.cdc.hl7.bench;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.LocalDate;
import java.util.List;
import org.openjdk.jmh.profile.GCProfiler;
import org.openjdk.jmh.runner.Runner;
import org.openjdk.jmh.runner.RunnerException;
import org.openjdk.jmh.runner.options.Options;
import org.openjdk.jmh.runner.options.OptionsBuilder;
import org.openjdk.jmh.runner.options.TimeValue;
import org.openjdk.jmh.results.format.ResultFormatType;

/**
 * Runs {@link ParsingBenchmarks} and {@link ExtractionBenchmarks} with JMH's built-in
 * GC/allocation profiler and native JSON result output, then writes the accompanying
 * {@code manifest.json} (T007). Produces one Baseline Results Artifact per invocation
 * under {@code baseline/<run-date>/} (spec.md FR-005, FR-010) - see
 * contracts/baseline-artifact-schema.md.
 *
 * <p>Usage: {@code mvn compile exec:java} (default output dir is {@code ../baseline}
 * relative to this module, i.e. {@code specs/004-scala-baseline-bench/baseline/}),
 * or pass an explicit output directory as the first argument.
 */
public final class BenchmarkRunner {

  private BenchmarkRunner() {
  }

  public static void main(String[] args) throws RunnerException, IOException {
    Path baselineDir = Path.of(args.length > 0 ? args[0] : "../baseline").toAbsolutePath().normalize();
    Files.createDirectories(baselineDir);
    String runDate = resolveRunDate(baselineDir);
    Path runDir = baselineDir.resolve(runDate);
    Files.createDirectories(runDir);
    Path resultsFile = runDir.resolve("jmh-results.json");

    System.out.println("Scala baseline benchmark run: " + runDate);
    System.out.println("Results will be written to: " + runDir);

    Options options = new OptionsBuilder()
        .include("gov\\.cdc\\.hl7\\.bench\\.ParsingBenchmarks\\..*")
        .include("gov\\.cdc\\.hl7\\.bench\\.ExtractionBenchmarks\\..*")
        .addProfiler(GCProfiler.class)
        // forks(3), not forks(1): for these sub-microsecond calls, run-to-run
        // reproducibility (spec.md SC-003) is dominated by which JIT compilation tier
        // a single JVM happens to land in during its measurement window, not by
        // measurement noise within one JVM - more forks average that out across fresh
        // JVMs, which more warmup/measurement iterations within one fork does not fix.
        .forks(3)
        .warmupIterations(3)
        .warmupTime(TimeValue.seconds(1))
        .measurementIterations(3)
        .measurementTime(TimeValue.seconds(1))
        .resultFormat(ResultFormatType.JSON)
        .result(resultsFile.toString())
        .build();

    new Runner(options).run();

    Path manifestPath = ManifestWriter.write(baselineDir, runDate);
    System.out.println("Wrote manifest: " + manifestPath);
    System.out.println("Wrote results: " + resultsFile);

    List<ExclusionLog.Entry> exclusions = ExclusionLog.entries();
    if (!exclusions.isEmpty()) {
      System.out.println("Excluded " + exclusions.size() + " message(s) - see manifest.json for reasons.");
    }
  }

  /** Never overwrites a previously committed run in place (spec.md FR-010): if
   *  {@code <baselineDir>/<today>} already exists (e.g. a same-day re-run during
   *  development), a numeric suffix is appended instead. */
  static String resolveRunDate(Path baselineDir) {
    String today = LocalDate.now().toString();
    if (!Files.exists(baselineDir.resolve(today))) {
      return today;
    }
    int suffix = 2;
    while (Files.exists(baselineDir.resolve(today + "-" + suffix))) {
      suffix++;
    }
    return today + "-" + suffix;
  }
}
