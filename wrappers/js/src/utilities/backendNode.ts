/// <reference types="node" />

import * as fs from "fs";
import * as path from "path";
import type {
  Backend,
  EicSettings,
  FileHandle,
  GridSettings,
  GroupingSettings,
  ModelHandle,
  PeakColumns,
  PeakSettings,
  TargetColumns,
  TransitionColumns,
  WarpModelHandle,
} from "./backend";
import type {
  DataXY,
  Feature,
  IonImage,
  NoiseLevel,
  Peak,
  PeakParameters,
  Scan,
  ScanParam,
  Target,
} from "../types/types";
import type { ModelEntry } from "../types/internal";
import { TAG_SEPARATOR } from "./options";
import { toQuantionError } from "./errors";
import { readRecords, readScanMetadata } from "./recordBridge";
import { readIonImage } from "./imageBridge";
import { readScanArrays, readScans } from "./scanBridge";
import {
  ByteRangeResult,
  RemoteSource,
  fetchHeader,
  newRemoteSource,
  prefetchRanges,
} from "./remoteSource";

function libraryFileName(platform: string): string {
  if (platform === "win32") return "libquantion.dll";
  if (platform === "darwin") return "libquantion.dylib";
  return "libquantion.so";
}

function platformDirName(platform: string, arch: string): string {
  const arm = arch === "arm64";
  if (platform === "darwin") return arm ? "macos-arm64" : "macos-x86_64";
  if (platform === "linux") return arm ? "linux-arm64" : "linux-x86_64";
  if (platform === "win32") return arm ? "windows-arm64" : "windows-x86_64";
  throw new Error(`quantion: unsupported ${platform}/${arch}`);
}

function isPackageRoot(directory: string): boolean {
  const manifest = path.join(directory, "package.json");
  if (!fs.existsSync(manifest)) return false;
  try {
    return JSON.parse(fs.readFileSync(manifest, "utf8")).name === "quantion";
  } catch {
    return false;
  }
}

function artifactsRoot(): string | null {
  const fromEnv = process.env.QUANTION_ARTIFACTS_ROOT;
  if (fromEnv) return fs.existsSync(fromEnv) ? fromEnv : null;
  let here = __dirname;
  for (;;) {
    const candidate = path.join(here, "artifacts");
    if (fs.existsSync(candidate)) return candidate;
    if (isPackageRoot(here)) return null;
    const parent = path.dirname(here);
    if (parent === here) return null;
    here = parent;
  }
}

function findDevelopmentLibrary(proc: NodeJS.Process): string | null {
  const fromEnv = proc.env.QUANTION_LIB;
  if (fromEnv) return fromEnv;

  const root = artifactsRoot();
  if (!root) return null;
  let wanted: string;
  try {
    wanted = platformDirName(proc.platform, proc.arch);
  } catch {
    return null;
  }
  const file = libraryFileName(proc.platform);

  const direct = path.join(root, wanted, file);
  if (fs.existsSync(direct)) return direct;

  const own = require("../../package.json").version as string;
  const preferred = path.join(root, own, wanted, file);
  if (fs.existsSync(preferred)) return preferred;

  const parts = (name: string) => name.split(".").map((p) => Number(p) || 0);
  const newest = fs
    .readdirSync(root)
    .filter((entry) => fs.existsSync(path.join(root, entry, wanted, file)))
    .filter((entry) => fs.existsSync(path.join(root, entry, "manifest.json")))
    .sort((left, right) => {
      const a = parts(left);
      const b = parts(right);
      for (let i = 0; i < Math.max(a.length, b.length); i++) {
        if ((a[i] ?? 0) !== (b[i] ?? 0)) return (b[i] ?? 0) - (a[i] ?? 0);
      }
      return 0;
    })[0];
  if (newest) return path.join(root, newest, wanted, file);
  return null;
}

const DEVELOPMENT_ADDON = path.join(
  __dirname,
  "..",
  "..",
  "build",
  "Release",
  "quantion.node",
);

const BUNDLED_FOLDER = path.join(__dirname, "..", "..", "native");

export function isMusl(proc: NodeJS.Process): boolean {
  if (proc.platform !== "linux") return false;
  const report = proc.report?.getReport() as
    { header?: { glibcVersionRuntime?: string } } | undefined;
  return !report?.header?.glibcVersionRuntime;
}

export function findBundledFiles(
  proc: NodeJS.Process,
  folder: string = BUNDLED_FOLDER,
  exists: (file: string) => boolean = fs.existsSync,
): { addon: string; library: string } {
  if (isMusl(proc)) {
    throw new Error(
      "quantion: there is no native build for Linux with musl (for example Alpine). " +
        "Use the WASM build, or set QUANTION_LIB to a libquantion built for this system.",
    );
  }
  let platform: string;
  try {
    platform = platformDirName(proc.platform, proc.arch);
  } catch {
    platform = "";
  }
  const addon = path.join(folder, platform, "quantion.node");
  const library = path.join(folder, platform, libraryFileName(proc.platform));
  if (!platform || !exists(addon) || !exists(library)) {
    throw new Error(
      `quantion: there is no native build for ${proc.platform}-${proc.arch}. ` +
        "Use the WASM build, or set QUANTION_LIB to a libquantion built for this system.",
    );
  }
  return { addon, library };
}

function findNativeFiles(proc: NodeJS.Process): {
  addon: string;
  library: string;
} {
  const developmentLibrary = findDevelopmentLibrary(proc);
  if (developmentLibrary && fs.existsSync(DEVELOPMENT_ADDON)) {
    return { addon: DEVELOPMENT_ADDON, library: developmentLibrary };
  }
  const artifactsFolder = proc.env.QUANTION_ARTIFACTS_ROOT;
  if (artifactsFolder && !developmentLibrary) {
    throw new Error(
      `quantion: QUANTION_ARTIFACTS_ROOT is ${artifactsFolder}, but it has no library for ${proc.platform}-${proc.arch}. ` +
        "Build one with 'make native', or unset QUANTION_ARTIFACTS_ROOT to use the bundled build.",
    );
  }
  const bundled = findBundledFiles(proc);
  return {
    addon: bundled.addon,
    library: developmentLibrary ?? bundled.library,
  };
}

function toNodeBuffer(data: Uint8Array): Buffer {
  if (Buffer.isBuffer(data)) return data;
  return Buffer.from(data.buffer, data.byteOffset, data.byteLength);
}

function catchNativeErrors(native: Record<string, unknown>): any {
  const wrapped: Record<string, unknown> = {};
  for (const [name, value] of Object.entries(native)) {
    if (typeof value !== "function") {
      wrapped[name] = value;
      continue;
    }
    wrapped[name] = (...values: unknown[]) => {
      try {
        return (value as (...given: unknown[]) => unknown)(...values);
      } catch (error) {
        throw toQuantionError(error);
      }
    };
  }
  return wrapped;
}

export class NodeBackend implements Backend {
  private native: any;
  private remoteByHandle = new WeakMap<object, RemoteSource>();
  ready = false;
  readonly handlesAreGcFinalized = true;
  readonly canReadPaths = true;

  constructor(proc: NodeJS.Process) {
    const files = findNativeFiles(proc);
    const native = require(files.addon);
    if (typeof native.bind !== "function") {
      throw new Error("quantion: the native addon does not expose bind");
    }
    native.bind(files.library);
    this.native = catchNativeErrors(native);
    this.ready = true;
  }

  async init(): Promise<void> {}

  private async prefetch(
    handle: FileHandle,
    plan: () => ByteRangeResult[],
  ): Promise<void> {
    const source = this.remoteByHandle.get(handle as object);
    if (!source) return;
    await prefetchRanges(source, plan());
  }

  parseMzmlBytes(data: Uint8Array): FileHandle {
    return this.native.parseMzml(toNodeBuffer(data));
  }

  parseMzmlPath(filePath: string): FileHandle {
    return this.native.parseMzmlPath(filePath);
  }

  parseIonPath(filePath: string, cacheSize: number): FileHandle {
    return this.native.parseIonPath(filePath, cacheSize);
  }

  async parseIonRemote(url: URL, cacheSize: number): Promise<FileHandle> {
    const source = newRemoteSource(url.href);
    const header = await fetchHeader(source);
    await prefetchRanges(
      source,
      this.native.planOpen(toNodeBuffer(header)) as ByteRangeResult[],
    );
    const serve = (offset: number, length: number): Buffer | null => {
      const bytes = source.cache.read(BigInt(offset), BigInt(length));
      return bytes ? toNodeBuffer(bytes) : null;
    };
    const handle = this.native.parseIonSource(serve, cacheSize) as FileHandle;
    this.remoteByHandle.set(handle as object, source);
    return handle;
  }

  parseIonBuffer(bytes: Uint8Array, cacheSize: number): FileHandle {
    return this.native.parseBin(toNodeBuffer(bytes), cacheSize);
  }

  freeFile(handle: FileHandle): void {
    this.remoteByHandle.delete(handle as object);
    this.native.dispose(handle);
  }

  writeIon(
    handle: FileHandle,
    out: string | null,
    overwrite: boolean,
    level: number,
    f32Compress: boolean,
    blockSize: number,
    mzWindow: number,
    headers: Uint8Array,
  ): Uint8Array | undefined {
    return this.native.writeIon(
      handle,
      out,
      overwrite ? 1 : 0,
      level,
      f32Compress ? 1 : 0,
      blockSize,
      mzWindow,
      toNodeBuffer(headers),
    );
  }

  writeMzml(
    handle: FileHandle,
    out: string | null,
    overwrite: boolean,
  ): Uint8Array | undefined {
    return this.native.writeMzml(handle, out, overwrite ? 1 : 0);
  }

  loadModelPath(filePath: string): ModelHandle {
    return this.native.loadModelPath(filePath);
  }

  loadModelBytes(bytes: Uint8Array): ModelHandle {
    return this.native.loadModelBytes(toNodeBuffer(bytes));
  }

  freeModel(model: ModelHandle): void {
    this.native.disposeModel(model);
  }

  modelMetadata(model: ModelHandle): ModelEntry[] {
    return readRecords(
      this.native.modelMetadata(model),
    ) as unknown as ModelEntry[];
  }

  loadWarp(model: ModelHandle): WarpModelHandle {
    return this.native.loadWarp(model);
  }

  freeWarpModel(warp: WarpModelHandle): void {
    this.native.disposeWarpModel(warp);
  }

  async prefetchRts(
    warp: WarpModelHandle,
    handle: FileHandle,
    mzs: Float64Array,
    rts: Float64Array,
  ): Promise<void> {
    await this.prefetch(handle, () =>
      this.native.planPredictRts(warp, handle, mzs, rts),
    );
  }

  predictRts(
    warp: WarpModelHandle,
    handle: FileHandle,
    mzs: Float64Array,
    rts: Float64Array,
  ): Float64Array {
    return this.native.predictRts(warp, handle, mzs, rts) as Float64Array;
  }

  calibrateRts(
    warp: WarpModelHandle,
    handles: FileHandle[],
    listLengths: number[],
    ids: string[],
    mzs: Float64Array,
    rts: Float64Array,
  ): Target[] {
    return readRecords(
      this.native.calibrateRts(warp, handles, listLengths, ids, mzs, rts),
    ) as unknown as Target[];
  }

  async calculateEic(
    handle: FileHandle,
    mz: number,
    eic: EicSettings,
  ): Promise<DataXY> {
    const values = [
      mz,
      eic.from,
      eic.to,
      eic.ppmTolerance,
      eic.mzTolerance,
      eic.timeUnit,
    ];
    await this.prefetch(handle, () => this.native.planEic(handle, ...values));
    return this.native.calculateEic(handle, ...values) as DataXY;
  }

  async getScans(
    handle: FileHandle,
    queryType: number,
    from: number,
    to: number,
    level: number,
    timeUnit: number,
  ): Promise<Scan[]> {
    const values = [queryType, from, to, level, timeUnit];
    await this.prefetch(handle, () => this.native.planScans(handle, ...values));
    return readScans(this.native.getScans(handle, ...values));
  }

  async getScansWithArrays(
    handle: FileHandle,
    queryType: number,
    from: number,
    to: number,
    level: number,
    timeUnit: number,
    accessions: Uint32Array,
    keys: string[],
  ): Promise<Scan[]> {
    const values = [queryType, from, to, level, timeUnit, accessions];
    await this.prefetch(handle, () =>
      this.native.planScansWithArrays(handle, ...values),
    );
    return readScanArrays(
      this.native.getScansWithArrays(handle, ...values),
      keys,
    );
  }

  async getScanMetadata(
    handle: FileHandle,
    index: number,
    tags: string[],
  ): Promise<ScanParam[]> {
    await this.prefetch(handle, () =>
      this.native.planScanMetadata(handle, index),
    );
    return readScanMetadata(
      this.native.getScanMetadata(handle, index, tags.join(TAG_SEPARATOR)),
    );
  }

  async getIonImage(
    handle: FileHandle,
    mz: number,
    tolerance: number,
    level: number,
  ): Promise<IonImage> {
    await this.prefetch(handle, () =>
      this.native.planImage(handle, mz, tolerance, level),
    );
    return readIonImage(this.native.getIonImage(handle, mz, tolerance, level));
  }

  findPeaks(x: Float64Array, y: Float64Array, options: PeakSettings): Peak[] {
    return readRecords(
      this.native.findPeaks(x, y, options),
    ) as unknown as Peak[];
  }

  getPeak(
    x: Float64Array,
    y: Float64Array,
    rt: number,
    range: number,
    options: PeakSettings,
  ): Peak {
    return readRecords(
      this.native.getPeak(x, y, rt, range, options),
    )[0] as unknown as Peak;
  }

  fitPeak(
    x: Float64Array,
    y: Float64Array,
    rt: number,
    shape: number,
    intensity: number,
  ): PeakParameters | null {
    const rows = readRecords(this.native.fitPeak(x, y, rt, shape, intensity));
    return rows.length ? (rows[0] as unknown as PeakParameters) : null;
  }

  drawPeaks(
    peaks: PeakColumns,
    from: number,
    to: number,
    points: number,
  ): DataXY {
    return this.native.drawPeaks(
      peaks.heights,
      peaks.centers,
      peaks.fwhms,
      peaks.tails,
      from,
      to,
      points,
    ) as DataXY;
  }

  findNoiseLevel(y: Float32Array): NoiseLevel {
    return this.native.findNoiseLevel(y) as NoiseLevel;
  }

  calculateBaseline(
    y: Float64Array,
    lambda: number,
    maxIterations: number,
  ): Float64Array {
    return this.native.calculateBaseline(y, lambda, maxIterations);
  }

  deisotopeFeatures(
    values: Float64Array,
    count: number,
    rtTolerance: number,
    mzTolerance: number,
  ): Feature[] {
    return readRecords(
      this.native.deisotopeFeatures(values, count, rtTolerance, mzTolerance),
    ) as unknown as Feature[];
  }

  async findTargets(
    handle: FileHandle,
    targets: TargetColumns,
    eic: EicSettings,
    options: PeakSettings,
    threads: number,
  ): Promise<Peak[]> {
    const window = [
      eic.from,
      eic.to,
      eic.ppmTolerance,
      eic.mzTolerance,
      eic.timeUnit,
    ];
    await this.prefetch(handle, () =>
      this.native.planFindTargets(handle, targets.mzs, ...window),
    );
    return readRecords(
      this.native.findTargets(
        handle,
        targets.rts,
        targets.mzs,
        targets.ranges,
        targets.ids,
        ...window,
        options,
        threads,
      ),
    ) as unknown as Peak[];
  }

  findTransitions(
    handle: FileHandle,
    targets: TransitionColumns,
    options: PeakSettings,
    threads: number,
  ): Peak[] {
    return readRecords(
      this.native.findTransitions(
        handle,
        targets.precursors,
        targets.products,
        targets.rts,
        targets.ranges,
        targets.ids,
        options,
        threads,
      ),
    ) as unknown as Peak[];
  }

  async findFeatures(
    handle: FileHandle,
    eic: EicSettings,
    grid: GridSettings,
    options: PeakSettings,
    threads: number,
  ): Promise<Feature[]> {
    const values = [
      eic.from,
      eic.to,
      eic.ppmTolerance,
      eic.mzTolerance,
      eic.timeUnit,
      grid.start,
      grid.end,
      grid.stepSize,
    ];
    await this.prefetch(handle, () =>
      this.native.planFindFeatures(handle, ...values),
    );
    return readRecords(
      this.native.findFeatures(handle, ...values, options, threads),
    ) as unknown as Feature[];
  }

  matchFeatures(
    directory: string,
    eic: EicSettings,
    grid: GridSettings,
    grouping: GroupingSettings,
    options: PeakSettings,
    threads: number,
  ): Feature[] {
    return readRecords(
      this.native.matchFeatures(
        directory,
        eic.from,
        eic.to,
        eic.ppmTolerance,
        eic.mzTolerance,
        eic.timeUnit,
        grid.start,
        grid.end,
        grid.stepSize,
        grouping.ppmTolerance,
        grouping.mzTolerance,
        grouping.rtTolerance,
        grouping.minFrequency,
        options,
        threads,
      ),
    ) as unknown as Feature[];
  }
}
