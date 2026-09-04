package gov.cdc.hl7.bench;

import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.module.scala.DefaultScalaModule$;
import gov.cdc.hl7.HL7ParseUtils;
import gov.cdc.hl7.model.Profile;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.List;
import org.junit.jupiter.api.Test;
import scala.Option;

class HierarchyBenchmarksSmokeTest {

  @Test
  void hierarchyModeScopesCorrectlyOnLargeMessage() throws IOException {
    List<Corpus.CorpusMessage> matches = Corpus.byType("ORU^R01^HIERARCHY");
    assertTrue(matches.size() == 1, "expected exactly one large-hierarchy message, got " + matches.size());
    String message = matches.get(0).content;

    String profileJson;
    try (InputStream in = HierarchyBenchmarksSmokeTest.class.getResourceAsStream("/profiles/large-hierarchy.json")) {
      profileJson = new String(in.readAllBytes(), StandardCharsets.UTF_8);
    }
    ObjectMapper mapper = new ObjectMapper();
    mapper.registerModule(DefaultScalaModule$.MODULE$);
    Profile profile = mapper.readValue(profileJson, Profile.class);

    HL7ParseUtils parser = new HL7ParseUtils(message, profile, true);
    Option<String[][]> result = parser.getValue("OBR[1] -> OBX-5");
    assertTrue(result.isDefined(), "expected OBR[1] -> OBX-5 to resolve");
    assertTrue(result.get().length == 5, "expected 5 OBX children for OBR[1], got " + result.get().length);

    Option<String[][]> resultAll = parser.getValue("OBR -> OBX-5");
    assertTrue(resultAll.isDefined());
    assertTrue(resultAll.get().length == 100, "expected 100 total OBX children across all OBR, got " + resultAll.get().length);
  }
}
