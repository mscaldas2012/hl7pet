package gov.cdc.hl7.bench;

import gov.cdc.hl7.HL7StaticParser;
import java.util.List;
import java.util.Map;
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
 * "Extraction" operation benchmarks (research.md #3): {@code getValue}/{@code
 * getFirstValue} evaluate a PATH string directly against the raw message string, so for
 * the Scala static-mode engine these figures are necessarily end-to-end parse+extract,
 * not extraction-in-isolation -- labeled as such in the results (spec.md FR-006).
 */
public class ExtractionBenchmarks {

  /** A representative repeating/nested-field PATH per message type, so the "extraction"
   *  benchmark isn't limited to the trivial PID-5.1 case for every type. */
  private static final Map<String, String> REPRESENTATIVE_PATH = Map.of(
      "ADT^A01", "PV1-3.1",
      "ADT^A08", "PV1-3.1",
      "ORU^R01", "OBX-5",
      "VXU^V04", "RXA-5.2",
      "ORM^O01", "OBR-4.2");

  @State(Scope.Thread)
  public static class TypicalMessageState {
    @Param({"ADT^A01", "ADT^A08", "ORU^R01", "VXU^V04", "ORM^O01"})
    public String messageType;

    String message;
    String representativePath;

    @Setup(Level.Trial)
    public void setUp() {
      List<Corpus.CorpusMessage> candidates = Corpus.byType(messageType);
      message = candidates.stream()
          .filter(m -> "typical".equals(m.sizeCategory))
          .findFirst()
          .orElseThrow(() -> new IllegalStateException("no typical-size message for " + messageType))
          .content;
      representativePath = REPRESENTATIVE_PATH.get(messageType);
    }
  }

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
  public void getFirstValuePatientLastName(TypicalMessageState state, Blackhole bh) {
    bh.consume(HL7StaticParser.getFirstValue(state.message, "PID-5.1"));
  }

  @Benchmark
  @BenchmarkMode({Mode.Throughput, Mode.SampleTime})
  @OutputTimeUnit(TimeUnit.MICROSECONDS)
  public void getValueRepresentativeField(TypicalMessageState state, Blackhole bh) {
    bh.consume(HL7StaticParser.getValue(state.message, state.representativePath));
  }

  @Benchmark
  @BenchmarkMode({Mode.Throughput, Mode.SampleTime})
  @OutputTimeUnit(TimeUnit.MICROSECONDS)
  public void getValueRepeatingFieldLarge(LargeMessageState state, Blackhole bh) {
    bh.consume(HL7StaticParser.getValue(state.message, "OBX-5"));
  }

  @Benchmark
  @BenchmarkMode({Mode.Throughput, Mode.SampleTime})
  @OutputTimeUnit(TimeUnit.MICROSECONDS)
  public void getFirstValueMinimal(MinimalMessageState state, Blackhole bh) {
    bh.consume(HL7StaticParser.getFirstValue(state.message, "PID-5.1"));
  }
}
