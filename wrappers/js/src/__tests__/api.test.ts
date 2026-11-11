import { describe, it, expect, beforeAll } from "@jest/globals";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { NodeBackend } from "../utilities/backendNode";

const fixture = path.join(
  __dirname,
  "../../../../core/tests/fixtures/api/api.ion",
);
const bundlePath = path.join(__dirname, "../../dist/quantion.js");
const ALANINE = 89.04768;

function getNode() {
  return require("../../lib/index-node.js");
}

function getWasm() {
  return require(bundlePath);
}

function hasNativeLibrary(): boolean {
  try {
    new NodeBackend(process);
    return true;
  } catch {
    return false;
  }
}

const describeWithNode = hasNativeLibrary() ? describe : describe.skip;

const builds: Array<[string, () => any]> = [];
if (hasNativeLibrary()) builds.push(["node", getNode]);
if (fs.existsSync(bundlePath)) builds.push(["wasm", getWasm]);

function readFixture(): Uint8Array {
  return new Uint8Array(fs.readFileSync(fixture));
}

describe.each(builds)("api on %s", (_name, load) => {
  let quantion: any;
  let file: any;

  beforeAll(async () => {
    quantion = load();
    await quantion.init();
    file = await quantion.parseIon(readFixture());
  });

  it("reads the window from the eic options", async () => {
    const whole = await quantion.calculateEic(file, ALANINE);
    const part = await quantion.calculateEic(file, ALANINE, { from: 2, to: 3 });
    expect(part.x.length).toBeLessThan(whole.x.length);
    expect(
      Array.from(part.x).every(
        (rt) => (rt as number) >= 2 && (rt as number) <= 3,
      ),
    ).toBe(true);
  });

  it("gives the same window in seconds", async () => {
    const minutes = await quantion.calculateEic(file, ALANINE, {
      from: 2,
      to: 3,
    });
    const seconds = await quantion.calculateEic(file, ALANINE, {
      from: 120,
      to: 180,
      timeUnit: quantion.TimeUnit.SECONDS,
    });
    expect(Array.from(seconds.x)).toEqual(Array.from(minutes.x));
  });

  it("returns peaks with only the public fields", async () => {
    const eic = await quantion.calculateEic(file, ALANINE, { from: 0, to: 5 });
    const peaks = quantion.findPeaks(eic.x, eic.y);
    expect(peaks.length).toBeGreaterThan(0);
    expect(Object.keys(peaks[0]).sort()).toEqual([
      "from",
      "integral",
      "intensity",
      "nPoints",
      "rt",
      "to",
    ]);
  });

  it("finds a target without a range", async () => {
    const peaks = await quantion.findTargets(file, [
      { mz: ALANINE, rt: 2.385 },
    ]);
    expect(peaks).toHaveLength(1);
    expect(peaks[0].intensity).toBeGreaterThan(0);
  });

  it("gives a list for one closest scan", async () => {
    const scans = await quantion.getScans(file, { rt: 2.385 });
    expect(scans).toHaveLength(1);
    expect(scans[0].level).toBe(1);
    expect(typeof scans[0].x).toBe("number");
  });

  it("draws peaks that add up", () => {
    const drawn = quantion.drawPeaks(
      [
        { height: 100, center: 2, fwhm: 0.05, tail: 0 },
        { height: 50, center: 2, fwhm: 0.05, tail: 0 },
      ],
      { from: 1.5, to: 2.5, points: 101 },
    );
    expect(Math.max(...drawn.y)).toBeCloseTo(150, 6);
  });

  it("writes ion and mzML bytes that open again", async () => {
    const ion = await quantion.writeIon(file);
    const again = await quantion.parseIon(ion);
    const first = await quantion.calculateEic(file, ALANINE);
    const second = await quantion.calculateEic(again, ALANINE);
    expect(Array.from(second.y)).toEqual(Array.from(first.y));
    const mzml = await quantion.writeMzml(file);
    const fromMzml = await quantion.parseMzml(mzml);
    expect(await quantion.getScans(fromMzml, { rt: 2.385 })).toHaveLength(1);
  });

  it("names the function and the argument in a Rust error", async () => {
    const error = await quantion
      .calculateEic(file, -1)
      .catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(quantion.QuantionError);
    expect(error.kind).toBe("invalid_argument");
    expect(error.code).toBe(1);
    expect(error.argument).toBe("mz");
    expect(error.message).toBe(
      "calculateEic: mz must be greater than 0, got -1",
    );
  });

  it("refuses an unknown option key at any depth", async () => {
    let caught: any;
    try {
      await quantion.findTargets(file, [{ mz: ALANINE, rt: 2.385 }], {
        findPeaks: { minIntensty: 1 },
      });
    } catch (error) {
      caught = error;
    }
    expect(caught).toBeInstanceOf(quantion.QuantionError);
    expect(caught.kind).toBe("unknown_option");
    expect(caught.code).toBe(3);
    expect(caught.argument).toBe("findPeaks.minIntensty");
  });

  it("refuses a target, transition, peak or range with an unknown key", async () => {
    const cases: Array<[() => unknown, string]> = [
      [
        () =>
          quantion.findTargets(file, [{ mz: ALANINE, rt: 2.385, rnage: 0.1 }]),
        "findTargets: targets[0].rnage is not an option; the options are mz, rt, id, range",
      ],
      [
        () =>
          quantion.findTransitions(file, [
            {
              precursor: ALANINE,
              product: ALANINE,
              rt: 2.385,
              range: 0.1,
              prodcut: 1,
            },
          ]),
        "findTransitions: targets[0].prodcut is not an option; the options are precursor, product, rt, range, id",
      ],
      [
        () =>
          quantion.drawPeaks([
            { height: 1, center: 2, fwhm: 0.1, tail: 0, hieght: 5 },
          ]),
        "drawPeaks: params[0].hieght is not an option; the options are height, center, fwhm, tail, r2",
      ],
      [
        () => quantion.getScans(file, { rt: { from: 1, too: 2 } }),
        "getScans: rt.too is not an option; the options are from, to",
      ],
    ];
    for (const [call, message] of cases) {
      let caught: any;
      try {
        await call();
      } catch (error) {
        caught = error;
      }
      expect(caught).toBeInstanceOf(quantion.QuantionError);
      expect(caught.kind).toBe("unknown_option");
      expect(caught.message).toBe(message);
    }
  });

  it("reports a missing required field as missing_argument", async () => {
    const cases: Array<[() => unknown, string]> = [
      [
        () => quantion.drawPeaks([{ height: 1, center: 2, fwhm: 0.1 }]),
        "drawPeaks: params[0].tail is required",
      ],
      [
        () => quantion.findTargets(file, [{ rt: 2.385 }]),
        "findTargets: targets[0].mz is required",
      ],
      [
        () => quantion.getScans(file, { rt: { from: 1 } }),
        "getScans: rt.to is required",
      ],
    ];
    for (const [call, message] of cases) {
      let caught: any;
      try {
        await call();
      } catch (error) {
        caught = error;
      }
      expect(caught).toBeInstanceOf(quantion.QuantionError);
      expect(caught.kind).toBe("missing_argument");
      expect(caught.message).toBe(message);
    }
  });

  it("refuses an enum value that is not uppercase", () => {
    expect(() =>
      quantion.findPeaks(new Float64Array(3), new Float64Array(3), {
        shape: "emg",
      }),
    ).toThrow("findPeaks: shape must be one of GAUSSIAN, EMG, got emg");
  });

  it("refuses headers without a URL", async () => {
    const error = await quantion
      .writeIon(file, {
        headers: [{ name: "Authorization", value: "Bearer x" }],
      })
      .catch((caught: unknown) => caught);
    expect(error.argument).toBe("headers");
  });

  it("refuses x and y of different lengths", () => {
    expect(() =>
      quantion.findPeaks(new Float64Array(5), new Float64Array(4)),
    ).toThrow("findPeaks: y must have the same length as x (5), got 4");
  });
});

describeWithNode("node only", () => {
  it("fits a drawn peak back", async () => {
    const quantion = getNode();
    const tall = quantion.drawPeaks(
      [{ height: 1000, center: 2, fwhm: 0.05, tail: 0 }],
      {
        from: 1,
        to: 3,
        points: 401,
      },
    );
    const fitted = quantion.fitPeak(tall.x, tall.y, 2, {
      shape: quantion.PeakShape.GAUSSIAN,
    });
    expect(fitted.center).toBeCloseTo(2, 2);
    expect(fitted.tail).toBe(0);
  });

  it("needs overwrite to replace a file", async () => {
    const quantion = getNode();
    const file = await quantion.parseIon(fixture);
    const folder = fs.mkdtempSync(path.join(os.tmpdir(), "quantion-js-"));
    const out = path.join(folder, "sample.ion");
    expect(await quantion.writeIon(file, { out })).toBeUndefined();
    const error = await quantion
      .writeIon(file, { out })
      .catch((caught: unknown) => caught);
    expect(error.kind).toBe("file_exists");
    await quantion.writeIon(file, { out, overwrite: true });
    fs.rmSync(folder, { recursive: true });
  });

  it("refuses a disposed file", async () => {
    const quantion = getNode();
    const file = await quantion.parseIon(fixture);
    file.dispose();
    const error = await quantion
      .calculateEic(file, ALANINE)
      .catch((caught: unknown) => caught);
    expect(error.message).toBe(
      "calculateEic: file is not open; it was disposed or never opened",
    );
  });
});
