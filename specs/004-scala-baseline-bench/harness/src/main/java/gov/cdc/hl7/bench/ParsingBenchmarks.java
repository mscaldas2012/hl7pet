package gov.cdc.hl7.bench;

import gov.cdc.hl7.HL7ParseError;
import gov.cdc.hl7.HL7StaticParser;
import java.util.List;
import java.util.concurrent.TimeUnit;
import org.openjdk.jmh.annotations.Benchmark;
import org.openjdk.jmh.annotations.BenchmarkMode;
import org.openjdk.jmh.annotations.Level;
import org.openjdk.jmh.annotations.Mode;
import org.openjdk.jmh.annotations.OutputTimeUnit;
import org.openjdk.jmh.annotations.Param;
import org.openjdk.jmh.annotations.Scope;
import org.openjdk.jmh.annotations.Setup;
import org.openjdk.jmh.annotations.State;
import org.openjdk.jmh.infra.Blackhole;

/**
 * "Parsing" operation benchmarks (research.md #3): raw structural scanning of a message
 * -- {@code retrieveFirstSegmentOf} / {@code retrieveMultipleSegments} -- with no
 * PATH-string evaluation. Compare against {@link ExtractionBenchmarks}, which measures
 * end-to-end parse+extract via {@code getValue}/{@code getFirstValue}.
 */
public class ParsingBenchmarks {

  /** One representative "typical"-size message per corpus message type (spec.md FR-006:
   *  results broken out per message type, not a single aggregate). */
  @State(Scope.Thread)
  public static class TypicalMessageState {
    @Param({"ADT^A01", "ADT^A08", "ORU^R01", "VXU^V04", "ORM^O01"})
    public String messageType;

    String message;

    @Setup(Level.Trial)
    public void setUp() {
      List<Corpus.CorpusMessage> candidates = Corpus.byType(messageType);
      message = candidates.stream()
          .filter(m -> "typical".equals(m.sizeCategory))
          .findFirst()
          .orElseThrow(() -> new IllegalStateException("no typical-size message for " + messageType))
          .content;
    }
  }

  /** The large/high-repetition message (100 OBX segments) -- benchmarked separately
   *  from the per-type loop above so its scale-extreme numbers aren't averaged away. */
  @State(Scope.Thread)
  public static class LargeMessageState {
    String message;

    @Setup(Level.Trial)
    public void setUp() {
      message = Corpus.all().stream()
          .filter(m -> "large-high-repetition".equals(m.sizeCategory))
          .findFirst()
          .orElseThrow(() -> new IllegalStateException("no large-high-repetition message in corpus"))
          .content;
    }
  }

  /** The minimal (MSH+PID only) message, likewise kept out of the per-type loop. */
  @State(Scope.Thread)
  public static class MinimalMessageState {
    String message;

    @Setup(Level.Trial)
    public void setUp() {
      message = Corpus.all().stream()
          .filter(m -> "minimal".equals(m.sizeCategory))
          .findFirst()
          .orElseThrow(() -> new IllegalStateException("no minimal message in corpus"))
          .content;
    }
  }

  @Benchmark
  @BenchmarkMode({Mode.Throughput, Mode.SampleTime})
  @OutputTimeUnit(TimeUnit.MICROSECONDS)
  public void retrieveFirstSegment(TypicalMessageState state, Blackhole bh) {
    try {
      bh.consume(HL7StaticParser.retrieveFirstSegmentOf(state.message, "PID"));
    } catch (HL7ParseError e) {
      ExclusionLog.record(state.messageType, e);
    }
  }

  @Benchmark
  @BenchmarkMode({Mode.Throughput, Mode.SampleTime})
  @OutputTimeUnit(TimeUnit.MICROSECONDS)
  public void retrieveFirstSegmentLarge(LargeMessageState state, Blackhole bh) {
    try {
      bh.consume(HL7StaticParser.retrieveFirstSegmentOf(state.message, "PID"));
    } catch (HL7ParseError e) {
      ExclusionLog.record("large-high-repetition", e);
    }
  }

  @Benchmark
  @BenchmarkMode({Mode.Throughput, Mode.SampleTime})
  @OutputTimeUnit(TimeUnit.MICROSECONDS)
  public void retrieveMultipleSegmentsLarge(LargeMessageState state, Blackhole bh) {
    try {
      bh.consume(HL7StaticParser.retrieveMultipleSegments(state.message, "OBX"));
    } catch (HL7ParseError e) {
      ExclusionLog.record("large-high-repetition", e);
    }
  }

  @Benchmark
  @BenchmarkMode({Mode.Throughput, Mode.SampleTime})
  @OutputTimeUnit(TimeUnit.MICROSECONDS)
  public void retrieveFirstSegmentMinimal(MinimalMessageState state, Blackhole bh) {
    try {
      bh.consume(HL7StaticParser.retrieveFirstSegmentOf(state.message, "PID"));
    } catch (HL7ParseError e) {
      ExclusionLog.record("minimal", e);
    }
  }
}
