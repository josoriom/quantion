import { describe, it, expect, beforeAll } from "@jest/globals";
import * as fs from "fs";
import * as path from "path";
import { NodeBackend } from "../utilities/backendNode";

const bundlePath = path.join(__dirname, "../../dist/quantion.js");
const SCAN_COUNT = 20;
const PEAK_COUNT = 50;
const MEGABYTE = 1024 * 1024;
const BLOCK_SIZE_OFFSET = 16;
const MZ_WINDOW_OFFSET = 24;
const everyScan = { rt: { from: 0, to: 100 } };

function writeArray(accession: string, name: string, values: number[]): string {
  const encoded = Buffer.from(Float64Array.from(values).buffer).toString(
    "base64",
  );
  return `<binaryDataArray encodedLength="${encoded.length}"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/><cvParam cvRef="MS" accession="MS:1000576" name="no compression"/><cvParam cvRef="MS" accession="${accession}" name="${name}"/><binary>${encoded}</binary></binaryDataArray>`;
}

function writeSpectrum(position: number): string {
  const peaks = Array.from({ length: PEAK_COUNT }, (_, peak) => peak);
  const mz = peaks.map((peak) => 100 + 18 * peak + position * 0.01);
  const intensity = peaks.map((peak) => 1000 + 10 * peak + position);
  return `<spectrum index="${position}" id="scan=${position + 1}" defaultArrayLength="${PEAK_COUNT}"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/><scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="${position * 0.1}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><binaryDataArrayList count="2">${writeArray("MS:1000514", "m/z array", mz)}${writeArray("MS:1000515", "intensity array", intensity)}</binaryDataArrayList></spectrum>`;
}

function writeMzml(): Uint8Array {
  const spectra = Array.from({ length: SCAN_COUNT }, (_, position) =>
    writeSpectrum(position),
  ).join("");
  return new TextEncoder().encode(
    `<?xml version="1.0"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><cvList count="1"><cv id="MS" fullName="x" URI="x"/></cvList><run id="r"><spectrumList count="${SCAN_COUNT}" defaultDataProcessingRef="d">${spectra}</spectrumList></run></mzML>`,
  );
}

function readHeader(ion: Uint8Array): [number, number] {
  const view = new DataView(ion.buffer, ion.byteOffset, ion.byteLength);
  return [
    Number(view.getBigUint64(BLOCK_SIZE_OFFSET, true)),
    view.getUint32(MZ_WINDOW_OFFSET, true),
  ];
}

function hasNativeLibrary(): boolean {
  try {
    new NodeBackend(process);
    return true;
  } catch {
    return false;
  }
}

const builds: Array<[string, () => any]> = [];
if (hasNativeLibrary()) {
  builds.push(["node", () => require("../../lib/index-node.js")]);
}
if (fs.existsSync(bundlePath)) builds.push(["wasm", () => require(bundlePath)]);

describe.each(builds)("writeIon blockSize and mzWindow (%s)", (_name, load) => {
  let quantion: any;
  let file: any;

  beforeAll(async () => {
    quantion = load();
    if (quantion.init) await quantion.init();
    file = await quantion.parseMzml(writeMzml());
  });

  async function getPeaks(sample: unknown): Promise<number[][][]> {
    const scans = await quantion.getScans(sample, everyScan, { level: 0 });
    return scans.map((scan: { mz: Float64Array; intensity: Float64Array }) => [
      Array.from(scan.mz),
      Array.from(scan.intensity),
    ]);
  }

  it("defaults to 8 MB blocks and 20 m/z windows", async () => {
    expect(readHeader(await quantion.writeIon(file))).toEqual([
      8 * MEGABYTE,
      20,
    ]);
  });

  it("records chosen values and reads the same peaks back", async () => {
    const expected = await getPeaks(file);
    expect(expected.length).toBe(SCAN_COUNT);
    const choices: Array<[number, number]> = [
      [1, 20],
      [2, 5],
      [16, 0],
      [8, 100],
    ];
    for (const [blockSize, mzWindow] of choices) {
      const ion = await quantion.writeIon(file, { blockSize, mzWindow });
      expect(readHeader(ion)).toEqual([blockSize * MEGABYTE, mzWindow]);
      expect(await getPeaks(await quantion.parseIon(ion))).toEqual(expected);
    }
  });

  it("refuses a bad blockSize", async () => {
    for (const blockSize of [
      0,
      -1,
      1.5,
      4294967296,
      "8",
      null,
      true,
      NaN,
      [8],
    ]) {
      const error = await quantion
        .writeIon(file, { blockSize })
        .catch((caught: unknown) => caught);
      expect(error).toBeInstanceOf(quantion.QuantionError);
      expect(error.kind).toBe("invalid_argument");
      expect(error.argument).toBe("blockSize");
    }
  });

  it("refuses a bad mzWindow", async () => {
    for (const mzWindow of [
      -1,
      2.5,
      4294967296,
      "20",
      null,
      true,
      Infinity,
      [20],
    ]) {
      const error = await quantion
        .writeIon(file, { mzWindow })
        .catch((caught: unknown) => caught);
      expect(error).toBeInstanceOf(quantion.QuantionError);
      expect(error.kind).toBe("invalid_argument");
      expect(error.argument).toBe("mzWindow");
    }
  });

  it("refuses an unknown option", async () => {
    const error = await quantion
      .writeIon(file, { blocksize: 8 })
      .catch((caught: unknown) => caught);
    expect(error.kind).toBe("unknown_option");
  });
});
