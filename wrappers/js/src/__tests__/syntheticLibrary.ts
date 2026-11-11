import * as http from "http";
import type { AddressInfo } from "net";

export const SCAN_COUNT = 6;

function getLevel(position: number): number {
  return position % 2 === 0 ? 1 : 2;
}

const MZ_UNIT = 'unitCvRef="MS" unitAccession="MS:1000040" unitName="m/z"';

function writeArray(accession: string, name: string): string {
  const encoded = Buffer.from(Float64Array.from([100, 200]).buffer).toString(
    "base64",
  );
  return `<binaryDataArray encodedLength="${encoded.length}"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/><cvParam cvRef="MS" accession="MS:1000576" name="no compression"/><cvParam cvRef="MS" accession="${accession}" name="${name}"/><binary>${encoded}</binary></binaryDataArray>`;
}

function writePrecursor(position: number): string {
  if (getLevel(position) === 1) return "";
  const mz = 300 + position;
  return `<precursorList count="1"><precursor spectrumRef="scan=${position}"><isolationWindow><cvParam cvRef="MS" accession="MS:1000827" name="isolation window target m/z" value="${mz}" ${MZ_UNIT}/></isolationWindow><selectedIonList count="2"><selectedIon><cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="${mz}" ${MZ_UNIT}/></selectedIon><selectedIon><cvParam cvRef="MS" accession="MS:1000041" name="charge state" value="2"/></selectedIon></selectedIonList><activation><cvParam cvRef="MS" accession="MS:1000133" name="collision-induced dissociation"/></activation></precursor></precursorList>`;
}

export function getLibraryId(position: number): string {
  return `SYNTHETIC${String(position).padStart(3, "0")}`;
}

function writeSpectrum(position: number): string {
  const level = getLevel(position);
  const start = position + 1;
  return `<spectrum index="${position}" id="scan=${start}" defaultArrayLength="2" dataProcessingRef="synthetic_processing"><referenceableParamGroupRef ref="common_params"/><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="${level}"/><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/><cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/><userParam name="library_id" value="${getLibraryId(position)}"/><userParam name="formula" value="C${start}H${level}"/><userParam name="library_id" value="REPEATED"/><scanList count="1"><cvParam cvRef="MS" accession="MS:1000795" name="no combination"/><scan instrumentConfigurationRef="synthetic_instrument"><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="${start}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/><userParam name="scan_note" value="synthetic note"/><scanWindowList count="1"><scanWindow><cvParam cvRef="MS" accession="MS:1000501" name="scan window lower limit" value="50" ${MZ_UNIT}/></scanWindow></scanWindowList></scan></scanList>${writePrecursor(position)}<binaryDataArrayList count="2">${writeArray("MS:1000514", "m/z array")}${writeArray("MS:1000515", "intensity array")}</binaryDataArrayList></spectrum>`;
}

export function writeLibraryMzml(): Uint8Array {
  const spectra = Array.from({ length: SCAN_COUNT }, (_, position) =>
    writeSpectrum(position),
  ).join("");
  return new TextEncoder().encode(
    `<?xml version="1.0"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><cvList count="1"><cv id="MS" fullName="x" URI="x"/></cvList><referenceableParamGroupList count="1"><referenceableParamGroup id="common_params"><userParam name="group_note" value="shared"/></referenceableParamGroup></referenceableParamGroupList><run id="r"><spectrumList count="${SCAN_COUNT}" defaultDataProcessingRef="d">${spectra}</spectrumList></run></mzML>`,
  );
}

export type ExpectedRow = {
  path: string;
  kind: string;
  accession: string | null;
  name: string;
  value: string | null;
  unit: string | null;
  unitAccession: string | null;
};

function row(
  path: string,
  kind: string,
  accession: string | null,
  name: string,
  value: string | null,
  unit: [string, string] | null = null,
): ExpectedRow {
  return {
    path,
    kind,
    accession,
    name,
    value,
    unit: unit ? unit[0] : null,
    unitAccession: unit ? unit[1] : null,
  };
}

export function getExpectedRows(position: number): ExpectedRow[] {
  const level = String(getLevel(position));
  const start = String(position + 1);
  const mz = String(300 + position);
  const scan = "spectrum/scanList/scan[0]";
  const mzUnit: [string, string] = ["m/z", "MS:1000040"];
  const rows = [
    row("spectrum", "ATTRIBUTE", null, "index", String(position)),
    row("spectrum", "ATTRIBUTE", null, "id", `scan=${start}`),
    row("spectrum", "ATTRIBUTE", null, "defaultArrayLength", "2"),
    row(
      "spectrum",
      "ATTRIBUTE",
      null,
      "dataProcessingRef",
      "synthetic_processing",
    ),
    row(
      "spectrum/referenceableParamGroupRef[0]",
      "ATTRIBUTE",
      null,
      "ref",
      "common_params",
    ),
    row("spectrum", "CV", "MS:1000511", "ms level", level),
    row("spectrum", "CV", "MS:1000127", "centroid spectrum", null),
    row("spectrum", "CV", "MS:1000130", "positive scan", null),
    row("spectrum", "USER", null, "library_id", getLibraryId(position)),
    row("spectrum", "USER", null, "formula", `C${start}H${level}`),
    row("spectrum", "USER", null, "library_id", "REPEATED"),
    row("spectrum/scanList", "CV", "MS:1000795", "no combination", null),
    row(
      scan,
      "ATTRIBUTE",
      null,
      "instrumentConfigurationRef",
      "synthetic_instrument",
    ),
    row(scan, "CV", "MS:1000016", "scan start time", start, [
      "minute",
      "UO:0000031",
    ]),
    row(scan, "USER", null, "scan_note", "synthetic note"),
    row(
      `${scan}/scanWindowList/scanWindow[0]`,
      "CV",
      "MS:1000501",
      "scan window lower limit",
      "50",
      mzUnit,
    ),
  ];
  if (level === "2") {
    const precursor = "spectrum/precursorList/precursor[0]";
    rows.push(
      row(precursor, "ATTRIBUTE", null, "spectrumRef", `scan=${position}`),
      row(
        `${precursor}/isolationWindow`,
        "CV",
        "MS:1000827",
        "isolation window target m/z",
        mz,
        mzUnit,
      ),
      row(
        `${precursor}/selectedIonList/selectedIon[0]`,
        "CV",
        "MS:1000744",
        "selected ion m/z",
        mz,
        mzUnit,
      ),
      row(
        `${precursor}/selectedIonList/selectedIon[1]`,
        "CV",
        "MS:1000041",
        "charge state",
        "2",
      ),
      row(
        `${precursor}/activation`,
        "CV",
        "MS:1000133",
        "collision-induced dissociation",
        null,
      ),
    );
  }
  const arrays: Array<[string, string]> = [
    ["MS:1000514", "m/z array"],
    ["MS:1000515", "intensity array"],
  ];
  arrays.forEach(([accession, name], number) => {
    const array = `spectrum/binaryDataArrayList/binaryDataArray[${number}]`;
    rows.push(
      row(array, "ATTRIBUTE", null, "encodedLength", "24"),
      row(array, "CV", "MS:1000523", "64-bit float", null),
      row(array, "CV", "MS:1000576", "no compression", null),
      row(array, "CV", accession, name, null),
    );
  });
  return rows;
}

export function getRowLibraryId(rows: ExpectedRow[]): string | null {
  return rows.find((found) => found.name === "library_id")?.value ?? null;
}

export type ByteRange = { from: number; to: number };

export type LibraryServer = {
  url: string;
  requests: ByteRange[];
  stop: () => Promise<void>;
};

export async function startLibraryServer(
  library: Uint8Array,
): Promise<LibraryServer> {
  const requests: ByteRange[] = [];
  const server = http.createServer((request, response) => {
    const wanted = /bytes=(\d+)-(\d+)/.exec(request.headers.range ?? "");
    if (!wanted) {
      response.writeHead(200, { "content-length": library.length });
      response.end(request.method === "HEAD" ? undefined : library);
      return;
    }
    const from = Number(wanted[1]);
    const to = Math.min(Number(wanted[2]), library.length - 1);
    requests.push({ from, to });
    response.writeHead(206, {
      "content-length": to - from + 1,
      "content-range": `bytes ${from}-${to}/${library.length}`,
    });
    response.end(library.subarray(from, to + 1));
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const port = (server.address() as AddressInfo).port;
  return {
    url: `http://127.0.0.1:${port}/library.ion`,
    requests,
    stop: () => new Promise<void>((resolve) => server.close(() => resolve())),
  };
}
