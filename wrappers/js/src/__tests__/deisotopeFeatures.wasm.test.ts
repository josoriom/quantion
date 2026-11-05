import { describe, it, expect } from "@jest/globals";
import * as fs from "fs";
import * as path from "path";

const bundlePath = path.join(__dirname, "../../dist/quantion.js");
const fixture = path.join(
  __dirname,
  "../../../../core/tests/fixtures/api/api.ion",
);
const ISOTOPE_DIFF = 1.003354835;
const describeWithBundle = fs.existsSync(bundlePath) ? describe : describe.skip;

function makeFeature(mz: number, intensity: number) {
  return {
    mz,
    rt: 5,
    from: 4.9,
    to: 5.1,
    intensity,
    integral: intensity,
    nPoints: 9,
  };
}

describeWithBundle("deisotopeFeatures on wasm", () => {
  async function getQuantion() {
    const quantion = require(bundlePath);
    await quantion.init();
    return quantion;
  }

  it("removes the isotope and sorts by m/z", async () => {
    const quantion = await getQuantion();
    const found = quantion.deisotopeFeatures([
      makeFeature(300, 500),
      makeFeature(200 + ISOTOPE_DIFF, 100),
      makeFeature(200, 1000),
    ]);
    expect(found.map((feature: { mz: number }) => feature.mz)).toEqual([
      200, 300,
    ]);
    expect(found[0].nPoints).toBe(9);
    expect(found[0].frequency).toBeUndefined();
  });

  it("keeps frequency", async () => {
    const quantion = await getQuantion();
    const found = quantion.deisotopeFeatures([
      { ...makeFeature(200, 1000), frequency: 0.5 },
    ]);
    expect(found[0].frequency).toBe(0.5);
  });

  it("cleans the features of a real file", async () => {
    const quantion = await getQuantion();
    const file = await quantion.parseIon(
      new Uint8Array(fs.readFileSync(fixture)),
    );
    const features = await quantion.findFeatures(file, {
      grid: { start: 85, end: 95 },
    });
    expect(features.length).toBeGreaterThan(0);
    expect(quantion.deisotopeFeatures(features).length).toBeGreaterThan(0);
  });

  it("rejects invalid values", async () => {
    const quantion = await getQuantion();
    expect(() => quantion.deisotopeFeatures([], { rtTolerance: -1 })).toThrow();
  });
});
