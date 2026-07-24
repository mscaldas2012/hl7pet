package gov.cdc.hl7.bench;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import gov.cdc.hl7.HL7StaticParser;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import scala.Option;

/**
 * Harness wiring only (plan.md Technical Context "Testing"): confirms the corpus
 * loads, the Scala engine dependency is present and callable, and ManifestWriter
 * produces schema-conformant JSON. Not exhaustive engine-correctness coverage -
 * that is Roadmap spec 003's (regression-suite) responsibility.
 */
class HarnessWiringTest {

  @AfterEach
  void resetExclusionLog() {
    ExclusionLog.reset();
  }

  @Test
  void corpusLoadsWithExpectedShape() {
    assertEquals("interim-v1", Corpus.corpusId());
    List<Corpus.CorpusMessage> all = Corpus.all();
    assertTrue(all.size() >= 20 && all.size() <= 30, "expected ~20-30 corpus messages, got " + all.size());

    assertTrue(all.stream().anyMatch(m -> "large-high-repetition".equals(m.sizeCategory)),
        "corpus must include at least one large/high-repetition message (FR-004)");
    assertTrue(all.stream().anyMatch(m -> "minimal".equals(m.sizeCategory)),
        "corpus must include at least one minimal message (FR-004)");

    List<String> types = Corpus.messageTypes();
    assertTrue(types.size() >= 3, "expected several distinct message types, got " + types);
  }

  @Test
  void hl7StaticParserIsOnTheClasspathAndCallable() {
    // gov.cdc:hl7-pet_2.13:1.2.11, resolved as a normal Maven dependency (FR-002/FR-003) -
    // not vendored source. Calling a real corpus message through it here proves both.
    Corpus.CorpusMessage sample = Corpus.byType("ADT^A01").get(0);
    Option<String> lastName = HL7StaticParser.getFirstValue(sample.content, "PID-5.1");
    assertTrue(lastName.isDefined(), "expected PID-5.1 to resolve on a synthetic ADT^A01 message");
  }

  @Test
  void manifestWriterProducesSchemaConformantJson(@TempDir Path tempDir) throws Exception {
    ExclusionLog.record("does-not-exist",
        new gov.cdc.hl7.HL7ParseError("synthetic test exclusion", "TEST", null));

    Path manifestPath = ManifestWriter.write(tempDir, "test-run");
    assertTrue(Files.exists(manifestPath));

    JsonNode root = new ObjectMapper().readTree(manifestPath.toFile());
    assertEquals("test-run", root.get("runDate").asText());
    assertFalse(root.get("engineCoordinate").asText().isBlank());
    assertEquals("interim-v1", root.get("corpusId").asText());
    assertEquals("jmh-results.json", root.get("resultsFile").asText());

    JsonNode env = root.get("hostEnvironment");
    assertFalse(env.get("cpu").asText().isBlank());
    assertFalse(env.get("os").asText().isBlank());
    assertFalse(env.get("jdkVendorVersion").asText().isBlank());
    assertFalse(env.get("jmhVersion").asText().isBlank());

    assertEquals(1, root.get("excludedMessages").size());
    assertEquals("does-not-exist", root.get("excludedMessages").get(0).get("messageId").asText());
  }
}
