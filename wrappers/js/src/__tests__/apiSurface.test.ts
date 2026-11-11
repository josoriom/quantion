import { describe, test, expect, beforeAll } from "@jest/globals";
import * as fs from "fs";
import * as path from "path";
import { NodeBackend } from "../utilities/backendNode";

const removedNames = [
  "mzmlToIonFile",
  "mzmlToIon",
  "ionToMzml",
  "parseMzML",
  "drawPeak",
  "toNumberArrays",
];

const publicNames = [
  "parseIon",
  "parseMzml",
  "writeIon",
  "writeMzml",
  "getScans",
  "calculateEic",
  "getIonImage",
  "findPeaks",
  "getPeak",
  "fitPeak",
  "drawPeaks",
  "findNoiseLevel",
  "calculateBaseline",
  "findTargets",
  "findTransitions",
  "findFeatures",
  "matchFeatures",
  "loadModel",
  "loadWarp",
];

const modulePaths: Array<[string, string]> = [
  ["source", "../utilities/api"],
  ["built", "../../lib/utilities/api.js"],
];

describe.each(modulePaths)("api surface (%s)", (_name, modulePath) => {
  const api = require(modulePath);

  test.each(removedNames)("%s is not exported", (name) => {
    expect(name in api).toBe(false);
  });

  test.each(publicNames)("%s is exported", (name) => {
    expect(typeof api[name]).toBe("function");
  });
});

describe("api surface (shipped types)", () => {
  const declarations = fs.readFileSync(
    path.join(__dirname, "..", "..", "lib", "utilities", "api.d.ts"),
    "utf8",
  );

  test.each(removedNames)("%s is not declared", (name) => {
    expect(declarations).not.toContain(`declare function ${name}(`);
  });

  test.each(publicNames)("%s is declared", (name) => {
    expect(declarations).toContain(`declare function ${name}(`);
  });
});

function hasNativeLibrary(): boolean {
  try {
    new NodeBackend(process);
    return true;
  } catch {
    return false;
  }
}

const describeWithNode = hasNativeLibrary() ? describe : describe.skip;

describeWithNode("enums and errors are exported from the entry point", () => {
  let quantion: any;

  beforeAll(() => {
    quantion = require("../../lib/index-node.js");
  });

  test("TimeUnit and PeakShape are uppercase", () => {
    expect(quantion.TimeUnit).toEqual({
      MINUTES: "MINUTES",
      SECONDS: "SECONDS",
    });
    expect(quantion.PeakShape).toEqual({ GAUSSIAN: "GAUSSIAN", EMG: "EMG" });
  });

  test("ScanParamKind is uppercase", () => {
    expect(quantion.ScanParamKind).toEqual({
      ATTRIBUTE: "ATTRIBUTE",
      CV: "CV",
      USER: "USER",
    });
  });

  test("QuantionError is exported", () => {
    expect(typeof quantion.QuantionError).toBe("function");
  });
});
