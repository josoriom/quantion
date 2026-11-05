import type {
  DataXY,
  Feature,
  FindPeaksOptions,
  IonImage,
  NoiseLevel,
  Peak,
  PeakParameters,
  Scan,
  ScanParam,
  Target,
} from "../types/types";
import type { ModelEntry } from "../types/internal";
import type { Complete } from "./options";

export type FileHandle = unknown;
export type ModelHandle = unknown;
export type WarpModelHandle = unknown;
export type MaybePromise<T> = T | Promise<T>;

export interface ByteRangeResult {
  offset: bigint;
  length: bigint;
}

export interface EicSettings {
  from: number;
  to: number;
  ppmTolerance: number;
  mzTolerance: number;
  timeUnit: number;
}

export interface GridSettings {
  start: number;
  end: number;
  stepSize: number;
}

export interface GroupingSettings {
  ppmTolerance: number;
  mzTolerance: number;
  rtTolerance: number;
  minFrequency: number;
}

export interface TargetColumns {
  rts: Float64Array;
  mzs: Float64Array;
  ranges: Float64Array;
  ids: string[];
}

export interface TransitionColumns {
  precursors: Float64Array;
  products: Float64Array;
  rts: Float64Array;
  ranges: Float64Array;
  ids: string[];
}

export interface PeakColumns {
  heights: Float64Array;
  centers: Float64Array;
  fwhms: Float64Array;
  tails: Float64Array;
}

export type PeakSettings = Complete<FindPeaksOptions>;

export interface Backend {
  readonly ready: boolean;
  readonly handlesAreGcFinalized: boolean;
  readonly canReadPaths: boolean;

  init(): Promise<void>;

  parseMzmlBytes(data: Uint8Array): FileHandle;
  parseMzmlPath(path: string): FileHandle;
  parseIonPath(path: string, cacheSize: number): MaybePromise<FileHandle>;
  parseIonRemote(url: URL, cacheSize: number): Promise<FileHandle>;
  parseIonBuffer(
    bytes: Uint8Array,
    cacheSize: number,
  ): MaybePromise<FileHandle>;
  freeFile(handle: FileHandle): void;

  writeIon(
    handle: FileHandle,
    out: string | null,
    overwrite: boolean,
    level: number,
    f32Compress: boolean,
    blockSize: number,
    mzWindow: number,
    headers: Uint8Array,
  ): MaybePromise<Uint8Array | undefined>;
  writeMzml(
    handle: FileHandle,
    out: string | null,
    overwrite: boolean,
  ): Uint8Array | undefined;

  loadModelPath(path: string): ModelHandle;
  loadModelBytes(bytes: Uint8Array): ModelHandle;
  freeModel(model: ModelHandle): void;
  modelMetadata(model: ModelHandle): ModelEntry[];
  loadWarp(model: ModelHandle): WarpModelHandle;
  freeWarpModel(warp: WarpModelHandle): void;
  prefetchRts(
    warp: WarpModelHandle,
    handle: FileHandle,
    mzs: Float64Array,
    rts: Float64Array,
  ): Promise<void>;
  predictRts(
    warp: WarpModelHandle,
    handle: FileHandle,
    mzs: Float64Array,
    rts: Float64Array,
  ): Float64Array;
  calibrateRts(
    warp: WarpModelHandle,
    handles: FileHandle[],
    listLengths: number[],
    ids: string[],
    mzs: Float64Array,
    rts: Float64Array,
  ): Target[];

  calculateEic(
    handle: FileHandle,
    mz: number,
    eic: EicSettings,
  ): MaybePromise<DataXY>;

  getScans(
    handle: FileHandle,
    queryType: number,
    from: number,
    to: number,
    level: number,
    timeUnit: number,
  ): MaybePromise<Scan[]>;

  getScansWithArrays(
    handle: FileHandle,
    queryType: number,
    from: number,
    to: number,
    level: number,
    timeUnit: number,
    accessions: Uint32Array,
    keys: string[],
  ): MaybePromise<Scan[]>;

  getScanMetadata(
    handle: FileHandle,
    index: number,
    tags: string[],
  ): MaybePromise<ScanParam[]>;

  getIonImage(
    handle: FileHandle,
    mz: number,
    tolerance: number,
    level: number,
    onProgress: (fetched: number, total: number, heldBytes: number) => void,
  ): MaybePromise<IonImage>;

  findPeaks(x: Float64Array, y: Float64Array, options: PeakSettings): Peak[];
  getPeak(
    x: Float64Array,
    y: Float64Array,
    rt: number,
    range: number,
    options: PeakSettings,
  ): Peak;
  fitPeak(
    x: Float64Array,
    y: Float64Array,
    rt: number,
    shape: number,
    intensity: number,
  ): PeakParameters | null;
  drawPeaks(
    peaks: PeakColumns,
    from: number,
    to: number,
    points: number,
  ): DataXY;
  findNoiseLevel(y: Float32Array): NoiseLevel;
  calculateBaseline(
    y: Float64Array,
    lambda: number,
    maxIterations: number,
  ): Float64Array;

  deisotopeFeatures(
    values: Float64Array,
    count: number,
    rtTolerance: number,
    mzTolerance: number,
  ): Feature[];

  findTargets(
    handle: FileHandle,
    targets: TargetColumns,
    eic: EicSettings,
    options: PeakSettings,
    threads: number,
  ): MaybePromise<Peak[]>;

  findTransitions(
    handle: FileHandle,
    targets: TransitionColumns,
    options: PeakSettings,
    threads: number,
  ): Peak[];

  findFeatures?(
    handle: FileHandle,
    eic: EicSettings,
    grid: GridSettings,
    options: PeakSettings,
    threads: number,
  ): MaybePromise<Feature[]>;

  matchFeatures?(
    directory: string,
    eic: EicSettings,
    grid: GridSettings,
    grouping: GroupingSettings,
    options: PeakSettings,
    threads: number,
  ): Feature[];
}
