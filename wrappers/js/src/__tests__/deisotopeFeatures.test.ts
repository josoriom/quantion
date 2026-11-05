import { describe, it, expect, beforeAll } from "@jest/globals";
import { NodeBackend } from "../utilities/backendNode";
import { setBackend, deisotopeFeatures } from "../utilities/api";
import { QuantionError } from "../utilities/errors";
import type { Feature } from "../types/types";

const ISOTOPE_DIFF = 1.003354835;

function makeFeature(
  mz: number,
  intensity: number,
  frequency?: number,
): Feature {
  return {
    mz,
    rt: 5,
    from: 4.9,
    to: 5.1,
    intensity,
    integral: intensity,
    nPoints: 9,
    ...(frequency === undefined ? {} : { frequency }),
  };
}

function getNodeBackend(): NodeBackend | null {
  try {
    return new NodeBackend(process);
  } catch {
    return null;
  }
}

const backend = getNodeBackend();
const describeNative = backend ? describe : describe.skip;

describeNative("deisotopeFeatures", () => {
  beforeAll(() => {
    if (backend) setBackend(backend);
  });

  it("removes the isotope and sorts by m/z", () => {
    const found = deisotopeFeatures([
      makeFeature(300, 500),
      makeFeature(200 + ISOTOPE_DIFF, 100),
      makeFeature(200, 1000),
    ]);
    expect(found.map((feature) => feature.mz)).toEqual([200, 300]);
    expect(found[0].nPoints).toBe(9);
    expect(found[0].frequency).toBeUndefined();
  });

  it("keeps frequency", () => {
    expect(deisotopeFeatures([makeFeature(200, 1000, 0.5)])[0].frequency).toBe(
      0.5,
    );
  });

  it("uses the options", () => {
    const features = [
      makeFeature(200, 1000),
      makeFeature(200 + ISOTOPE_DIFF + 0.003, 100),
    ];
    expect(deisotopeFeatures(features)).toHaveLength(1);
    expect(deisotopeFeatures(features, { mzTolerance: 0.001 })).toHaveLength(2);
  });

  it("accepts an empty list", () => {
    expect(deisotopeFeatures([])).toEqual([]);
  });

  it("rejects unknown keys and invalid values", () => {
    const options = { window: 1 } as never;
    expect(() => deisotopeFeatures([], options)).toThrow(QuantionError);
    expect(() => deisotopeFeatures([], { rtTolerance: -1 })).toThrow(
      QuantionError,
    );
    const extra = [{ ...makeFeature(200, 1000), extra: 1 }];
    expect(() => deisotopeFeatures(extra)).toThrow(QuantionError);
    expect(() => deisotopeFeatures([{ mz: 200 } as Feature])).toThrow(
      QuantionError,
    );
    expect(() => deisotopeFeatures("features" as never)).toThrow(QuantionError);
  });
});
