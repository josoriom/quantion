import type { Scan } from "../types/types";
import {
  PAYLOAD_SCANS,
  SECTION_BASE_PEAK_INT,
  SECTION_BASE_PEAK_MZ,
  SECTION_INTENSITY,
  SECTION_MS_LEVEL,
  SECTION_MZ,
  SECTION_POINT_STARTS,
  SECTION_POLARITY,
  SECTION_POSITION_X,
  SECTION_POSITION_Y,
  SECTION_POSITION_Z,
  SECTION_RT,
  SECTION_SCAN_ARRAY_STARTS,
  SECTION_SCAN_ARRAY_VALUES,
  SECTION_SCAN_INDEX,
  SECTION_SELECTED_ION_MZ,
  SECTION_TOTAL_ION_CURRENT,
  expectKind,
  readCountSection,
  readF64Section,
  readBridge,
} from "./bridge";

export function readScans(buffer: ArrayBuffer): Scan[] {
  const bridge = readBridge(buffer);
  expectKind(bridge, PAYLOAD_SCANS);

  const pointStarts = readCountSection(bridge, SECTION_POINT_STARTS);
  const mz = readF64Section(bridge, SECTION_MZ);
  const intensity = readF64Section(bridge, SECTION_INTENSITY);
  const scanIndex = readF64Section(bridge, SECTION_SCAN_INDEX);
  const rt = readF64Section(bridge, SECTION_RT);
  const level = readF64Section(bridge, SECTION_MS_LEVEL);
  const precursorMz = readF64Section(bridge, SECTION_SELECTED_ION_MZ);
  const basePeakMz = readF64Section(bridge, SECTION_BASE_PEAK_MZ);
  const basePeakIntensity = readF64Section(bridge, SECTION_BASE_PEAK_INT);
  const totalIonCurrent = readF64Section(bridge, SECTION_TOTAL_ION_CURRENT);
  const polarity = readF64Section(bridge, SECTION_POLARITY);
  const x = readF64Section(bridge, SECTION_POSITION_X);
  const y = readF64Section(bridge, SECTION_POSITION_Y);
  const z = readF64Section(bridge, SECTION_POSITION_Z);

  const count = bridge.recordCount;
  if (pointStarts.length !== count + 1) {
    throw new Error(
      "quantion bridge: point starts do not match the scan count",
    );
  }
  if (intensity.length !== mz.length) {
    throw new Error(
      "quantion bridge: intensity length does not match m/z length",
    );
  }
  if (pointStarts[count] !== mz.length) {
    throw new Error(
      "quantion bridge: point starts do not span the m/z section",
    );
  }

  const scans: Scan[] = new Array(count);
  for (let index = 0; index < count; index += 1) {
    const from = pointStarts[index];
    const to = pointStarts[index + 1];
    scans[index] = {
      index: scanIndex[index],
      rt: rt[index],
      mz: mz.subarray(from, to),
      intensity: intensity.subarray(from, to),
      level: level[index],
      precursorMz: precursorMz[index],
      basePeakMz: basePeakMz[index],
      basePeakIntensity: basePeakIntensity[index],
      totalIonCurrent: totalIonCurrent[index],
      polarity: polarity[index],
      x: x[index],
      y: y[index],
      z: z[index],
    };
  }
  return scans;
}

export function readScanArrays(buffer: ArrayBuffer, keys: string[]): Scan[] {
  const scans = readScans(buffer);
  const bridge = readBridge(buffer);
  const starts = readCountSection(bridge, SECTION_SCAN_ARRAY_STARTS);
  const values = readF64Section(bridge, SECTION_SCAN_ARRAY_VALUES);
  if (starts.length !== scans.length * keys.length + 1) {
    throw new Error(
      "quantion bridge: array starts do not match the scans and arrays",
    );
  }
  if (starts[starts.length - 1] !== values.length) {
    throw new Error(
      "quantion bridge: array starts do not span the array values",
    );
  }
  let position = 0;
  for (const scan of scans) {
    for (const key of keys) {
      scan[key] = values.subarray(starts[position], starts[position + 1]);
      position += 1;
    }
  }
  return scans;
}
