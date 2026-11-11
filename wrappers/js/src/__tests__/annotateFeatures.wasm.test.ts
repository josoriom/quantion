import { describe, it, expect, beforeAll, afterAll } from "@jest/globals";
import * as fs from "fs";
import * as path from "path";
import { folder, samples, features, startServer, FixtureServer } from "./annotateFixtures";

const bundlePath = path.join(__dirname, "../../dist/quantion.js");
const describeWithBundle = fs.existsSync(bundlePath) ? describe : describe.skip;

describeWithBundle("annotateFeatures on wasm", () => {
  let server: FixtureServer;
  let quantion: any;

  beforeAll(async () => {
    server = await startServer();
    quantion = require(bundlePath);
    await quantion.init();
  });

  afterAll(() => server.close());

  async function getFiles() {
    return Promise.all(
      samples.map((name) => quantion.parseIon(new Uint8Array(fs.readFileSync(path.join(folder, `${name}.ion`))))),
    );
  }

  function getOptions(model = "model", changes: Record<string, unknown> = {}) {
    return { library: server.url("library.ion"), model: server.url(`${model}.onnx`), ...changes };
  }

  it("names the feature and fetches each source once", async () => {
    const files = await getFiles();
    for (const name of Object.keys(server.requests)) delete server.requests[name];
    const [first] = await quantion.annotateFeatures(files, features, getOptions());
    expect([first.label, first.id, first.database]).toEqual(["Caffeine", "ENTRY_1", "SOURCE_A"]);
    expect(first.frequency).toBeCloseTo(1 / 3);
    expect(first.files[0].matches.map((found: { label: string }) => found.label)).toEqual(["Caffeine", "Compound Y"]);
    expect(Array.from(first.files[0].matches[0].mz)).toEqual([110.0713, 123.0427, 138.0662]);
    expect(server.requests).toEqual({ "library.ion": 4, "model.onnx": 1 });
    files.forEach((file) => file.dispose());
  });

  it("refuses a model without a key", async () => {
    const files = await getFiles();
    await expect(quantion.annotateFeatures(files, features, getOptions("model_without_label"))).rejects.toThrow("label");
    files.forEach((file) => file.dispose());
  });
});
