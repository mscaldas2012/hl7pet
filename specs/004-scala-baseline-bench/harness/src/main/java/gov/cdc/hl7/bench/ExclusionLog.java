package gov.cdc.hl7.bench;

import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;

/**
 * Collects corpus messages excluded from a benchmark run because they failed to parse
 * under the Scala engine, instead of aborting the run (spec.md FR-007, Edge Cases).
 * JMH benchmark methods run concurrently across forks/threads, so this is append-only
 * and thread-safe.
 */
public final class ExclusionLog {

  public static final class Entry {
    public final String messageId;
    public final String reason;

    public Entry(String messageId, String reason) {
      this.messageId = messageId;
      this.reason = reason;
    }
  }

  private static final List<Entry> ENTRIES = new CopyOnWriteArrayList<>();

  private ExclusionLog() {
  }

  public static void record(String messageId, Throwable cause) {
    ENTRIES.add(new Entry(messageId, cause.getClass().getSimpleName() + ": " + cause.getMessage()));
  }

  public static List<Entry> entries() {
    return List.copyOf(ENTRIES);
  }

  public static void reset() {
    ENTRIES.clear();
  }
}
