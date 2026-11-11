import { describe, it, expect } from "@jest/globals";
import * as fs from "fs";
import * as path from "path";

const bundlePath = path.join(__dirname, "../../dist/quantion.js");
const fixtures = path.join(__dirname, "../../../../core/tests/fixtures");
const targets = [
  { mz: 250.1, rt: 1.2 },
  { mz: 310.2, rt: 3.1 },
];
const describeWithBundle = fs.existsSync(bundlePath) ? describe : describe.skip;

describeWithBundle("warp model on wasm", () => {
  function getQuantion() {
    return require(bundlePath);
  }

  function readFixture(name: string): Uint8Array {
    return new Uint8Array(fs.readFileSync(path.join(fixtures, name)));
  }

  it("predicts from model bytes and a file in memory", async () => {
    const quantion = getQuantion();
    await quantion.init();
    const warp = quantion.loadWarp(
      await quantion.loadModel(readFixture("shift_model.onnx")),
    );
    const file = await quantion.parseIon(readFixture("api/api.ion"));
    const predicted = await warp.predictRts(file, targets);
    expect(predicted).toHaveLength(targets.length);
    predicted.forEach((prediction: { rt: number; uncertainty: number }) => {
      expect(Number.isFinite(prediction.rt)).toBe(true);
      expect(prediction.uncertainty).toBeGreaterThan(0);
    });
    file.dispose();
    warp.dispose();
  });

  it("names a target without mz by its number", async () => {
    const quantion = getQuantion();
    await quantion.init();
    const warp = quantion.loadWarp(
      await quantion.loadModel(readFixture("shift_model.onnx")),
    );
    const file = await quantion.parseIon(readFixture("api/api.ion"));
    await expect(
      warp.predictRts(file, [targets[0], { mz: NaN, rt: 2 }]),
    ).rejects.toThrow("predictRts: target 2 has no valid m/z or rt");
    file.dispose();
    warp.dispose();
  });

  it("refuses a model file path", async () => {
    const quantion = getQuantion();
    await quantion.init();
    await expect(
      quantion.loadModel(path.join(fixtures, "shift_model.onnx")),
    ).rejects.toThrow(
      "loadModel: source must be a URL or bytes in the browser",
    );
  });
});
