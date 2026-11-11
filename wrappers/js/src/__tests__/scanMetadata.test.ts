import { describe, it, expect, beforeAll, afterAll } from "@jest/globals";
import { NodeBackend } from "../utilities/backendNode";
import {
  setBackend,
  parseIon,
  parseMzml,
  writeIon,
  getScans,
  getScanMetadata,
} from "../utilities/api";
import { QuantionError } from "../utilities/errors";
import type { SampleFile } from "../utilities/sampleFile";
import {
  SCAN_COUNT,
  type LibraryServer,
  getExpectedRows,
  getLibraryId,
  getRowLibraryId,
  startLibraryServer,
  writeLibraryMzml,
} from "./syntheticLibrary";

function getNodeBackend(): NodeBackend | null {
  try {
    return new NodeBackend(process);
  } catch {
    return null;
  }
}

const nodeBackend = getNodeBackend();
const describeWithNode = nodeBackend ? describe : describe.skip;
const everyScan = { rt: { from: 0, to: 100 } };

describeWithNode("getScanMetadata", () => {
  let mzml: SampleFile;
  let ion: SampleFile;
  let server: LibraryServer;

  beforeAll(async () => {
    setBackend(nodeBackend!);
    mzml = await parseMzml(writeLibraryMzml());
    const library = (await writeIon(mzml)) as Uint8Array;
    ion = await parseIon(library);
    server = await startLibraryServer(library);
  });

  afterAll(() => server.stop());

  it("gives each scan its position in the spectrum list", async () => {
    for (const file of [mzml, ion]) {
      const all = await getScans(file, everyScan, { level: 0 });
      expect(all.map((scan) => scan.index)).toEqual([0, 1, 2, 3, 4, 5]);
      const levelTwo = await getScans(file, everyScan, { level: 2 });
      expect(levelTwo.map((scan) => scan.index)).toEqual([1, 3, 5]);
    }
  });

  it("sets the index when reading chosen arrays", async () => {
    for (const file of [mzml, ion]) {
      const scans = await getScans(file, everyScan, {
        level: 2,
        arrays: [{ key: "intensity", accession: "MS:1000515" }],
      });
      expect(scans.map((scan) => scan.index)).toEqual([1, 3, 5]);
    }
  });

  it("gives every tag in tree order", async () => {
    for (const file of [mzml, ion]) {
      for (let position = 0; position < SCAN_COUNT; position += 1) {
        expect(await getScanMetadata(file, position)).toEqual(
          getExpectedRows(position),
        );
      }
    }
  });

  it("gives the same rows from mzML and ion", async () => {
    for (let position = 0; position < SCAN_COUNT; position += 1) {
      expect(await getScanMetadata(ion, position)).toEqual(
        await getScanMetadata(mzml, position),
      );
    }
  });

  it("matches found scans by index", async () => {
    for (const scan of await getScans(ion, everyScan, { level: 2 })) {
      const rows = await getScanMetadata(ion, scan.index);
      expect(getRowLibraryId(rows)).toBe(getLibraryId(scan.index));
    }
  });

  it("gives an uppercase kind", async () => {
    const kinds = new Set(
      (await getScanMetadata(ion, 1)).map((row) => row.kind),
    );
    expect([...kinds].sort()).toEqual(["ATTRIBUTE", "CV", "USER"]);
  });

  it("gives null for a missing field", async () => {
    const rows = await getScanMetadata(ion, 0);
    const centroid = rows.find((row) => row.name === "centroid spectrum");
    expect(centroid?.value).toBeNull();
    expect(centroid?.unitAccession).toBeNull();
    expect(centroid?.accession).toBe("MS:1000127");
  });

  it("keeps only the chosen tags", async () => {
    for (const file of [mzml, ion]) {
      const ions = await getScanMetadata(file, 1, { tags: ["selectedIon"] });
      expect(ions.map((row) => row.name)).toEqual([
        "selected ion m/z",
        "charge state",
      ]);
      const chosen = await getScanMetadata(file, 1, {
        tags: ["activation", "scan"],
      });
      expect(chosen.map((row) => row.path)).toEqual([
        "spectrum/scanList/scan[0]",
        "spectrum/scanList/scan[0]",
        "spectrum/scanList/scan[0]",
        "spectrum/precursorList/precursor[0]/activation",
      ]);
    }
  });

  it("keeps every tag when tags is empty", async () => {
    expect(await getScanMetadata(ion, 1, { tags: [] })).toEqual(
      getExpectedRows(1),
    );
  });

  it("refuses bad tags", async () => {
    for (const tags of [
      ["cvParam"],
      ["Scan"],
      ["scan[0]"],
      [""],
      [1],
      "scan",
      null,
    ]) {
      const error = await getScanMetadata(ion, 0, {
        tags: tags as string[],
      }).catch((caught) => caught);
      expect(error).toBeInstanceOf(QuantionError);
      expect(error.kind).toBe("invalid_argument");
      expect(error.argument).toBe("tags");
    }
  });

  it("refuses an unknown option", async () => {
    const error = await getScanMetadata(ion, 0, {
      tag: ["scan"],
    } as never).catch((caught) => caught);
    expect(error).toBeInstanceOf(QuantionError);
    expect(error.kind).toBe("unknown_option");
    expect(error.argument).toBe("tag");
  });

  it("refuses an index out of range", async () => {
    for (const file of [mzml, ion]) {
      const error = await getScanMetadata(file, SCAN_COUNT).catch(
        (caught) => caught,
      );
      expect(error).toBeInstanceOf(QuantionError);
      expect(error.kind).toBe("invalid_argument");
      expect(error.function).toBe("getScanMetadata");
      expect(error.argument).toBe("index");
    }
  });

  it("refuses an index that is not a whole number 0 or more", async () => {
    for (const index of [-1, 1.5, NaN, Infinity, "1", true]) {
      const error = await getScanMetadata(ion, index as number).catch(
        (caught) => caught,
      );
      expect(error).toBeInstanceOf(QuantionError);
      expect(error.kind).toBe("invalid_argument");
      expect(error.argument).toBe("index");
    }
  });

  it("refuses a missing index", async () => {
    const getWithoutIndex = getScanMetadata as (
      file: SampleFile,
    ) => ReturnType<typeof getScanMetadata>;
    for (const call of [
      () => getWithoutIndex(ion),
      () => getScanMetadata(ion, undefined as unknown as number),
      () => getScanMetadata(ion, null as unknown as number),
    ]) {
      const error = await call().catch((caught) => caught);
      expect(error).toBeInstanceOf(QuantionError);
      expect(error.kind).toBe("invalid_argument");
      expect(error.argument).toBe("index");
    }
  });

  it("refuses index as an array key", async () => {
    const error = await getScans(ion, everyScan, {
      arrays: [{ key: "index", accession: "MS:1000786" }],
    }).catch((caught) => caught);
    expect(error).toBeInstanceOf(QuantionError);
    expect(error.argument).toBe("arrays.key");
  });

  it("reads over a URL with only the planned bytes", async () => {
    for (const position of [0, 3, SCAN_COUNT - 1]) {
      const file = await parseIon(server.url);
      const opened = server.requests.length;
      expect(await getScanMetadata(file, position)).toEqual(
        getExpectedRows(position),
      );
      expect(server.requests.length).toBeGreaterThan(opened);
      file.dispose();
    }
  });
});
