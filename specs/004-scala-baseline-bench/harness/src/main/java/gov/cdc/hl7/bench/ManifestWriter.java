package gov.cdc.hl7.bench;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;

/**
 * Writes {@code manifest.json} for a Benchmark Run, per
 * contracts/baseline-artifact-schema.md. Every field JMH itself doesn't already know
 * about (engine version, corpus id, host description, exclusions) lives here;
 * {@code jmh-results.json} (written separately by JMH's own {@code -rf json}) remains
 * the single source of truth for every Metric Result (spec.md FR-005, FR-008).
 */
public final class ManifestWriter {

  /** Fallback used only when the harness is run outside {@code mvn exec:java} (e.g. an
   *  IDE launch) and the pom-injected {@code hl7pet.coordinate} system property isn't
   *  set - kept in sync with pom.xml's {@code hl7pet.coordinate} property. */
  private static final String DEFAULT_ENGINE_COORDINATE = "gov.cdc:hl7-pet_2.13:1.2.11";

  private ManifestWriter() {
  }

  public static Path write(Path baselineDir, String runDate) throws IOException {
    Path runDir = baselineDir.resolve(runDate);
    Files.createDirectories(runDir);

    ObjectMapper mapper = new ObjectMapper();
    ObjectNode root = mapper.createObjectNode();
    root.put("runDate", runDate);
    root.put("engineCoordinate", System.getProperty("hl7pet.coordinate", DEFAULT_ENGINE_COORDINATE));
    root.put("corpusId", Corpus.corpusId());

    HostEnvironment env = HostEnvironment.capture();
    ObjectNode envNode = root.putObject("hostEnvironment");
    envNode.put("cpu", env.cpu);
    envNode.put("os", env.os);
    envNode.put("jdkVendorVersion", env.jdkVendorVersion);
    envNode.put("jmhVersion", env.jmhVersion);

    ArrayNode excludedMessages = root.putArray("excludedMessages");
    for (ExclusionLog.Entry entry : ExclusionLog.entries()) {
      ObjectNode entryNode = excludedMessages.addObject();
      entryNode.put("messageId", entry.messageId);
      entryNode.put("reason", entry.reason);
    }

    root.put("resultsFile", "jmh-results.json");

    Path manifestPath = runDir.resolve("manifest.json");
    mapper.writerWithDefaultPrettyPrinter().writeValue(manifestPath.toFile(), root);
    return manifestPath;
  }
}
