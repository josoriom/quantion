import { describe, it, expect, beforeAll } from "@jest/globals";
import * as path from "path";
import { NodeBackend } from "../utilities/backendNode";
import {
  setBackend,
  parseIon,
  parseMzml,
  writeIon,
  getScans,
} from "../utilities/api";
import { QuantionError } from "../utilities/errors";

const sampleFile = path.join(
  __dirname,
  "../../../../core/tests/fixtures/api/api.ion",
);

function getNodeBackend(): NodeBackend | null {
  try {
    return new NodeBackend(process);
  } catch {
    return null;
  }
}

const nodeBackend = getNodeBackend();
const describeWithNode = nodeBackend ? describe : describe.skip;

describeWithNode("getScans", () => {
  beforeAll(() => {
    setBackend(nodeBackend!);
  });

  it("gives every scan inside an rt range", async () => {
    const scans = await getScans(await parseIon(sampleFile), {
      rt: { from: 2.3, to: 2.5 },
    });
    expect(scans.length).toBeGreaterThan(0);
    expect(scans.every((scan) => scan.rt >= 2.3 && scan.rt <= 2.5)).toBe(true);
  });

  it("gives the closest scan for one rt", async () => {
    const scans = await getScans(await parseIon(sampleFile), { rt: 2.385 });
    expect(scans).toHaveLength(1);
    expect(Math.abs(scans[0].rt - 2.385)).toBeLessThan(0.05);
    expect(scans[0].level).toBe(1);
  });

  it("refuses an mz that is not positive", async () => {
    const error = await getScans(await parseIon(sampleFile), { mz: 0 }).catch(
      (caught) => caught,
    );
    expect(error).toBeInstanceOf(QuantionError);
    expect(error.message).toBe("getScans: mz must be greater than 0, got 0");
    expect(error.argument).toBe("mz");
  });
});

const FINGERPRINT = { key: "fingerprint", accession: "MS:1000786" };

function getFingerprint(position: number): Float64Array {
  return Float64Array.from(
    { length: 256 },
    (_, index) => (index - 128) / 256 + position,
  );
}

function writeArray(
  accession: string,
  name: string,
  value: string,
  values: ArrayLike<number>,
): string {
  const encoded = Buffer.from(Float64Array.from(values).buffer).toString(
    "base64",
  );
  return `<binaryDataArray arrayLength="${values.length}" encodedLength="${encoded.length}"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/><cvParam cvRef="MS" accession="MS:1000576" name="no compression" value=""/><cvParam cvRef="MS" accession="${accession}" name="${name}" value="${value}"/><binary>${encoded}</binary></binaryDataArray>`;
}

function writeMzml(): Uint8Array {
  const spectra = [0, 1, 2]
    .map(
      (position) =>
        `<spectrum index="${position}" id="scan=${position + 1}" defaultArrayLength="3"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="2"/><cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/><scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="${position + 1}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><precursorList count="1"><precursor><selectedIonList count="1"><selectedIon><cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="${300 + position}"/></selectedIon></selectedIonList></precursor></precursorList><binaryDataArrayList count="3">${writeArray("MS:1000514", "m/z array", "", [100.5, 200.25, 300.125])}${writeArray("MS:1000515", "intensity array", "", [10, 20, 30])}${writeArray("MS:1000786", "non-standard data array", "ms2_fingerprint", getFingerprint(position))}</binaryDataArrayList></spectrum>`,
    )
    .join("");
  return new TextEncoder().encode(
    `<?xml version="1.0"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><cvList count="1"><cv id="MS" fullName="x" URI="x"/></cvList><run id="r"><spectrumList count="3" defaultDataProcessingRef="d">${spectra}</spectrumList></run></mzML>`,
  );
}

const describeArrays = nodeBackend ? describe : describe.skip;

describeArrays("getScans arrays", () => {
  const wholeRun = { rt: { from: 0, to: 10 } };

  async function openFile() {
    const mzml = await parseMzml(writeMzml());
    const bytes = (await writeIon(mzml)) as Uint8Array;
    return parseIon(bytes);
  }

  beforeAll(() => setBackend(nodeBackend as NodeBackend));

  it("gives mz and intensity only by default", async () => {
    const scans = await getScans(await openFile(), wholeRun, { level: 2 });
    expect(scans).toHaveLength(3);
    expect(Array.from(scans[0].mz)).toEqual([100.5, 200.25, 300.125]);
    expect(Array.from(scans[0].intensity)).toEqual([10, 20, 30]);
    expect(scans[0].fingerprint).toBeUndefined();
  });

  it("reads a custom array back exactly with negatives", async () => {
    const scans = await getScans(await openFile(), wholeRun, {
      level: 2,
      arrays: [FINGERPRINT],
    });
    expect(scans).toHaveLength(3);
    scans.forEach((scan, position) => {
      expect(Array.from(scan.fingerprint as Float64Array)).toEqual(
        Array.from(getFingerprint(position)),
      );
      expect(scan.mz).toHaveLength(0);
      expect(scan.intensity).toHaveLength(0);
      expect(scan.precursorMz).toBe(300 + position);
    });
  });

  it("reads a custom array with mz and intensity", async () => {
    const scans = await getScans(
      await openFile(),
      { mz: { from: 300.5, to: 301.5 } },
      {
        level: 2,
        arrays: [
          { key: "mz", accession: "MS:1000514" },
          { key: "intensity", accession: "MS:1000515" },
          FINGERPRINT,
        ],
      },
    );
    expect(scans).toHaveLength(1);
    expect(Array.from(scans[0].mz)).toEqual([100.5, 200.25, 300.125]);
    expect(Array.from(scans[0].fingerprint as Float64Array)).toEqual(
      Array.from(getFingerprint(1)),
    );
  });

  it("refuses bad arrays", async () => {
    const file = await openFile();
    const bad: unknown[] = [
      [],
      "mz",
      [{ key: "", accession: "MS:1000786" }],
      [{ key: "rt", accession: "MS:1000786" }],
      [{ key: "fingerprint", accession: "1000786" }],
      [{ key: "mz", accession: "MS:1000786" }],
      [{ key: "masses", accession: "MS:1000514" }],
      [FINGERPRINT, FINGERPRINT],
      [{ key: "fingerprint" }],
      [{ key: "fingerprint", accession: "MS:1000786", name: "x" }],
    ];
    for (const arrays of bad) {
      await expect(
        getScans(file, wholeRun, { arrays } as never),
      ).rejects.toBeInstanceOf(QuantionError);
    }
  });
});
