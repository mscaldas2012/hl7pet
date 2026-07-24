package gov.cdc.hl7.bench;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;

/**
 * Host/environment metadata recorded alongside a Benchmark Run (data-model.md), so a
 * reader can tell whether two runs are directly comparable (spec.md Edge Cases: results
 * are not portable across machines). Best-effort: CPU model detection shells out to a
 * platform command and falls back to JVM-reported arch/core-count if that fails.
 */
public final class HostEnvironment {

  public final String cpu;
  public final String os;
  public final String jdkVendorVersion;
  public final String jmhVersion;

  private HostEnvironment(String cpu, String os, String jdkVendorVersion, String jmhVersion) {
    this.cpu = cpu;
    this.os = os;
    this.jdkVendorVersion = jdkVendorVersion;
    this.jmhVersion = jmhVersion;
  }

  public static HostEnvironment capture() {
    return new HostEnvironment(
        cpuModel(),
        System.getProperty("os.name") + " " + System.getProperty("os.version"),
        System.getProperty("java.vendor") + " " + System.getProperty("java.version"),
        org.openjdk.jmh.util.Version.getPlainVersion());
  }

  private static String cpuModel() {
    String osName = System.getProperty("os.name", "").toLowerCase();
    try {
      if (osName.contains("mac")) {
        String brand = runCommand("sysctl", "-n", "machdep.cpu.brand_string");
        if (brand != null && !brand.isBlank()) {
          return brand.trim();
        }
      } else if (osName.contains("linux")) {
        String modelName = readProcCpuinfoModelName();
        if (modelName != null && !modelName.isBlank()) {
          return modelName.trim();
        }
      }
    } catch (Exception e) {
      // Fall through to the generic fallback below -- CPU model is diagnostic
      // metadata, not something a benchmark run should fail over.
    }
    return System.getProperty("os.arch", "unknown") + " (" + Runtime.getRuntime().availableProcessors()
        + " cores, model name unavailable)";
  }

  private static String runCommand(String... command) throws Exception {
    Process process = new ProcessBuilder(command).redirectErrorStream(true).start();
    try (BufferedReader reader =
        new BufferedReader(new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8))) {
      String output = reader.readLine();
      process.waitFor();
      return output;
    }
  }

  private static String readProcCpuinfoModelName() throws Exception {
    try (BufferedReader reader =
        new BufferedReader(new InputStreamReader(
            java.nio.file.Files.newInputStream(java.nio.file.Path.of("/proc/cpuinfo")),
            StandardCharsets.UTF_8))) {
      String line;
      while ((line = reader.readLine()) != null) {
        if (line.startsWith("model name")) {
          int colon = line.indexOf(':');
          return colon >= 0 ? line.substring(colon + 1) : line;
        }
      }
    }
    return null;
  }
}
