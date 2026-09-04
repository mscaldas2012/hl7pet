package gov.cdc.hl7.bench;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.module.scala.DefaultScalaModule$;
import gov.cdc.hl7.HL7ParseUtils;
import gov.cdc.hl7.model.Profile;
import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.concurrent.TimeUnit;
import org.openjdk.jmh.annotations.Benchmark;
import org.openjdk.jmh.annotations.BenchmarkMode;
import org.openjdk.jmh.annotations.Level;
import org.openjdk.jmh.annotations.Mode;
import org.openjdk.jmh.annotations.OutputTimeUnit;
import org.openjdk.jmh.annotations.Scope;
import org.openjdk.jmh.annotations.Setup;
import org.openjdk.jmh.annotations.State;
import org.openjdk.jmh.infra.Blackhole;

/**
 * Hierarchy-mode (spec 002/008's {@code ->} operator) benchmarks (research.md
 * #4) — a feature spec 004's original {@link ParsingBenchmarks}/
 * {@link ExtractionBenchmarks} never covered at all: both exclusively call
 * {@code HL7StaticParser} (flat/static mode). Uses {@link HL7ParseUtils}'s
 * hierarchy-mode constructor ({@code buildHierarchy = true}) against
 * Roadmap spec 009's {@code large-hierarchy.hl7}/{@code large-hierarchy.json}
 * (20 {@code OBR} occurrences, 5 {@code OBX} children each — large enough
 * that a bounded-scan implementation and an accidentally-unbounded one would
 * behave visibly differently).
 */
public class HierarchyBenchmarks {

  @State(Scope.Thread)
  public static class LargeHierarchyState {
    String message;
    Profile profile;

    @Setup(Level.Trial)
    public void setUp() throws IOException {
      List<Corpus.CorpusMessage> matches = Corpus.byType("ORU^R01^HIERARCHY");
      message = matches.stream()
          .findFirst()
          .orElseThrow(() -> new IllegalStateException("large-hierarchy message not found in corpus"))
          .content;
      // NOT gov.cdc.hl7.model.ProfileFactory.apply() -- verified live (research.md
      // #4 addendum) that it NPEs on any profile with a leaf segment (unconditional
      // .get("children").getAsJsonObject with no null check) and additionally
      // reads the misspelled JSON key "catdinality" instead of "cardinality" for
      // every segment. The Jackson/DefaultScalaModule path HL7HierarchyParser's own
      // parseMessageHierarchyFromJson already uses (spec 002's traced source) is
      // the one that actually works.
      ObjectMapper mapper = new ObjectMapper();
      mapper.registerModule(DefaultScalaModule$.MODULE$);
      profile = mapper.readValue(readProfileResource("/profiles/large-hierarchy.json"), Profile.class);
    }
  }

  private static String readProfileResource(String classpathPath) {
    try (InputStream in = HierarchyBenchmarks.class.getResourceAsStream(classpathPath)) {
      if (in == null) {
        throw new IOException("Profile not found on classpath: " + classpathPath);
      }
      return new String(in.readAllBytes(), StandardCharsets.UTF_8);
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }

  @Benchmark
  @BenchmarkMode({Mode.Throughput, Mode.SampleTime})
  @OutputTimeUnit(TimeUnit.MICROSECONDS)
  public void getValueSingleHopPlain(LargeHierarchyState state, Blackhole bh) {
    HL7ParseUtils parser = new HL7ParseUtils(state.message, state.profile, true);
    bh.consume(parser.getValue("OBR[1] -> OBX-5"));
  }

  @Benchmark
  @BenchmarkMode({Mode.Throughput, Mode.SampleTime})
  @OutputTimeUnit(TimeUnit.MICROSECONDS)
  public void getValueSingleHopIndexedChild(LargeHierarchyState state, Blackhole bh) {
    HL7ParseUtils parser = new HL7ParseUtils(state.message, state.profile, true);
    bh.consume(parser.getValue("OBR[1] -> OBX[3]-5"));
  }

  @Benchmark
  @BenchmarkMode({Mode.Throughput, Mode.SampleTime})
  @OutputTimeUnit(TimeUnit.MICROSECONDS)
  public void getValueSingleHopFilteredChild(LargeHierarchyState state, Blackhole bh) {
    HL7ParseUtils parser = new HL7ParseUtils(state.message, state.profile, true);
    bh.consume(parser.getValue("OBR[1] -> OBX[@5='VAL-1-2']-5"));
  }

  @Benchmark
  @BenchmarkMode({Mode.Throughput, Mode.SampleTime})
  @OutputTimeUnit(TimeUnit.MICROSECONDS)
  public void getValueAllParentsCombined(LargeHierarchyState state, Blackhole bh) {
    HL7ParseUtils parser = new HL7ParseUtils(state.message, state.profile, true);
    bh.consume(parser.getValue("OBR -> OBX-5"));
  }
}
