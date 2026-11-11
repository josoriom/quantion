import { QuantionError, invalidArgument, missingArgument } from "./errors";
import { PeakShape, TimeUnit } from "../types/types";
import type {
  AnnotateFeaturesOptions,
  CalculateBaselineOptions,
  DeisotopeFeaturesOptions,
  DrawPeaksOptions,
  CalculateEicOptions,
  FindPeaksOptions,
  FitPeakOptions,
  GetPeakOptions,
  GetScanMetadataOptions,
  GetScansOptions,
  ScanArray,
  GridOptions,
  GroupingOptions,
  Header,
  GetIonImageOptions,
  ParseIonOptions,
  WriteIonOptions,
  WriteMzmlOptions,
} from "../types/types";

export type Complete<T> = { [Key in keyof T]-?: T[Key] };

export interface TargetSettingsInput {
  eic: Complete<CalculateEicOptions>;
  findPeaks: Complete<FindPeaksOptions>;
  threads: number;
}

export interface TransitionSettingsInput {
  findPeaks: Complete<FindPeaksOptions>;
  threads: number;
}

export interface FeatureSettingsInput {
  eic: Complete<CalculateEicOptions>;
  grid: Complete<GridOptions>;
  findPeaks: Complete<FindPeaksOptions>;
  threads: number;
}

export interface MatchSettingsInput extends FeatureSettingsInput {
  grouping: Complete<GroupingOptions>;
}

export const FIND_PEAKS_DEFAULTS: Complete<FindPeaksOptions> = {
  minIntegral: NaN,
  minIntensity: 500,
  minPeakWidthPoints: 3,
  noise: NaN,
  autoNoise: true,
  autoBaseline: true,
  lambda: 20,
  maxIterations: 4,
  allowOverlap: false,
  minSnr: 2,
  minR2: 0,
  shape: PeakShape.EMG,
  kernelSize: 0,
};

export const EIC_DEFAULTS: Complete<CalculateEicOptions> = {
  from: 0,
  to: Infinity,
  ppmTolerance: 20,
  mzTolerance: 0.005,
  timeUnit: TimeUnit.MINUTES,
};

export const GRID_DEFAULTS: Complete<GridOptions> = {
  start: 40,
  end: 1000,
  stepSize: 0.005,
};

export const GROUPING_DEFAULTS: Complete<GroupingOptions> = {
  ppmTolerance: 5,
  mzTolerance: 0.0025,
  rtTolerance: 0.05,
  minFrequency: 0,
};

type Kind = "number" | "boolean" | "string";

function describe(value: unknown): string {
  if (value === null) return "null";
  if (Array.isArray(value)) return "an array";
  return typeof value;
}

function getObject(
  value: unknown,
  functionName: string,
  path: string,
): Record<string, unknown> {
  if (value === undefined) return {};
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw invalidArgument(
      functionName,
      path || "options",
      `must be an object, got ${describe(value)}`,
    );
  }
  return value as Record<string, unknown>;
}

export const TARGET_KEYS = ["mz", "rt", "id", "range"];
export const FEATURE_NUMBER_KEYS = [
  "mz",
  "rt",
  "from",
  "to",
  "intensity",
  "integral",
  "nPoints",
] as const;
export const TRANSITION_KEYS = ["precursor", "product", "rt", "range", "id"];
export const PEAK_KEYS = ["height", "center", "fwhm", "tail", "r2"];
export const RANGE_KEYS = ["from", "to"];

export function checkItems(
  items: object[],
  allowed: string[],
  functionName: string,
  argument: string,
): void {
  for (let index = 0; index < items.length; index++) {
    checkKeys(
      items[index] as Record<string, unknown>,
      allowed,
      functionName,
      `${argument}[${index}].`,
    );
  }
}

export function getRequiredNumber(
  value: unknown,
  functionName: string,
  argument: string,
): number {
  if (value === undefined || value === null) {
    throw missingArgument(functionName, argument);
  }
  if (typeof value !== "number") {
    throw invalidArgument(
      functionName,
      argument,
      `must be a number, got ${typeof value}`,
    );
  }
  return value;
}

export function checkKeys(
  given: Record<string, unknown>,
  allowed: readonly string[],
  functionName: string,
  path: string,
): void {
  for (const key of Object.keys(given)) {
    if (!allowed.includes(key)) {
      const argument = `${path}${key}`;
      throw new QuantionError(
        "unknown_option",
        functionName,
        `${argument} is not an option; the options are ${allowed.join(", ")}`,
        argument,
      );
    }
  }
}

function getValue<T>(
  given: Record<string, unknown>,
  key: string,
  kind: Kind,
  fallback: T,
  functionName: string,
  path: string,
): T {
  const value = given[key];
  if (value === undefined) return fallback;
  if (typeof value !== kind) {
    throw invalidArgument(
      functionName,
      `${path}${key}`,
      `must be a ${kind}, got ${describe(value)}`,
    );
  }
  return value as T;
}

function getChoice<T extends string>(
  given: Record<string, unknown>,
  key: string,
  choices: Record<string, T>,
  fallback: T,
  functionName: string,
  path: string,
): T {
  const value = given[key];
  if (value === undefined) return fallback;
  const allowed = Object.values(choices) as string[];
  if (typeof value !== "string" || !allowed.includes(value)) {
    throw invalidArgument(
      functionName,
      `${path}${key}`,
      `must be one of ${allowed.join(", ")}, got ${String(value)}`,
    );
  }
  return value as T;
}

function readNumbers<T extends object>(
  value: unknown,
  defaults: Complete<T>,
  functionName: string,
  path: string,
): Complete<T> {
  const given = getObject(value, functionName, path);
  const keys = Object.keys(defaults);
  checkKeys(given, keys, functionName, path);
  const result: Record<string, unknown> = {};
  for (const key of keys) {
    const fallback = (defaults as Record<string, unknown>)[key];
    result[key] = getValue(
      given,
      key,
      typeof fallback as Kind,
      fallback,
      functionName,
      path,
    );
  }
  return result as Complete<T>;
}

export function readFindPeaksOptions(
  value: unknown,
  functionName: string,
  path = "findPeaks.",
): Complete<FindPeaksOptions> {
  const given = getObject(value, functionName, path);
  const { shape, ...numbers } = FIND_PEAKS_DEFAULTS;
  const { shape: givenShape, ...givenNumbers } = given;
  checkKeys(given, Object.keys(FIND_PEAKS_DEFAULTS), functionName, path);
  const result = readNumbers(givenNumbers, numbers, functionName, path);
  return {
    ...result,
    shape: getChoice(
      { shape: givenShape },
      "shape",
      PeakShape,
      shape,
      functionName,
      path,
    ),
  };
}

export function readEicOptions(
  value: unknown,
  functionName: string,
  path = "eic.",
): Complete<CalculateEicOptions> {
  const given = getObject(value, functionName, path);
  const { timeUnit, ...numbers } = EIC_DEFAULTS;
  const { timeUnit: givenUnit, ...givenNumbers } = given;
  checkKeys(given, Object.keys(EIC_DEFAULTS), functionName, path);
  return {
    ...readNumbers(givenNumbers, numbers, functionName, path),
    timeUnit: getChoice(
      { timeUnit: givenUnit },
      "timeUnit",
      TimeUnit,
      timeUnit,
      functionName,
      path,
    ),
  };
}

export function readGridOptions(
  value: unknown,
  functionName: string,
): Complete<GridOptions> {
  return readNumbers(value, GRID_DEFAULTS, functionName, "grid.");
}

export function readGroupingOptions(
  value: unknown,
  functionName: string,
): Complete<GroupingOptions> {
  return readNumbers(value, GROUPING_DEFAULTS, functionName, "grouping.");
}

function readThreads(
  given: Record<string, unknown>,
  functionName: string,
): number {
  return getValue(given, "threads", "number", 1, functionName, "");
}

export function readGetPeakOptions(
  value: unknown,
  functionName: string,
): { range: number; findPeaks: Complete<FindPeaksOptions> } {
  const given = getObject(value, functionName, "");
  checkKeys(given, ["range", "findPeaks"], functionName, "");
  return {
    range: getValue(given, "range", "number", 0, functionName, ""),
    findPeaks: readFindPeaksOptions(given.findPeaks, functionName),
  };
}

export function readFitPeakOptions(
  value: unknown,
  functionName: string,
): Complete<FitPeakOptions> {
  const given = getObject(value, functionName, "");
  checkKeys(given, ["shape", "intensity"], functionName, "");
  return {
    shape: getChoice(
      given,
      "shape",
      PeakShape,
      PeakShape.EMG,
      functionName,
      "",
    ),
    intensity: getValue(given, "intensity", "number", 0, functionName, ""),
  };
}

export function readDrawPeaksOptions(
  value: unknown,
  functionName: string,
): Complete<DrawPeaksOptions> {
  return readNumbers(value, { from: 0, to: 0, points: 1000 }, functionName, "");
}

export function readBaselineOptions(
  value: unknown,
  functionName: string,
): Complete<CalculateBaselineOptions> {
  return readNumbers(value, { lambda: 20, maxIterations: 4 }, functionName, "");
}

export function readDeisotopeFeaturesOptions(
  value: unknown,
  functionName: string,
): Complete<DeisotopeFeaturesOptions> {
  return readNumbers(
    value,
    { rtTolerance: 0.02, mzTolerance: 0.005 },
    functionName,
    "",
  );
}

export function readFindTargetsOptions(
  value: unknown,
  functionName: string,
): TargetSettingsInput {
  const given = getObject(value, functionName, "");
  checkKeys(given, ["eic", "findPeaks", "threads"], functionName, "");
  return {
    eic: readEicOptions(given.eic, functionName),
    findPeaks: readFindPeaksOptions(given.findPeaks, functionName),
    threads: readThreads(given, functionName),
  };
}

export function readFindTransitionsOptions(
  value: unknown,
  functionName: string,
): TransitionSettingsInput {
  const given = getObject(value, functionName, "");
  checkKeys(given, ["findPeaks", "threads"], functionName, "");
  return {
    findPeaks: readFindPeaksOptions(given.findPeaks, functionName),
    threads: readThreads(given, functionName),
  };
}

export function readFindFeaturesOptions(
  value: unknown,
  functionName: string,
): FeatureSettingsInput {
  const given = getObject(value, functionName, "");
  checkKeys(given, ["eic", "grid", "findPeaks", "threads"], functionName, "");
  return {
    eic: readEicOptions(given.eic, functionName),
    grid: readGridOptions(given.grid, functionName),
    findPeaks: readFindPeaksOptions(given.findPeaks, functionName),
    threads: readThreads(given, functionName),
  };
}

export function readMatchFeaturesOptions(
  value: unknown,
  functionName: string,
): MatchSettingsInput {
  const given = getObject(value, functionName, "");
  checkKeys(
    given,
    ["eic", "grid", "grouping", "findPeaks", "threads"],
    functionName,
    "",
  );
  return {
    eic: readEicOptions(given.eic, functionName),
    grid: readGridOptions(given.grid, functionName),
    grouping: readGroupingOptions(given.grouping, functionName),
    findPeaks: readFindPeaksOptions(given.findPeaks, functionName),
    threads: readThreads(given, functionName),
  };
}

const DEFAULT_SCAN_ARRAYS: ScanArray[] = [
  { key: "mz", accession: "MS:1000514" },
  { key: "intensity", accession: "MS:1000515" },
];
const RESERVED_SCAN_KEYS = new Set([
  "index",
  "rt",
  "level",
  "precursor_mz",
  "precursorMz",
  "base_peak_mz",
  "basePeakMz",
  "base_peak_intensity",
  "basePeakIntensity",
  "total_ion_current",
  "totalIonCurrent",
  "polarity",
  "x",
  "y",
  "z",
]);
const SCAN_ARRAY_FIELDS = ["key", "accession"];
const ACCESSION_PATTERN = /^MS:[0-9]+$/;
const MAX_ACCESSION = 4294967295;

export function hasDefaultScanArrays(arrays: ScanArray[]): boolean {
  return (
    arrays.length === DEFAULT_SCAN_ARRAYS.length &&
    arrays.every(
      (array, index) =>
        array.key === DEFAULT_SCAN_ARRAYS[index].key &&
        array.accession === DEFAULT_SCAN_ARRAYS[index].accession,
    )
  );
}

export function getAccessionNumbers(arrays: ScanArray[]): Uint32Array {
  return Uint32Array.from(arrays, (array) => Number(array.accession.slice(3)));
}

function checkScanArray(
  value: unknown,
  seenKeys: Set<string>,
  functionName: string,
): ScanArray {
  if (value === null || typeof value !== "object") {
    throw invalidArgument(
      functionName,
      "arrays",
      "must hold objects with key and accession",
    );
  }
  const given = value as Record<string, unknown>;
  checkKeys(given, SCAN_ARRAY_FIELDS, functionName, "arrays.");
  const { key, accession } = given;
  if (typeof key !== "string" || key === "") {
    throw invalidArgument(functionName, "arrays.key", "must not be empty");
  }
  if (RESERVED_SCAN_KEYS.has(key)) {
    throw invalidArgument(
      functionName,
      "arrays.key",
      `cannot be ${JSON.stringify(key)}: a scan already has that field`,
    );
  }
  if (
    typeof accession !== "string" ||
    !ACCESSION_PATTERN.test(accession) ||
    Number(accession.slice(3)) > MAX_ACCESSION
  ) {
    throw invalidArgument(
      functionName,
      "arrays.accession",
      `must look like MS:1000514, got ${JSON.stringify(accession)}`,
    );
  }
  for (const pair of DEFAULT_SCAN_ARRAYS) {
    if ((key === pair.key) !== (accession === pair.accession)) {
      throw invalidArgument(
        functionName,
        "arrays",
        `${pair.key} and ${pair.accession} must go together`,
      );
    }
  }
  if (seenKeys.has(key)) {
    throw invalidArgument(
      functionName,
      "arrays.key",
      `repeats ${JSON.stringify(key)}`,
    );
  }
  seenKeys.add(key);
  return { key, accession };
}

function readScanArrays(value: unknown, functionName: string): ScanArray[] {
  if (value === undefined) return DEFAULT_SCAN_ARRAYS;
  if (!Array.isArray(value) || value.length === 0) {
    throw invalidArgument(
      functionName,
      "arrays",
      "must have at least one array",
    );
  }
  const seenKeys = new Set<string>();
  return value.map((array) => checkScanArray(array, seenKeys, functionName));
}

export function readGetScansOptions(
  value: unknown,
  functionName: string,
): Complete<GetScansOptions> {
  const given = getObject(value, functionName, "");
  checkKeys(given, ["level", "timeUnit", "arrays"], functionName, "");
  return {
    arrays: readScanArrays(given.arrays, functionName),
    level: getValue(given, "level", "number", 1, functionName, ""),
    timeUnit: getChoice(
      given,
      "timeUnit",
      TimeUnit,
      TimeUnit.MINUTES,
      functionName,
      "",
    ),
  };
}

export const TAG_SEPARATOR = "\x1f";

export const SCAN_METADATA_TAGS = [
  "spectrum",
  "referenceableParamGroupRef",
  "spectrumDescription",
  "scanList",
  "scan",
  "scanWindowList",
  "scanWindow",
  "precursorList",
  "precursor",
  "isolationWindow",
  "selectedIonList",
  "selectedIon",
  "activation",
  "productList",
  "product",
  "binaryDataArrayList",
  "binaryDataArray",
] as const;

export function readGetScanMetadataOptions(
  value: unknown,
  functionName: string,
): Complete<GetScanMetadataOptions> {
  const given = getObject(value, functionName, "");
  checkKeys(given, ["tags"], functionName, "");
  const tags = given.tags === undefined ? [] : given.tags;
  if (!Array.isArray(tags)) {
    throw invalidArgument(
      functionName,
      "tags",
      `must be an array of tag names, got ${describe(tags)}`,
    );
  }
  for (const tag of tags) {
    if (!(SCAN_METADATA_TAGS as readonly unknown[]).includes(tag)) {
      throw invalidArgument(
        functionName,
        "tags",
        `must be one of ${SCAN_METADATA_TAGS.join(", ")}, got ${describe(tag)}`,
      );
    }
  }
  return { tags: tags as string[] };
}

export function readParseIonOptions(
  value: unknown,
  functionName: string,
): Complete<ParseIonOptions> {
  const settings = readNumbers(value, { maxCacheSize: 0 }, functionName, "");
  const size = settings.maxCacheSize;
  if (!Number.isSafeInteger(size) || size < 0) {
    throw invalidArgument(
      functionName,
      "maxCacheSize",
      `must be a whole number, 0 or more, got ${size}`,
    );
  }
  return settings;
}

export interface WriteIonSettings {
  out: string | null;
  overwrite: boolean;
  level: number;
  f32Compress: boolean;
  blockSize: number;
  mzWindow: number;
  headers: Header[];
}

const LARGEST_U32 = 4294967295;

function getWholeNumber(
  given: Record<string, unknown>,
  key: string,
  fallback: number,
  smallest: number,
  functionName: string,
): number {
  const value = given[key] === undefined ? fallback : given[key];
  if (
    typeof value !== "number" ||
    !Number.isInteger(value) ||
    value < smallest ||
    value > LARGEST_U32
  ) {
    throw invalidArgument(
      functionName,
      key,
      `must be a whole number from ${smallest} to ${LARGEST_U32}, got ${describe(value)}`,
    );
  }
  return value;
}

function readSource(
  given: Record<string, unknown>,
  key: string,
  fallback: string,
  functionName: string,
): string {
  const source = getValue(given, key, "string", fallback, functionName, "");
  if (source.trim() === "") {
    throw invalidArgument(functionName, key, "must be a path or a URL, got an empty text");
  }
  return source;
}

function readOut(
  given: Record<string, unknown>,
  functionName: string,
): string | null {
  const out = given.out;
  if (out === undefined) return null;
  if (out instanceof URL) return out.href;
  if (typeof out !== "string") {
    throw invalidArgument(
      functionName,
      "out",
      `must be a path or a URL, got ${describe(out)}`,
    );
  }
  return out;
}

function readHeaders(value: unknown, functionName: string): Header[] {
  if (value === undefined) return [];
  if (!Array.isArray(value)) {
    throw invalidArgument(
      functionName,
      "headers",
      `must be an array of { name, value }, got ${describe(value)}`,
    );
  }
  return value.map((header, index) => {
    const path = `headers[${index}].`;
    const given = getObject(header, functionName, path);
    checkKeys(given, ["name", "value"], functionName, path);
    return {
      name: getValue(given, "name", "string", "", functionName, path),
      value: getValue(given, "value", "string", "", functionName, path),
    };
  });
}

export function readWriteIonOptions(
  value: unknown,
  functionName: string,
): WriteIonSettings {
  const given = getObject(value, functionName, "");
  checkKeys(
    given,
    [
      "out",
      "overwrite",
      "level",
      "f32Compress",
      "blockSize",
      "mzWindow",
      "headers",
    ],
    functionName,
    "",
  );
  return {
    out: readOut(given, functionName),
    overwrite: getValue(given, "overwrite", "boolean", false, functionName, ""),
    level: getValue(given, "level", "number", 12, functionName, ""),
    f32Compress: getValue(
      given,
      "f32Compress",
      "boolean",
      false,
      functionName,
      "",
    ),
    blockSize: getWholeNumber(given, "blockSize", 8, 1, functionName),
    mzWindow: getWholeNumber(given, "mzWindow", 20, 0, functionName),
    headers: readHeaders(given.headers, functionName),
  };
}

export function readWriteMzmlOptions(
  value: unknown,
  functionName: string,
): { out: string | null; overwrite: boolean } {
  const given = getObject(value, functionName, "");
  checkKeys(
    given,
    ["out", "overwrite"] as (keyof WriteMzmlOptions)[],
    functionName,
    "",
  );
  return {
    out: readOut(given, functionName),
    overwrite: getValue(given, "overwrite", "boolean", false, functionName, ""),
  };
}

export function readIonImageOptions(
  value: unknown,
  functionName: string,
): Complete<GetIonImageOptions> {
  const given = getObject(value, functionName, "");
  checkKeys(given, ["tolerance", "level", "onProgress"], functionName, "");
  const onProgress = given.onProgress;
  if (onProgress !== undefined && typeof onProgress !== "function") {
    throw invalidArgument(
      functionName,
      "onProgress",
      `must be a function, got ${describe(onProgress)}`,
    );
  }
  return {
    tolerance: getValue(given, "tolerance", "number", 0.125, functionName, ""),
    level: getValue(given, "level", "number", 1, functionName, ""),
    onProgress: (onProgress ??
      (() => {})) as Complete<GetIonImageOptions>["onProgress"],
  };
}

export function packHeaders(headers: Header[]): Uint8Array {
  return new TextEncoder().encode(
    headers.map((header) => `${header.name}\0${header.value}\0`).join(""),
  );
}

export function getTimeUnitCode(timeUnit: TimeUnit): number {
  return timeUnit === TimeUnit.SECONDS ? 1 : 0;
}

export function getShapeCode(shape: PeakShape): number {
  return shape === PeakShape.GAUSSIAN ? 0 : 1;
}

export const DEFAULT_LIBRARY =
  "https://github.com/josoriom/cipher/releases/latest/download/library_positive.ion";
export const DEFAULT_MODEL =
  "https://github.com/josoriom/cipher/releases/latest/download/model.onnx";

export function readAnnotateFeaturesOptions(
  value: unknown,
  functionName: string,
): Complete<AnnotateFeaturesOptions> {
  const given = getObject(value, functionName, "");
  checkKeys(
    given,
    ["library", "model", "matches", "ppmTolerance", "mzTolerance", "rtRange", "minRelativeIntensity"],
    functionName,
    "",
  );
  return {
    library: readSource(given, "library", DEFAULT_LIBRARY, functionName),
    model: readSource(given, "model", DEFAULT_MODEL, functionName),
    matches: getValue(given, "matches", "number", 3, functionName, ""),
    ppmTolerance: getValue(given, "ppmTolerance", "number", 20, functionName, ""),
    mzTolerance: getValue(given, "mzTolerance", "number", 0.005, functionName, ""),
    rtRange: getValue(given, "rtRange", "number", 0.3, functionName, ""),
    minRelativeIntensity: getValue(given, "minRelativeIntensity", "number", 0.02, functionName, ""),
  };
}

