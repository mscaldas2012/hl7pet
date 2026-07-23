# HL7-PET Functional Specification

**Version:** 1.2.10  
**Package:** `gov.cdc.hl7`  
**License:** Apache 2.0  
**Author:** Marcelo Caldas, CDC

---

## Table of Contents

1. [Overview](#1-overview)
2. [Installation](#2-installation)
3. [Core Concepts](#3-core-concepts)
4. [API Reference](#4-api-reference)
5. [Return Types & Error Handling](#5-return-types--error-handling)
6. [Built-in Profiles & Rules Files](#6-built-in-profiles--rules-files)
7. [Known Limitations](#7-known-limitations)
8. [Quick-Start Examples](#8-quick-start-examples)

---

## 1. Overview

HL7-PET (**HL7 Parsing and Extraction Toolkit**) is a Scala library for working with HL7 v2 messages. It does not transform messages into a deep object model; instead it exposes a lightweight path-based query API that operates directly on the raw message string. This makes it well-suited for applications that need to read a handful of fields from many messages efficiently.

**Capabilities at a glance:**

| Capability | Primary class |
|---|---|
| Field extraction by path | `HL7ParseUtils`, `HL7StaticParser` |
| Hierarchy-aware extraction (parent→child) | `HL7ParseUtils` (with profile) |
| Segment structure & cardinality validation | `StructureValidator` |
| Conditional & conformance rule validation | `RulesValidator` |
| Batch file (FHS/BHS/BTS/FTS) validation | `BatchValidator` |
| PII redaction / de-identification | `DeIdentifier` |
| File splitting (single messages or batches) | `HL7FileUtils` |

**Supported HL7 versions:** v2.x (any version; profiles define the expected structure)  
**Runtime requirements:** JVM 11+, Scala 2.13.x

---

## 2. Installation

The library is built with SBT. To use it in your own SBT project, publish it locally and add the dependency:

```bash
# In the HL7-PET repo directory:
sbt publishLocal
```

Then in your project's `build.sbt`:

```scala
libraryDependencies += "gov.cdc.hl7" %% "hl7-pet" % "1.2.10"
```

**Transitive dependencies** your project will also require:

```scala
"com.fasterxml.jackson.core"   % "jackson-databind"        % "2.17.0"
"com.fasterxml.jackson.module" %% "jackson-module-scala"   % "2.17.0"
"com.google.code.gson"         % "gson"                    % "2.10.1"
```

---

## 3. Core Concepts

### 3.1 Path Expression Syntax

Paths are the primary way to address data within an HL7 message. The full grammar is:

```
PATH  ::= SEGMENT_EXPR [ "-" FIELD_EXPR ]
        | SEGMENT_EXPR " -> " CHILD_PATH      (hierarchy mode only)

SEGMENT_EXPR ::= SEG [ "[" SEG_IDX "]" ]
SEG          ::= /[A-Z0-9]{3}/                (e.g. MSH, PID, OBR, OBX)
SEG_IDX      ::= number                       (1-based occurrence index)
               | "$LAST"                      (last occurrence)
               | "*"                          (all occurrences — same as omitting the index)
               | "@" FILTER                   (field value filter)

FILTER       ::= field_num [ "." comp_num ] OPERATOR "'" value { "||" value } "'"
OPERATOR     ::= "=" | "!=" | ">" | ">=" | "<" | "<="

FIELD_EXPR   ::= field_num [ "[" FIELD_IDX "]" ] [ "." comp_num [ "." subcomp_num ] ]
FIELD_IDX    ::= number | "$LAST"
field_num    ::= /[0-9]+/                     (1-based; MSH field 1 = "|", field 2 = "^~\&")
comp_num     ::= /[0-9]+/                     (1-based component)
subcomp_num  ::= /[0-9]+/                     (1-based sub-component)

CHILD_PATH   ::= SEGMENT_EXPR [ "-" FIELD_EXPR ]
```

**Examples:**

| Path | Meaning |
|---|---|
| `MSH-12` | MSH field 12 (Version ID), all repetitions |
| `MSH-9.1` | MSH field 9, component 1 (Message Code) |
| `OBR[1]-4.1` | First OBR, field 4, component 1 |
| `OBX[2]-5` | Second OBX, field 5 (Observation Value) |
| `OBX[$LAST]-5` | Last OBX, field 5 |
| `OBX[@3.1='12345-6']-5` | OBX where field 3, component 1 equals `12345-6`, return field 5 |
| `OBX[@11!='X']-5` | OBX where field 11 is not `X`, return field 5 |
| `OBR[1] -> OBX-5` | Field 5 of OBX segments that are children of the first OBR (hierarchy mode) |
| `PID[1]-5[2].1` | PID field 5, second repetition, component 1 |
| `MSH-21[3].1` | MSH field 21, third repetition, component 1 |

### 3.2 Profiles

A **Profile** is a JSON document that describes the expected message structure. It has two top-level keys:

- **`segmentDefinition`** — defines the segment hierarchy and cardinality. Each entry maps a segment name to a `SegmentConfig` with a `cardinality` string and optional `children`.
- **`segmentFields`** — defines field-level metadata (name, data type, max length, usage, cardinality, conformance) for each segment.

**Cardinality strings:**

| String | Meaning |
|---|---|
| `[1..1]` | Exactly one (required) |
| `[0..1]` | Zero or one (optional) |
| `[1..*]` | One or more (required, repeating) |
| `[0..*]` | Zero or more (optional, repeating) |
| `[m..n]` | Between m and n occurrences |

**Field usage codes:**

| Code | Meaning |
|---|---|
| `R` | Required |
| `RE` | Required if known |
| `O` | Optional |
| `X` | Not used |
| `C(...)` | Conditional (predicate-based) |

**Minimal profile example:**

```json
{
  "segmentDefinition": {
    "MSH": {
      "cardinality": "[1..1]",
      "children": {
        "PID": { "cardinality": "[1..1]" },
        "OBR": {
          "cardinality": "[1..*]",
          "children": {
            "OBX": { "cardinality": "[0..*]" }
          }
        }
      }
    }
  },
  "segmentFields": {
    "MSH": [
      { "fieldNumber": 9, "name": "Message Type", "dataType": "MSG",
        "maxLength": 15, "usage": "R", "cardinality": "[1..1]", "conformance": "" }
    ]
  }
}
```

### 3.3 Hierarchy Mode vs. Static Mode

**Static mode** (`buildHierarchy = false`) treats all segments as a flat list. The `HL7StaticParser` object always operates this way. It is faster and does not require a profile. Parent→child path expressions (`->`) are unavailable.

**Hierarchy mode** (`buildHierarchy = true`, requires a profile) builds a tree from the message at construction time using the profile's `segmentDefinition`. Parent→child path expressions become available and correctly scope child segments to their parent occurrence.

When you construct `HL7ParseUtils` with only a message string (no profile), the library loads `PhinGuideProfile.json` as the default profile but disables hierarchy building for performance:

```scala
// Static mode (no hierarchy, uses PhinGuideProfile internally for defaults only)
val parser = new HL7ParseUtils(message)

// Hierarchy mode (requires explicit profile)
val parser = new HL7ParseUtils(message, profile)
// or equivalently:
val parser = new HL7ParseUtils(message, profile, true)
```

---

## 4. API Reference

### 4.1 `HL7ParseUtils` (class)

The main entry point for instance-based parsing. Maintains the message string and optionally a pre-built hierarchy.

#### Constructors

```scala
// Static mode: no hierarchy, loads PhinGuideProfile.json as default
new HL7ParseUtils(message: String)

// Hierarchy mode: profile required, hierarchy ON by default
new HL7ParseUtils(message: String, profile: Profile)

// Explicit control
new HL7ParseUtils(message: String, profile: Profile, buildHierarchy: Boolean)
```

#### Static factory

```scala
// Loads a profile by classpath resource filename and returns a hierarchy-enabled parser
HL7ParseUtils.getParser(message: String, profileFilename: String): HL7ParseUtils
```

#### Methods

```scala
// Count how many times a segment appears
def peek(segment: String): Int

// Retrieve a segment that must appear exactly once; throws HL7ParseError if 0 or >1 found
def retrieveSegment(segment: String): (Int, Array[String])

// Retrieve all occurrences of a segment; keys are 1-based line numbers
def retrieveMultipleSegments(segment: String): SortedMap[Int, Array[String]]

// Retrieve the first occurrence of a segment
def retrieveFirstSegmentOf(segment: String): (Int, Array[String])

// Split a segment line or raw string into components of the specified field
def splitFields(line: String, field: Int): Array[String]
def splitFields(line: Array[String], field: Int): Array[String]

// Extract values by path from the full message
// Returns None when the path matches nothing or all values are empty (with removeEmpty=true)
def getValue(path: String): Option[Array[Array[String]]]
def getValue(path: String, removeEmpty: Boolean): Option[Array[Array[String]]]

// Extract values by path from a single pre-split segment line
def getValue(path: String, segment: Array[String], removeEmpty: Boolean): Option[Array[String]]

// Convenience: returns only the first matching scalar value
def getFirstValue(path: String): Option[String]
```

**Return structure for `getValue(path)`:**  
`Option[Array[Array[String]]]` — the outer array has one entry per matching segment occurrence; the inner array has one entry per field repetition.

---

### 4.2 `HL7StaticParser` (object)

Stateless counterpart to `HL7ParseUtils`. All methods take the raw message string as the first argument. Use this when you need a single extraction without holding a parser instance, or when working with individual pre-fetched segment lines.

```scala
def peek(msg: String, segment: String): Int
def retrieveSegment(msg: String, segment: String): (Int, Array[String])
def retrieveMultipleSegments(msg: String, segment: String): SortedMap[Int, Array[String]]
def retrieveFirstSegmentOf(msg: String, segment: String): (Int, Array[String])
def splitFields(line: String, field: Int): Array[String]

def getValue(msg: String, path: String): Option[Array[Array[String]]]
def getValue(msg: String, path: String, removeEmpty: Boolean): Option[Array[Array[String]]]
def getValue(msg: String, path: String, segment: Array[String], removeEmpty: Boolean): Option[Array[String]]
def getFirstValue(msg: String, path: String): Option[String]
```

**Useful constants:**

```scala
HL7StaticParser.NEW_LINE_FEED          // regex matching \r\n, \n\r, \r, \n
HL7StaticParser.HL7_FIELD_SEPARATOR    // "\\|"
HL7StaticParser.HL7_COMPONENT_SEPARATOR// "\\^"
HL7StaticParser.HL7_FIELD_REPETITION   // "\\~"
HL7StaticParser.LAST_INDEX             // "$LAST"
```

---

### 4.3 `StructureValidator` (class)

Validates the structure and field-level content of an HL7 message against a profile.

#### Constructor

```scala
// Pass null for either profile arg to load the bundled defaults:
//   profile          → DefaultProfile.json
//   fieldDefinitions → DefaultFieldsProfile.json
new StructureValidator(message: String, profile: Profile, fieldDefinitions: Profile)
```

#### Methods

```scala
// Run full validation: cardinality, field usage, data types, max lengths, default values
def validateMessage(): ValidationErrors

// Check cardinality for a single (segmentName → SegmentConfig) pair; used internally
def checkSegmentCardinality(segmentConfig: (String, SegmentConfig), errors: ValidationErrors): Unit
```

`validateMessage()` checks:
1. Segment cardinality (required/optional/bounded per profile hierarchy)
2. Field usage (`R`, `RE`, `O`, `X`, conditional `C(...)`)
3. Field cardinality (repeating vs. single-valued)
4. Data type (ST, NM, TS/DT, composite types recursively)
5. Max field length
6. Default/constant field values

---

### 4.4 `RulesValidator` (class)

Validates business rules loaded from a JSON rules file on the classpath.

#### Constructor

```scala
// rulesFile must be a classpath resource (e.g., "predicateRules.json")
new RulesValidator(rulesFile: String)
```

#### Methods

```scala
// Check predicate rules: requiredIfEmpty, requiredIfNotEmpty
def validatePredicate(message: String): ValidationErrors

// Check conformance rules: regEx, constant, existsUnique
def validateConformance(message: String): ValidationErrors
```

**Predicate rule types:**

| `name` | Behavior |
|---|---|
| `requiredIfEmpty` | Field `field` is required when `reference` is absent |
| `requiredIfNotEmpty` | Field `field` is required when `reference` is present |

**Conformance rule types:**

| `name` | Behavior |
|---|---|
| `regEx` | Each occurrence of `field` must match the `reference` regex |
| `constant` | Each occurrence of `field` must equal the `reference` string exactly |
| `existsUnique` | `field` must appear exactly once across the entire message |

**Rules file schema (JSON):**

```json
{
  "rules": {
    "predicates": [
      {
        "name": "requiredIfNotEmpty",
        "segment": "OBR",
        "field": "OBR-22",
        "reference": "OBR-4.1",
        "usage": "R",
        "comment": "OBR[*]-22",
        "description": "OBR-22 is required when OBR-4.1 is present"
      }
    ],
    "conformance": [
      {
        "name": "regEx",
        "segment": "MSH",
        "field": "MSH-12",
        "reference": "2\\.5\\.1",
        "usage": "R",
        "comment": "MSH-12",
        "description": "MSH-12 must be 2.5.1"
      }
    ]
  }
}
```

---

### 4.5 `BatchValidator` (class)

Validates the HL7 batch envelope structure (FHS/BHS/BTS/FTS) and optionally splits the batch into individual messages.

#### Constructors

```scala
// Uses bundled DefaultBatchingProfile.json
new BatchValidator(message: String)

// With custom profile
new BatchValidator(message: String, profile: Profile)

// With hierarchy control (rarely needed)
new BatchValidator(message: String, profile: Profile, buildHierarchy: Boolean)
```

#### Methods

```scala
// Validate batch envelope structure and trailer counts
def validateBatchingInfo(): ValidationErrors

// Strip FHS/BHS/BTS/FTS lines and return a List of individual message strings
def debatchMessages(): List[String]
```

**`validateBatchingInfo()` checks:**
- All four batch segments (FHS, BHS, BTS, FTS) are present if any one is present
- No duplicates of any batch segment
- FHS is on line 1; BHS follows immediately after FHS (or is line 1 if FHS absent)
- FTS is the last line; BTS is the second-to-last (when FTS present) or last
- BTS field 1 (message count) matches the actual MSH count
- At least one MSH segment is present

**Error categories emitted:** `INVALID_BATCH_SEGMENTS`, `INVALID_BATCH_SEGMENT`, `INVALID_FILE_HEADER_SEGMENT`, `DUPLICATE_SEGMENT`, `INVALID_MESSAGE`

---

### 4.6 `DeIdentifier` (class)

Redacts or removes sensitive fields from HL7 messages using a rule-driven approach.

#### Constructor

```scala
new DeIdentifier()
```

#### Methods

```scala
// De-identify a message string in memory
// Returns (cleanMessage, redactionReport)
def deIdentifyMessage(message: String, rules: Array[String]): (String, java.util.List[RedactInfo])

// Read message and rules from files; write cleaned message to <input>_deidentified<ext>
def deIdentifyFile(messageFileName: String, rulesFileName: String): Unit
```

#### Rule format

Each rule is a comma-separated string with two or three fields:

```
"<PATH>,<REPLACEMENT>[,<CONDITION>]"
```

**Replacement values:**

| Value | Effect |
|---|---|
| Any literal string | Replaces matched field value with the literal |
| `$HASH` | Replaces with `abs(originalValue.hashCode).toString` |
| `$REMOVE` | Removes the entire segment line from the message |

**Condition format:**

```
"<CONDITION_PATH> <OPERATOR> <VALUE>"
```

| Operator | Example | Meaning |
|---|---|---|
| `=` | `PID-3.5 = PI` | Redact when field equals `PI` |
| `!=` | `PID-3.5 != PT` | Redact when field does not equal `PT` |
| `IN` | `PID-3.5 IN (PI;PT)` | Redact when field is in the set |
| `!IN` | `PID-3.5 !IN (PI;PT)` | Redact when field is not in the set |

When no condition is specified, redaction is unconditional.

#### `RedactInfo` (returned in report)

```scala
case class RedactInfo(
  path: String,        // HL7 path that was redacted
  fieldIndex: Int,     // Repetition index (0 = whole field/segment)
  rule: String,        // Human-readable description of what was done
  condition: String,   // Condition that triggered redaction (if any)
  lineNumber: Int      // 1-based line number in original message
)
```

---

### 4.7 `HL7FileUtils` (class)

File-level utilities for splitting multi-message or batch files on disk.

```scala
new HL7FileUtils()
```

#### Methods

```scala
// Split a file containing multiple HL7 messages (separated by MSH) into individual files
// Output: <outputDir>/<prefix>_00001.txt, <prefix>_00002.txt, ...
def splitMessages(filename: String, outputDir: String, outputFileNamePrefix: String): Unit

// Split a file containing multiple HL7 batches (separated by FHS) into individual files
def splitBatches(filename: String, outputDir: String, outputFileNamePrefix: String): Unit

// Extract OBX observation text to a clean human-readable file
// Output: <filename_without_ext>_clean<ext>
def genOBXMesages(filename: String): Unit
```

`outputDir` is created automatically if it does not exist.

---

## 5. Return Types & Error Handling

### `Option[...]` return convention

All extraction methods return `Option` values and never throw for missing data:

- `None` — path matched no segments, field was out of range, or all values were empty (when `removeEmpty = true`)
- `Some(...)` — one or more values found

### `ValidationErrors`

Aggregates all errors and warnings from a validation run.

```scala
class ValidationErrors {
  def addEntry(entry: ErrorEntry): Unit
  def getEntries(): List[ErrorEntry]
  def getTotalErrors(): Int
  def getTotalWarnings(): Int
}
```

### `ErrorEntry`

```scala
class ErrorEntry(
  line: Int,               // 1-based line number in the message (0 = message-level)
  beginColumn: Int,        // Start column of the offending value
  endColumn: Int,          // End column
  path: String,            // HL7 path (e.g., "OBR[1]-4[1]")
  classification: ClassificationEnum, // ERROR | WARNING
  category: String         // e.g., "INVALID_USAGE", "INVALID_CARDINALITY", "INVALID_PREDICATE"
) {
  var description: String  // Human-readable explanation
}
```

**Category values:**

| Category | Emitted by |
|---|---|
| `INVALID_MESSAGE` | `StructureValidator`, `BatchValidator` |
| `INVALID_USAGE` | `StructureValidator` |
| `INVALID_CARDINALITY` | `StructureValidator` |
| `INVALID_SEGMENT` | `StructureValidator` |
| `INVALID_FIELD_TYPE` | `StructureValidator` |
| `INVALID_FIELD_LENGTH` | `StructureValidator` |
| `INVALID_DEFAULT_VALUE` | `StructureValidator` |
| `INVALID_PREDICATE` | `RulesValidator` |
| `INVALID_CONFORMANCE` | `RulesValidator` |
| `INVALID_BATCH_SEGMENTS` | `BatchValidator` |
| `INVALID_BATCH_SEGMENT` | `BatchValidator` |
| `DUPLICATE_SEGMENT` | `BatchValidator` |

### `HL7ParseError`

Thrown by segment retrieval methods when structural preconditions are violated (not for missing data):

```scala
case class HL7ParseError(message: String, segment: String, cause: Throwable = null)
  extends Exception
```

Thrown when:
- `retrieveSegment` finds more than one occurrence of a segment it expected to be unique
- `retrieveFirstSegmentOf` is called for a segment that does not exist
- `splitFields` references a field index that does not exist
- A profile contains an invalid cardinality string

---

## 6. Built-in Profiles & Rules Files

All bundled resources live under `src/main/resources/` and are available on the classpath at runtime.

### Structure Profiles

| Filename | Description |
|---|---|
| `PhinGuideProfile.json` | **Default for `HL7ParseUtils`**. PHIN messaging guide structure for ORU^R01 v2.5.1 with field definitions (MSH, SFT, PID, NTE, OBR, OBX, SPM). |
| `DefaultProfile.json` | General ORU structure without full field definitions. Used as default by `StructureValidator`. |
| `DefaultFieldsProfile.json` | **Default field definitions** used by `StructureValidator` when `fieldDefinitions` is `null`. |
| `BasicProfile.json` | Minimal MSH→PID→OBR→OBX→SPM hierarchy, no field-level definitions. |
| `DefaultBatchingProfile.json` | **Default for `BatchValidator`**. Defines FHS/BHS/BTS/FTS structure and field rules. |
| `PhinGuideProfile_NoORC.json` | PHIN profile variant without ORC segments. |
| `PhinProfileFlat.json` | PHIN profile with a flat (non-nested) segment structure. |
| `COVID_ORC.json` | COVID-19 ELR profile variant including ORC segments. |

### Validation Rules Files

| Filename | Used with |
|---|---|
| `predicateRules.json` | `new RulesValidator("predicateRules.json")` |
| `conformanceRules.json` | `new RulesValidator("conformanceRules.json")` |

### De-identification Rules Files

| Filename | Description |
|---|---|
| `redaction_rules.txt` | Default regex-based redaction rules for the `DeIdentifierApp` CLI |
| `default_rules.txt` | Alternative default rules set |

---

## 7. Known Limitations

| Limitation | Detail |
|---|---|
| **No escaped character support** | HL7 escape sequences (e.g., `\H\`, `\N\`) are not decoded or re-encoded. Values are returned as-is from the raw message string. (As of v1.2.2) |
| **MSH-1 and MSH-2 must be standard** | MSH-1 must be `\|` and MSH-2 must be `^~\&`. Non-standard delimiters are not supported. |
| **Filter syntax is limited** | Filters support only a single field+component expression per filter clause. Complex multi-field filter predicates are not supported. |
| **Hierarchy requires a profile** | The `->` child navigation operator is only available when `buildHierarchy = true` and a profile is supplied. Calling it on a static-mode parser silently falls back to flat extraction. |
| **`retrieveSegment` throws on duplicates** | If the same segment appears more than once, `retrieveSegment` throws `HL7ParseError`. Use `retrieveMultipleSegments` when repeating segments are expected. |
| **De-identification condition parsing** | Condition expressions must be exactly three whitespace-separated tokens: `PATH OPERATOR VALUE`. Multi-token values are not supported. |
| **No streaming API** | The entire message must be loaded into a `String` before parsing. Very large files should be pre-split with `HL7FileUtils` before parsing. |

---

## 8. Quick-Start Examples

### Example 1 — Basic field extraction (no profile)

```scala
import gov.cdc.hl7.HL7ParseUtils

val message = scala.io.Source.fromResource("my_message.hl7").mkString

val parser = new HL7ParseUtils(message)

// Single value
val version: Option[String] = parser.getFirstValue("MSH-12")

// Multiple segment occurrences
val allObxValues: Option[Array[Array[String]]] = parser.getValue("OBX-5")
allObxValues.foreach { results =>
  results.foreach { row => row.foreach(println) }
}

// Segment count
val obxCount: Int = parser.peek("OBX")
```

---

### Example 2 — Hierarchy-aware extraction

```scala
import com.fasterxml.jackson.databind.ObjectMapper
import com.fasterxml.jackson.module.scala.DefaultScalaModule
import gov.cdc.hl7.{HL7ParseUtils, HL7StaticParser}
import gov.cdc.hl7.model.Profile

val message = scala.io.Source.fromResource("covid19_elr.hl7").mkString

// Load a profile
val mapper = new ObjectMapper().registerModule(DefaultScalaModule)
val profileJson = scala.io.Source.fromResource("DefaultProfile.json").mkString
val profile = mapper.readValue(profileJson, classOf[Profile])

// Or use the static factory:
// val parser = HL7ParseUtils.getParser(message, "DefaultProfile.json")

val parser = new HL7ParseUtils(message, profile)

// OBX children of the first OBR only
val obr1Results = parser.getValue("OBR[1] -> OBX-5")

// Filter by LOINC code
val covidResult = parser.getValue("OBX[@3.1='94500-6']-5")
```

---

### Example 3 — Structure & rules validation

```scala
import gov.cdc.hl7.{StructureValidator, RulesValidator}

val message = scala.io.Source.fromResource("my_message.hl7").mkString

// Structure validation (null = use defaults)
val structErrors = new StructureValidator(message, null, null).validateMessage()
println(s"Errors: ${structErrors.getTotalErrors()}, Warnings: ${structErrors.getTotalWarnings()}")
structErrors.getEntries().foreach { e =>
  println(s"[${e.classification}] line ${e.line}: ${e.path} — ${e.description}")
}

// Rules validation
val rulesValidator = new RulesValidator("predicateRules.json")
val predicateErrors = rulesValidator.validatePredicate(message)
val conformanceErrors = rulesValidator.validateConformance(message)
```

---

### Example 4 — Batch validation and debatching

```scala
import gov.cdc.hl7.BatchValidator

val batchMessage = scala.io.Source.fromFile("/data/incoming/batch.hl7").mkString

val validator = new BatchValidator(batchMessage)
val errors = validator.validateBatchingInfo()

if (errors.getTotalErrors() == 0) {
  val messages: List[String] = validator.debatchMessages()
  messages.foreach { msg =>
    // process each individual message
  }
} else {
  errors.getEntries().foreach(e => println(e.description))
}
```

---

### Example 5 — De-identification

```scala
import gov.cdc.hl7.DeIdentifier

val message = scala.io.Source.fromResource("patient_report.hl7").mkString

val deidentifier = new DeIdentifier()

val rules = Array(
  "PID-5,REDACTED",                        // Replace patient name unconditionally
  "PID-7,$HASH",                           // Hash date of birth
  "PID-3,$REMOVE,PID-3.5 !IN (PT;PI;MA)"  // Remove identifier unless it's a trusted type
)

val (cleanMessage, report) = deidentifier.deIdentifyMessage(message, rules)

report.forEach { r =>
  println(s"Redacted ${r.path} on line ${r.lineNumber}: ${r.rule}")
}

// Write to file
import gov.cdc.utils.FileUtils
FileUtils.writeToFile("/data/output/deidentified.hl7", cleanMessage)
```
