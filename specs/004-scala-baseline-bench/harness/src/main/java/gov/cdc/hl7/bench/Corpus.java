package gov.cdc.hl7.bench;

import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

/**
 * Loads the interim synthetic HL7 corpus (data-model.md "Benchmark Message Corpus")
 * from {@code src/main/resources/corpus/} for use by JMH {@code @State} benchmark
 * classes. See {@code corpus-manifest.json} for the corpus id and message list
 * (spec.md FR-004, FR-009).
 */
public final class Corpus {

  public static final class CorpusMessage {
    public final String messageId;
    public final String messageType;
    public final String sizeCategory;
    public final String content;

    CorpusMessage(String messageId, String messageType, String sizeCategory, String content) {
      this.messageId = messageId;
      this.messageType = messageType;
      this.sizeCategory = sizeCategory;
      this.content = content;
    }
  }

  private static final String CORPUS_ID;
  private static final List<CorpusMessage> ALL;

  static {
    try {
      Manifest manifest = readManifest();
      CORPUS_ID = manifest.corpusId;
      List<CorpusMessage> loaded = new ArrayList<>();
      for (ManifestEntry entry : manifest.messages) {
        loaded.add(new CorpusMessage(entry.messageId, entry.messageType, entry.sizeCategory,
            readResource("/corpus/" + entry.filePath)));
      }
      ALL = List.copyOf(loaded);
    } catch (IOException e) {
      throw new UncheckedIOException("Failed to load benchmark corpus", e);
    }
  }

  private Corpus() {
  }

  public static String corpusId() {
    return CORPUS_ID;
  }

  public static List<CorpusMessage> all() {
    return ALL;
  }

  public static List<CorpusMessage> byType(String messageType) {
    return ALL.stream().filter(m -> m.messageType.equals(messageType)).collect(Collectors.toList());
  }

  public static List<String> messageTypes() {
    return ALL.stream().map(m -> m.messageType).distinct().collect(Collectors.toList());
  }

  private static Manifest readManifest() throws IOException {
    try (InputStream in = Corpus.class.getResourceAsStream("/corpus/corpus-manifest.json")) {
      if (in == null) {
        throw new IOException("corpus-manifest.json not found on classpath under /corpus/");
      }
      Map<String, Object> raw = new ObjectMapper().readValue(in, Map.class);
      Manifest manifest = new Manifest();
      manifest.corpusId = (String) raw.get("corpusId");
      manifest.messages = new ArrayList<>();
      for (Object entryObj : (List<?>) raw.get("messages")) {
        Map<?, ?> entryMap = (Map<?, ?>) entryObj;
        ManifestEntry entry = new ManifestEntry();
        entry.messageId = (String) entryMap.get("messageId");
        entry.messageType = (String) entryMap.get("messageType");
        entry.sizeCategory = (String) entryMap.get("sizeCategory");
        entry.filePath = (String) entryMap.get("filePath");
        manifest.messages.add(entry);
      }
      return manifest;
    }
  }

  private static String readResource(String classpathPath) throws IOException {
    try (InputStream in = Corpus.class.getResourceAsStream(classpathPath)) {
      if (in == null) {
        throw new IOException("Corpus message not found on classpath: " + classpathPath);
      }
      return new String(in.readAllBytes(), StandardCharsets.UTF_8);
    }
  }

  private static final class Manifest {
    String corpusId;
    List<ManifestEntry> messages;
  }

  private static final class ManifestEntry {
    String messageId;
    String messageType;
    String sizeCategory;
    String filePath;
  }
}
