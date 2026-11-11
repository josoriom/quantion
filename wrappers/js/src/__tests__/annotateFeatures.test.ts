import { describe, it, expect, beforeAll, afterAll } from "@jest/globals";
import * as path from "path";
import { NodeBackend } from "../utilities/backendNode";
import { setBackend, annotateFeatures, parseIon } from "../utilities/api";
import type { SampleFile } from "../utilities/sampleFile";
import { folder, samples, features, withoutCandidates, startServer, getPlain, FixtureServer } from "./annotateFixtures";

function getNodeBackend(): NodeBackend | null {
  try {
    return new NodeBackend(process);
  } catch {
    return null;
  }
}

const nodeBackend = getNodeBackend();
const describeWithNode = nodeBackend ? describe : describe.skip;

function getOptions(model = "model", changes: Record<string, unknown> = {}) {
  return { library: path.join(folder, "library.ion"), model: path.join(folder, `${model}.onnx`), ...changes };
}

describeWithNode("annotateFeatures", () => {
  let files: SampleFile[];

  beforeAll(async () => {
    setBackend(nodeBackend!);
    files = await Promise.all(samples.map((name) => parseIon(path.join(folder, `${name}.ion`))));
  });

  afterAll(() => files.forEach((file) => file.dispose()));

  it("names the feature after the best compound", async () => {
    const [first] = await annotateFeatures(files, features, getOptions());
    expect([first.label, first.id, first.database]).toEqual(["Caffeine", "ENTRY_1", "SOURCE_A"]);
    expect(first.frequency).toBeCloseTo(1 / 3);
    expect(first.score!).toBeGreaterThan(0.99);
    expect(first.cosine!).toBeGreaterThan(0.99);
  });

  it("keeps the best matches of each file in order", async () => {
    const [first] = await annotateFeatures(files, features, getOptions());
    expect(first.files.map((file) => file.file)).toEqual([0, 1, 2]);
    expect(first.files[0].scanIndex).not.toBeNull();
    expect(first.files[1].scanIndex).toBeNull();
    const matches = first.files[0].matches;
    expect(matches.map((found) => found.label)).toEqual(["Caffeine", "Compound Y"]);
    expect(matches[0].score).toBeGreaterThanOrEqual(matches[1].score);
    expect(Array.from(matches[0].mz)).toEqual([110.0713, 123.0427, 138.0662]);
    expect(matches[0].metadata).toEqual({ family: "FAMILY_C", note: "synthetic 1" });
  });

  it("limits each file to matches", async () => {
    const [first] = await annotateFeatures(files, features, getOptions("model", { matches: 1 }));
    expect(first.files[0].matches.map((found) => found.label)).toEqual(["Caffeine"]);
  });

  it("gives no label to a feature without candidates", async () => {
    const annotations = await annotateFeatures(files, withoutCandidates, getOptions());
    for (const annotation of annotations) {
      expect(annotation.label).toBeNull();
      expect(annotation.score).toBeNull();
      expect(annotation.frequency).toBe(0);
      expect(annotation.files.every((file) => file.matches.length === 0)).toBe(true);
    }
  });

  it("gives nothing for no features and refuses no files", async () => {
    expect(await annotateFeatures(files, [], getOptions())).toEqual([]);
    await expect(annotateFeatures([], features, getOptions())).rejects.toMatchObject({ argument: "files" });
  });

  it("refuses invalid and unknown options", async () => {
    const cases: Record<string, unknown> = { matches: 0, rtRange: 0, minRelativeIntensity: 2, library: "" };
    for (const [key, value] of Object.entries(cases)) {
      await expect(annotateFeatures(files, features, getOptions("model", { [key]: value }))).rejects.toThrow(key);
    }
    await expect(annotateFeatures(files, features, getOptions("model", { color: "red" }))).rejects.toThrow("color");
  });

  it("refuses models with missing or invalid keys", async () => {
    await expect(annotateFeatures(files, features, getOptions("model_without_label"))).rejects.toThrow("label");
    await expect(annotateFeatures(files, features, getOptions("model_with_bad_max_peaks"))).rejects.toThrow("max_peaks");
  });

  it("gives the same result over URLs with one prefetch per source", async () => {
    const server: FixtureServer = await startServer();
    try {
      const local = await annotateFeatures(files, features, getOptions());
      const sample = await parseIon(server.url("sample_peak_and_ms2.ion"));
      delete server.requests["sample_peak_and_ms2.ion"];
      const options = { library: server.url("library.ion"), model: server.url("model.onnx") };
      const remote = await annotateFeatures([sample, files[1], files[2]], features, options);
      expect(getPlain(remote)).toEqual(getPlain(local));
      expect(server.requests).toEqual({ "library.ion": 4, "model.onnx": 1, "sample_peak_and_ms2.ion": 1 });
      sample.dispose();
    } finally {
      await server.close();
    }
  });
});
