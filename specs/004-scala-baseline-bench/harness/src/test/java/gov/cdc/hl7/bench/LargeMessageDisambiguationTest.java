package gov.cdc.hl7.bench;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.util.List;
import org.junit.jupiter.api.Test;

class LargeMessageDisambiguationTest {
  @Test
  void largeHighRepetitionResolvesToTheOriginalOruMessageNotTheHierarchyOne() {
    List<Corpus.CorpusMessage> matches = Corpus.all().stream()
        .filter(m -> "large-high-repetition".equals(m.sizeCategory))
        .toList();
    assertEquals(1, matches.size(), "expected exactly one large-high-repetition message now that large-hierarchy uses its own category");
    assertEquals("oru_r01_large_026", matches.get(0).messageId);
  }
}
