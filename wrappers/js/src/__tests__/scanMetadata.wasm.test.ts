import { describe, it, expect, beforeAll, afterAll } from "@jest/globals";
import * as fs from "fs";
import * as path from "path";
import {
  SCAN_COUNT,
  type LibraryServer,
  getExpectedRows,
  startLibraryServer,
  writeLibraryMzml,
} from "./syntheticLibrary";

const bundlePath = path.join(__dirname, "../../dist/quantion.js");
const describeWithBundle = fs.existsSync(bundlePath) ? describe : describe.skip;

describeWithBundle("getScanMetadata on wasm", () => {
  let quantion: any;
  let library: Uint8Array;
  let server: LibraryServer;

  beforeAll(async () => {
    quantion = require(bundlePath);
    await quantion.init();
    const mzml = await quantion.parseMzml(writeLibraryMzml());
    library = await quantion.writeIon(mzml);
    mzml.dispose();
    server = await startLibraryServer(library);
  });

  afterAll(() => server.stop());

  it("reads scans by rt and mz together, in memory and over a URL", async () => {
    const query = { rt: { from: 1.5, to: 4.5 }, mz: 303 };
    for (const file of [await quantion.parseIon(library), await quantion.parseIon(server.url)]) {
      const scans = await quantion.getScans(file, query, { level: 2 });
      expect(scans.map((scan: { rt: number }) => scan.rt)).toEqual([4]);
      file.dispose();
    }
  });

  it("reads the metadata of a file in memory", async () => {
    const file = await quantion.parseIon(library);
    for (let position = 0; position < SCAN_COUNT; position += 1) {
      expect(await quantion.getScanMetadata(file, position)).toEqual(
        getExpectedRows(position),
      );
    }
    file.dispose();
  });

  it("keeps only the chosen tags", async () => {
    const file = await quantion.parseIon(library);
    const rows = await quantion.getScanMetadata(file, 1, {
      tags: ["selectedIon"],
    });
    expect(rows.map((row: { name: string }) => row.name)).toEqual([
      "selected ion m/z",
      "charge state",
    ]);
    file.dispose();
  });

  it("gives each scan its index", async () => {
    const file = await quantion.parseIon(library);
    const scans = await quantion.getScans(
      file,
      { rt: { from: 0, to: 100 } },
      { level: 2 },
    );
    expect(scans.map((scan: { index: number }) => scan.index)).toEqual([
      1, 3, 5,
    ]);
    file.dispose();
  });

  it("reads over a URL with only the planned bytes", async () => {
    for (const position of [0, 3, SCAN_COUNT - 1]) {
      const file = await quantion.parseIon(server.url);
      const opened = server.requests.length;
      expect(await quantion.getScanMetadata(file, position)).toEqual(
        getExpectedRows(position),
      );
      expect(server.requests.length).toBeGreaterThan(opened);
      file.dispose();
    }
  });

  it("refuses a missing index", async () => {
    const file = await quantion.parseIon(library);
    const error = await quantion
      .getScanMetadata(file)
      .catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(quantion.QuantionError);
    expect(error.kind).toBe("invalid_argument");
    expect(error.argument).toBe("index");
    file.dispose();
  });

  it("refuses an index out of range over a URL before any read", async () => {
    const file = await quantion.parseIon(server.url);
    const opened = server.requests.length;
    const error = await quantion
      .getScanMetadata(file, SCAN_COUNT)
      .catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(quantion.QuantionError);
    expect(error.argument).toBe("index");
    expect(server.requests.length).toBe(opened);
    file.dispose();
  });
});
