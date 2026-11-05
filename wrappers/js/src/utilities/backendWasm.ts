/// <reference types="node" />

declare const __INLINE__: boolean;
declare const __WASM_DATA_URL__: string | undefined;
declare var require: any;

import type {
  Backend,
  ByteRangeResult,
  EicSettings,
  FileHandle,
  GridSettings,
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
import { readIonImage } from "./imageBridge";
import { readRecords, readScanMetadata } from "./recordBridge";
import { encodeTargetIds } from "./pack";
import { readScanArrays, readScans } from "./scanBridge";
import { QuantionError, invalidArgument, readNativeError } from "./errors";
import { TAG_SEPARATOR, getShapeCode } from "./options";
import {
  fetchHeader,
  newRemoteSource,
  prefetchRanges as prefetchRangesForSource,
  type RemoteSource,
} from "./remoteSource";

const IS_INLINE_BUILD = typeof __INLINE__ !== "undefined" && __INLINE__;
const OUTPUT_BUFFER_PAIR_BYTES = 8;
const NOISE_LEVEL_SLOT_BYTES = 16;
const NOISE_LEVEL_INTENSITY_OFFSET = 8;
const PEAK_OPTIONS_STRUCT_BYTES = 80;
const QUANTION_ABI_VERSION = 2;
const IMAGE_BATCH_SCANS = 256;
const LARGEST_WASM_SIZE = 4294967295;
const IMAGE_BATCH_BUDGET_BYTES = 192 * 1024 * 1024;

class RangeSourceRegistry {
  private sources = new Map<number, RemoteSource>();
  private next_id = 1;

  add(source: RemoteSource): number {
    const id = this.next_id++;
    this.sources.set(id, source);
    return id;
  }

  get(id: number): RemoteSource | undefined {
    return this.sources.get(id);
  }

  remove(id: number): void {
    this.sources.delete(id);
  }
}

const range_sources = new RangeSourceRegistry();
let wasm_memory: WebAssembly.Memory | null = null;

function resolveTextDecoder(): TextDecoder {
  return typeof TextDecoder !== "undefined"
    ? new TextDecoder("utf-8")
    : new (require("node:util").TextDecoder)("utf-8");
}

function writeF64(view: DataView, offset: number, value: number): void {
  view.setFloat64(offset, value, true);
}

function writeI32(
  view: DataView,
  offset: number,
  value: number | boolean,
): void {
  view.setInt32(
    offset,
    typeof value === "boolean" ? (value ? 1 : 0) : value | 0,
    true,
  );
}

function packPeakOptions(options: PeakSettings): Uint8Array {
  const view = new DataView(new ArrayBuffer(PEAK_OPTIONS_STRUCT_BYTES));
  writeF64(view, 0, options.minIntegral);
  writeF64(view, 8, options.minIntensity);
  writeI32(view, 16, options.minPeakWidthPoints);
  writeI32(view, 20, getShapeCode(options.shape));
  writeF64(view, 24, options.noise);
  writeI32(view, 32, options.autoNoise);
  writeI32(view, 36, options.autoBaseline);
  writeI32(view, 40, options.maxIterations);
  writeI32(view, 44, options.allowOverlap);
  writeF64(view, 48, options.lambda);
  writeF64(view, 56, options.minSnr);
  writeF64(view, 64, options.minR2);
  writeI32(view, 72, options.kernelSize);
  return new Uint8Array(view.buffer);
}

function total_range_bytes(ranges: ByteRangeResult[]): bigint {
  let total = 0n;
  for (const range of ranges) total += range.length;
  return total;
}

type ExportFunction = (...values: number[]) => number;

class TrapGuard {
  private trapped = false;

  get isTrapped(): boolean {
    return this.trapped;
  }

  watch<T extends Function>(call: T): T {
    const guard = this;
    return function (this: unknown, ...args: unknown[]) {
      if (guard.trapped) {
        throw new Error(
          "quantion: this WASM instance trapped earlier and cannot be used; load it again",
        );
      }
      try {
        return (call as unknown as (...rest: unknown[]) => unknown).apply(
          this,
          args,
        );
      } catch (reason) {
        if (reason instanceof WebAssembly.RuntimeError) guard.trapped = true;
        throw reason;
      }
    } as unknown as T;
  }
}

class WasmExports {
  readonly memory: WebAssembly.Memory;
  readonly alloc: (size: number) => number;
  readonly free: (ptr: number, size: number) => void;
  readonly guard = new TrapGuard();
  private readonly functions = new Map<string, ExportFunction>();

  constructor(instance: WebAssembly.Instance) {
    const exports = instance.exports;
    this.memory = exports.memory as WebAssembly.Memory;
    for (const [name, value] of Object.entries(exports)) {
      if (typeof value === "function") {
        this.functions.set(
          name,
          this.guard.watch(value as unknown as ExportFunction),
        );
      }
    }
    this.alloc = this.get("alloc");
    this.free = this.get("free_");
  }

  has(name: string): boolean {
    return this.functions.has(name);
  }

  get(name: string): ExportFunction {
    const found = this.functions.get(name);
    if (!found) {
      throw new QuantionError(
        "internal",
        name,
        "is not in this WASM build; rebuild it",
      );
    }
    return found;
  }

  call(name: string, ...values: number[]): number {
    return this.get(name)(...values);
  }
}

class WasmApi {
  private readonly heap: WasmHeap;
  readonly fn: WasmExports;
  private readonly firstSlot: number;
  private readonly secondSlot: number;
  private readonly errorSlot: number;
  private readonly peakOptionsSlot: number;
  private readonly handleSlot: number;
  private readonly noiseLevelSlot: number;
  private readonly sourceIdByHandle = new Map<number, number>();

  constructor(instance: WebAssembly.Instance) {
    this.fn = new WasmExports(instance);
    const version = this.fn.call("quantion_abi_version");
    if (version !== QUANTION_ABI_VERSION) {
      throw new QuantionError(
        "internal",
        "quantion",
        `the WASM module has ABI ${version}, the wrapper needs ${QUANTION_ABI_VERSION}; rebuild it`,
      );
    }
    const size = this.fn.call("quantion_sizeof_peak_options");
    if (size !== PEAK_OPTIONS_STRUCT_BYTES) {
      throw new QuantionError(
        "internal",
        "quantion",
        `the WASM module peak options are ${size} bytes, the wrapper needs ${PEAK_OPTIONS_STRUCT_BYTES}; rebuild it`,
      );
    }
    this.heap = new WasmHeap(this.fn.memory, this.fn.alloc, this.fn.free);
    this.firstSlot = this.heap.allocPermanent(OUTPUT_BUFFER_PAIR_BYTES);
    this.secondSlot = this.heap.allocPermanent(OUTPUT_BUFFER_PAIR_BYTES);
    this.errorSlot = this.heap.allocPermanent(OUTPUT_BUFFER_PAIR_BYTES);
    this.peakOptionsSlot = this.heap.allocPermanent(PEAK_OPTIONS_STRUCT_BYTES);
    this.handleSlot = this.heap.allocPermanent(4);
    this.noiseLevelSlot = this.heap.allocPermanent(NOISE_LEVEL_SLOT_BYTES);
  }

  private fail(name: string, code: number): QuantionError {
    if (this.fn.call("quantion_last_error", this.errorSlot) === 0) {
      const { ptr, len } = this.heap.readOutputSlot(this.errorSlot);
      const text = this.heap.decoder.decode(this.heap.copyOutAndFree(ptr, len));
      if (text) return readNativeError(text);
    }
    return new QuantionError(
      "internal",
      name,
      `the call failed with code ${code} (this is a bug in quantion, please report it)`,
    );
  }

  private run(name: string, ...values: number[]): void {
    const code = this.fn.call(name, ...values);
    if (code !== 0) throw this.fail(name, code);
  }

  private withMemory<T>(
    data: Uint8Array[],
    work: (pointers: number[]) => T,
  ): T {
    const allocations = data.map((bytes) => this.heap.allocAndWrite(bytes));
    try {
      return work(allocations.map(([pointer]) => pointer));
    } finally {
      this.heap.freeMany(allocations);
    }
  }

  private takeBytes(slot: number): Uint8Array {
    const { ptr, len } = this.heap.readOutputSlot(slot);
    return this.heap.copyOutAndFree(ptr, len);
  }

  private takeNumbers(slot: number): Float64Array {
    const bytes = this.takeBytes(slot);
    return new Float64Array(
      bytes.buffer,
      bytes.byteOffset,
      bytes.byteLength / 8,
    );
  }

  private takeRanges(slot: number): ByteRangeResult[] {
    const bytes = this.takeBytes(slot);
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const ranges: ByteRangeResult[] = [];
    for (let offset = 0; offset + 16 <= bytes.length; offset += 16) {
      ranges.push({
        offset: view.getBigUint64(offset, true),
        length: view.getBigUint64(offset + 8, true),
      });
    }
    return ranges;
  }

  private takeRecords<T>(slot: number): T[] {
    return readRecords(this.heap.readBridgeFromSlot(slot)) as unknown as T[];
  }

  private peakOptionsPointer(options: PeakSettings): number {
    this.heap.writeBytesAt(this.peakOptionsSlot, packPeakOptions(options));
    return this.peakOptionsSlot;
  }

  private remoteSource(handle: number): RemoteSource | undefined {
    const sourceId = this.sourceIdByHandle.get(handle);
    return sourceId === undefined ? undefined : range_sources.get(sourceId);
  }

  private async prefetch(handle: number, plan: () => void): Promise<void> {
    const source = this.remoteSource(handle);
    if (!source) return;
    plan();
    await prefetchRangesForSource(source, this.takeRanges(this.firstSlot));
  }

  private openHandle(name: string, ...values: number[]): number {
    this.run(name, ...values, this.handleSlot);
    return this.heap.readU32(this.handleSlot);
  }

  parseMzml(data: Uint8Array): number {
    return this.withMemory([data], ([pointer]) =>
      this.openHandle("parse_mzml", pointer, data.length),
    );
  }

  parseBin(data: Uint8Array, cacheSize: number): number {
    return this.withMemory([data], ([pointer]) =>
      this.openHandle("parse_bin", pointer, data.length, cacheSize),
    );
  }

  async parseIonUrl(url: string, cacheSize: number): Promise<number> {
    const source = newRemoteSource(url);
    const sourceId = range_sources.add(source);
    try {
      const header = await fetchHeader(source);
      this.withMemory([header], ([pointer]) =>
        this.run("plan_open", pointer, header.length, this.firstSlot),
      );
      await prefetchRangesForSource(source, this.takeRanges(this.firstSlot));
      const handle = this.openHandle("parse_ion_url", sourceId, cacheSize);
      this.sourceIdByHandle.set(handle, sourceId);
      return handle;
    } catch (error) {
      range_sources.remove(sourceId);
      throw error;
    }
  }

  freeFile(handle: number): void {
    try {
      this.fn.call("free_mzml", handle);
    } finally {
      const sourceId = this.sourceIdByHandle.get(handle);
      if (sourceId !== undefined) {
        range_sources.remove(sourceId);
        this.sourceIdByHandle.delete(handle);
      }
    }
  }

  writeIon(
    handle: number,
    out: string | null,
    overwrite: boolean,
    level: number,
    f32Compress: boolean,
    blockSize: number,
    mzWindow: number,
    headers: Uint8Array,
  ): Uint8Array | undefined {
    const text =
      out === null ? new Uint8Array(0) : new TextEncoder().encode(`${out}\0`);
    return this.withMemory([text, headers], ([outPointer, headersPointer]) => {
      this.run(
        "write_ion",
        handle,
        out === null ? 0 : outPointer,
        overwrite ? 1 : 0,
        level,
        f32Compress ? 1 : 0,
        blockSize,
        mzWindow,
        headers.length ? headersPointer : 0,
        headers.length,
        this.firstSlot,
      );
      const bytes = this.takeBytes(this.firstSlot);
      return bytes.length || out === null ? bytes : undefined;
    });
  }

  writeMzml(handle: number): Uint8Array {
    this.run("write_mzml", handle, 0, 0, this.firstSlot);
    return this.takeBytes(this.firstSlot);
  }

  loadModel(bytes: Uint8Array): number {
    return this.withMemory([bytes], ([pointer]) => {
      this.run(
        "model_load_bytes",
        pointer,
        bytes.length,
        this.handleSlot,
        this.secondSlot,
      );
      return this.heap.readU32(this.handleSlot);
    });
  }

  freeModel(model: number): void {
    this.fn.call("model_free", model);
  }

  modelMetadata(model: number): ModelEntry[] {
    this.run("model_metadata", model, this.firstSlot);
    return this.takeRecords<ModelEntry>(this.firstSlot);
  }

  loadWarp(model: number): number {
    this.run("warp_model_load", model, this.handleSlot, this.secondSlot);
    return this.heap.readU32(this.handleSlot);
  }

  freeWarp(warp: number): void {
    this.fn.call("warp_model_free", warp);
  }

  async prefetchRts(
    warp: number,
    handle: number,
    mzs: Float64Array,
    rts: Float64Array,
  ): Promise<void> {
    await this.prefetch(handle, () =>
      this.withMemory(
        [toUint8View(mzs), toUint8View(rts)],
        ([mzPointer, rtPointer]) =>
          this.run(
            "plan_predict_rts",
            warp,
            handle,
            mzPointer,
            rtPointer,
            mzs.length,
            this.firstSlot,
          ),
      ),
    );
  }

  predictRts(
    warp: number,
    handle: number,
    mzs: Float64Array,
    rts: Float64Array,
  ): Float64Array {
    this.withMemory(
      [toUint8View(mzs), toUint8View(rts)],
      ([mzPointer, rtPointer]) =>
        this.run(
          "predict_rts_from_file",
          warp,
          handle,
          mzPointer,
          rtPointer,
          mzs.length,
          this.firstSlot,
        ),
    );
    return this.takeNumbers(this.firstSlot);
  }

  calibrateRts(
    warp: number,
    handles: number[],
    listLengths: number[],
    ids: string[],
    mzs: Float64Array,
    rts: Float64Array,
  ): Target[] {
    const { packedBytes, offsets, lengths } = encodeTargetIds(ids);
    this.withMemory(
      [
        toUint8View(Uint32Array.from(handles)),
        toUint8View(Uint32Array.from(listLengths)),
        toUint8View(offsets),
        toUint8View(lengths),
        packedBytes,
        toUint8View(mzs),
        toUint8View(rts),
      ],
      ([files, counts, idOffsets, idLengths, idText, mzPointer, rtPointer]) =>
        this.run(
          "calibrate_rts",
          warp,
          files,
          handles.length,
          counts,
          listLengths.length,
          idOffsets,
          idLengths,
          idText,
          packedBytes.length,
          mzPointer,
          rtPointer,
          mzs.length,
          this.firstSlot,
        ),
    );
    return this.takeRecords<Target>(this.firstSlot);
  }

  async calculateEic(
    handle: number,
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
    await this.prefetch(handle, () =>
      this.run("plan_eic", handle, ...values, this.firstSlot),
    );
    this.run(
      "calculate_eic",
      handle,
      ...values,
      this.firstSlot,
      this.secondSlot,
    );
    return {
      x: this.takeNumbers(this.firstSlot),
      y: this.takeNumbers(this.secondSlot),
    };
  }

  async getScans(
    handle: number,
    queryType: number,
    from: number,
    to: number,
    level: number,
    timeUnit: number,
  ): Promise<Scan[]> {
    const values = [queryType, from, to, level, timeUnit];
    await this.prefetch(handle, () =>
      this.run("plan_scans", handle, ...values, this.firstSlot),
    );
    this.run("get_scans", handle, ...values, this.firstSlot);
    return readScans(this.heap.readBridgeFromSlot(this.firstSlot));
  }

  async getScansWithArrays(
    handle: number,
    queryType: number,
    from: number,
    to: number,
    level: number,
    timeUnit: number,
    accessions: Uint32Array,
    keys: string[],
  ): Promise<Scan[]> {
    const values = [queryType, from, to, level, timeUnit];
    await this.prefetch(handle, () =>
      this.withMemory([toUint8View(accessions)], ([pointer]) =>
        this.run(
          "plan_scans_with_arrays",
          handle,
          ...values,
          pointer,
          accessions.length,
          this.firstSlot,
        ),
      ),
    );
    this.withMemory([toUint8View(accessions)], ([pointer]) =>
      this.run(
        "get_scans_with_arrays",
        handle,
        ...values,
        pointer,
        accessions.length,
        this.firstSlot,
      ),
    );
    return readScanArrays(this.heap.readBridgeFromSlot(this.firstSlot), keys);
  }

  private async computeImageStreaming(
    source: RemoteSource,
    handle: number,
    mz: number,
    tolerance: number,
    level: number,
    onProgress: (fetched: number, total: number, heldBytes: number) => void,
  ): Promise<IonImage> {
    const session = this.openHandle(
      "image_begin",
      handle,
      mz,
      tolerance,
      level,
    );
    try {
      this.run("image_scan_count", session, this.handleSlot);
      const total = this.heap.readU32(this.handleSlot);
      onProgress(0, total, source.cache.bytes());
      source.cache.pin();
      const planBatch = (from: number, count: number) => {
        this.run("image_ranges", handle, session, from, count, this.firstSlot);
        return this.takeRanges(this.firstSlot);
      };
      let done = 0;
      while (done < total) {
        let count = Math.min(IMAGE_BATCH_SCANS, total - done);
        let ranges = planBatch(done, count);
        while (
          count > 1 &&
          total_range_bytes(ranges) > BigInt(IMAGE_BATCH_BUDGET_BYTES)
        ) {
          count = Math.floor(count / 2);
          ranges = planBatch(done, count);
        }
        await prefetchRangesForSource(source, ranges);
        this.run("image_fold", handle, session, done, count);
        source.cache.dropUnpinned();
        done += count;
        onProgress(done, total, source.cache.bytes());
      }
      this.run("image_finish", session, this.firstSlot);
      return readIonImage(this.heap.readBridgeFromSlot(this.firstSlot));
    } finally {
      this.fn.call("image_free", session);
    }
  }

  async getScanMetadata(
    handle: number,
    index: number,
    tags: string[],
  ): Promise<ScanParam[]> {
    if (index > LARGEST_WASM_SIZE) {
      throw invalidArgument(
        "getScanMetadata",
        "index",
        `must be at most ${LARGEST_WASM_SIZE} in the WebAssembly build, got ${index}`,
      );
    }
    await this.prefetch(handle, () =>
      this.run("plan_scan_metadata", handle, index, this.firstSlot),
    );
    const text = new TextEncoder().encode(`${tags.join(TAG_SEPARATOR)}\0`);
    this.withMemory([text], ([tagsPointer]) => {
      this.run("get_scan_metadata", handle, index, tagsPointer, this.firstSlot);
    });
    return readScanMetadata(this.heap.readBridgeFromSlot(this.firstSlot));
  }

  async getIonImage(
    handle: number,
    mz: number,
    tolerance: number,
    level: number,
    onProgress: (fetched: number, total: number, heldBytes: number) => void,
  ): Promise<IonImage> {
    const source = this.remoteSource(handle);
    if (source) {
      return this.computeImageStreaming(
        source,
        handle,
        mz,
        tolerance,
        level,
        onProgress,
      );
    }
    this.run("get_ion_image", handle, mz, tolerance, level, this.firstSlot);
    return readIonImage(this.heap.readBridgeFromSlot(this.firstSlot));
  }

  findPeaks(x: Float64Array, y: Float64Array, options: PeakSettings): Peak[] {
    this.withMemory([toUint8View(x), toUint8View(y)], ([xPointer, yPointer]) =>
      this.run(
        "find_peaks",
        xPointer,
        yPointer,
        x.length,
        this.peakOptionsPointer(options),
        this.firstSlot,
      ),
    );
    return this.takeRecords<Peak>(this.firstSlot);
  }

  getPeak(
    x: Float64Array,
    y: Float64Array,
    rt: number,
    range: number,
    options: PeakSettings,
  ): Peak {
    this.withMemory([toUint8View(x), toUint8View(y)], ([xPointer, yPointer]) =>
      this.run(
        "get_peak",
        xPointer,
        yPointer,
        x.length,
        rt,
        range,
        this.peakOptionsPointer(options),
        this.firstSlot,
      ),
    );
    return this.takeRecords<Peak>(this.firstSlot)[0];
  }

  fitPeak(
    x: Float64Array,
    y: Float64Array,
    rt: number,
    shape: number,
    intensity: number,
  ): PeakParameters | null {
    this.withMemory([toUint8View(x), toUint8View(y)], ([xPointer, yPointer]) =>
      this.run(
        "fit_peak",
        xPointer,
        yPointer,
        x.length,
        rt,
        shape,
        intensity,
        this.firstSlot,
      ),
    );
    const rows = this.takeRecords<PeakParameters>(this.firstSlot);
    return rows.length ? rows[0] : null;
  }

  drawPeaks(
    peaks: PeakColumns,
    from: number,
    to: number,
    points: number,
  ): DataXY {
    this.withMemory(
      [
        toUint8View(peaks.heights),
        toUint8View(peaks.centers),
        toUint8View(peaks.fwhms),
        toUint8View(peaks.tails),
      ],
      ([heights, centers, fwhms, tails]) =>
        this.run(
          "draw_peaks",
          heights,
          centers,
          fwhms,
          tails,
          peaks.heights.length,
          from,
          to,
          points,
          this.firstSlot,
          this.secondSlot,
        ),
    );
    return {
      x: this.takeNumbers(this.firstSlot),
      y: this.takeNumbers(this.secondSlot),
    };
  }

  findNoiseLevel(y: Float32Array): NoiseLevel {
    this.withMemory([toUint8View(y)], ([yPointer]) =>
      this.run(
        "find_noise_level",
        yPointer,
        y.length,
        this.noiseLevelSlot,
        this.noiseLevelSlot + NOISE_LEVEL_INTENSITY_OFFSET,
      ),
    );
    return {
      width: this.heap.readU32(this.noiseLevelSlot),
      intensity: this.heap.readF64(
        this.noiseLevelSlot + NOISE_LEVEL_INTENSITY_OFFSET,
      ),
    };
  }

  calculateBaseline(
    y: Float64Array,
    lambda: number,
    maxIterations: number,
  ): Float64Array {
    this.withMemory([toUint8View(y)], ([yPointer]) =>
      this.run(
        "calculate_baseline",
        yPointer,
        y.length,
        lambda,
        maxIterations,
        this.firstSlot,
      ),
    );
    return this.takeNumbers(this.firstSlot);
  }

  deisotopeFeatures(
    values: Float64Array,
    count: number,
    rtTolerance: number,
    mzTolerance: number,
  ): Feature[] {
    this.withMemory([toUint8View(values)], ([valuesPointer]) =>
      this.run(
        "deisotope_features",
        valuesPointer,
        count,
        rtTolerance,
        mzTolerance,
        this.firstSlot,
      ),
    );
    return this.takeRecords<Feature>(this.firstSlot);
  }

  async findTargets(
    handle: number,
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
      this.withMemory([toUint8View(targets.mzs)], ([mzPointer]) =>
        this.run(
          "plan_find_targets",
          handle,
          mzPointer,
          targets.mzs.length,
          ...window,
          this.firstSlot,
        ),
      ),
    );
    const { packedBytes, offsets, lengths } = encodeTargetIds(targets.ids);
    this.withMemory(
      [
        toUint8View(targets.rts),
        toUint8View(targets.mzs),
        toUint8View(targets.ranges),
        toUint8View(offsets),
        toUint8View(lengths),
        packedBytes,
      ],
      ([rts, mzs, ranges, idOffsets, idLengths, idText]) =>
        this.run(
          "find_targets",
          handle,
          rts,
          mzs,
          ranges,
          idOffsets,
          idLengths,
          idText,
          packedBytes.length,
          targets.rts.length,
          ...window,
          this.peakOptionsPointer(options),
          threads,
          this.firstSlot,
        ),
    );
    return this.takeRecords<Peak>(this.firstSlot);
  }

  findTransitions(
    handle: number,
    targets: TransitionColumns,
    options: PeakSettings,
    threads: number,
  ): Peak[] {
    const { packedBytes, offsets, lengths } = encodeTargetIds(targets.ids);
    this.withMemory(
      [
        toUint8View(targets.precursors),
        toUint8View(targets.products),
        toUint8View(targets.rts),
        toUint8View(targets.ranges),
        toUint8View(offsets),
        toUint8View(lengths),
        packedBytes,
      ],
      ([precursors, products, rts, ranges, idOffsets, idLengths, idText]) =>
        this.run(
          "find_transitions",
          handle,
          precursors,
          products,
          rts,
          ranges,
          idOffsets,
          idLengths,
          idText,
          packedBytes.length,
          targets.rts.length,
          this.peakOptionsPointer(options),
          threads,
          this.firstSlot,
        ),
    );
    return this.takeRecords<Peak>(this.firstSlot);
  }

  async findFeatures(
    handle: number,
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
      this.run("plan_find_features", handle, ...values, this.firstSlot),
    );
    this.run(
      "find_features",
      handle,
      ...values,
      this.peakOptionsPointer(options),
      threads,
      this.firstSlot,
    );
    return this.takeRecords<Feature>(this.firstSlot);
  }
}

function make_range_read_import(): (
  source_id: number,
  offset_lo: number,
  offset_hi: number,
  len: number,
  dest_ptr: number,
) => number {
  return (
    source_id: number,
    offset_lo: number,
    offset_hi: number,
    len: number,
    dest_ptr: number,
  ): number => {
    if (!wasm_memory) return -1;

    const source = range_sources.get(source_id >>> 0);
    if (!source) return -1;

    const offset = BigInt(offset_lo >>> 0) + (BigInt(offset_hi >>> 0) << 32n);
    const length = BigInt(len >>> 0);

    const bytes = source.cache.read(offset, length);
    if (!bytes) return -1;

    new Uint8Array(wasm_memory.buffer, dest_ptr >>> 0, len >>> 0).set(bytes);
    return 0;
  };
}

async function load_and_instantiate(
  bytes: ArrayBuffer | Uint8Array,
): Promise<WasmApi> {
  const imports = {
    env: {
      js_log: (_ptr: number, _len: number) => {},
      range_read: make_range_read_import(),
    },
  };
  const result = await WebAssembly.instantiate(bytes, imports);
  const instance = (result as any).instance as WebAssembly.Instance;
  wasm_memory = (instance.exports as any).memory as WebAssembly.Memory;
  return new WasmApi(instance);
}

async function loadWasm(): Promise<WasmApi> {
  if (IS_INLINE_BUILD) {
    if (!__WASM_DATA_URL__)
      throw new Error("Inline build is missing __WASM_DATA_URL__");
    const bytes = await fetch(__WASM_DATA_URL__).then((r) => r.arrayBuffer());
    return load_and_instantiate(bytes);
  }

  throw new Error(
    "quantion: the WASM build must be loaded from the bundle (quantion.js)",
  );
}

export class WasmBackend implements Backend {
  private api: WasmApi | null = null;
  ready = false;
  readonly handlesAreGcFinalized = false;
  readonly canReadPaths = false;

  async init(): Promise<void> {
    if (this.api) return;
    this.api = await loadWasm();
    this.ready = true;
  }

  private getApi(): WasmApi {
    if (!this.api) {
      throw new QuantionError(
        "internal",
        "quantion",
        "the WASM module is not loaded; call init() first",
      );
    }
    return this.api;
  }

  parseMzmlBytes(data: Uint8Array): FileHandle {
    return this.getApi().parseMzml(data);
  }

  parseMzmlPath(): FileHandle {
    throw invalidArgument(
      "parseMzml",
      "source",
      "must be bytes in the browser; paths need Node",
    );
  }

  parseIonPath(): FileHandle {
    throw invalidArgument(
      "parseIon",
      "source",
      "must be a URL or bytes in the browser; paths need Node",
    );
  }

  async parseIonRemote(url: URL, cacheSize: number): Promise<FileHandle> {
    return this.getApi().parseIonUrl(url.href, cacheSize);
  }

  parseIonBuffer(bytes: Uint8Array, cacheSize: number): FileHandle {
    return this.getApi().parseBin(bytes, cacheSize);
  }

  freeFile(handle: FileHandle): void {
    this.getApi().freeFile(handle as number);
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
    return this.getApi().writeIon(
      handle as number,
      out,
      overwrite,
      level,
      f32Compress,
      blockSize,
      mzWindow,
      headers,
    );
  }

  writeMzml(handle: FileHandle): Uint8Array | undefined {
    return this.getApi().writeMzml(handle as number);
  }

  loadModelPath(): ModelHandle {
    throw invalidArgument(
      "loadModel",
      "source",
      "must be a URL or bytes in the browser; paths need Node",
    );
  }

  loadModelBytes(bytes: Uint8Array): ModelHandle {
    return this.getApi().loadModel(bytes);
  }

  freeModel(model: ModelHandle): void {
    this.getApi().freeModel(model as number);
  }

  modelMetadata(model: ModelHandle): ModelEntry[] {
    return this.getApi().modelMetadata(model as number);
  }

  loadWarp(model: ModelHandle): WarpModelHandle {
    return this.getApi().loadWarp(model as number);
  }

  freeWarpModel(warp: WarpModelHandle): void {
    this.getApi().freeWarp(warp as number);
  }

  prefetchRts(
    warp: WarpModelHandle,
    handle: FileHandle,
    mzs: Float64Array,
    rts: Float64Array,
  ): Promise<void> {
    return this.getApi().prefetchRts(
      warp as number,
      handle as number,
      mzs,
      rts,
    );
  }

  predictRts(
    warp: WarpModelHandle,
    handle: FileHandle,
    mzs: Float64Array,
    rts: Float64Array,
  ): Float64Array {
    return this.getApi().predictRts(warp as number, handle as number, mzs, rts);
  }

  calibrateRts(
    warp: WarpModelHandle,
    handles: FileHandle[],
    listLengths: number[],
    ids: string[],
    mzs: Float64Array,
    rts: Float64Array,
  ): Target[] {
    return this.getApi().calibrateRts(
      warp as number,
      handles as number[],
      listLengths,
      ids,
      mzs,
      rts,
    );
  }

  calculateEic(
    handle: FileHandle,
    mz: number,
    eic: EicSettings,
  ): Promise<DataXY> {
    return this.getApi().calculateEic(handle as number, mz, eic);
  }

  getScans(
    handle: FileHandle,
    queryType: number,
    from: number,
    to: number,
    level: number,
    timeUnit: number,
  ): Promise<Scan[]> {
    return this.getApi().getScans(
      handle as number,
      queryType,
      from,
      to,
      level,
      timeUnit,
    );
  }

  getScansWithArrays(
    handle: FileHandle,
    queryType: number,
    from: number,
    to: number,
    level: number,
    timeUnit: number,
    accessions: Uint32Array,
    keys: string[],
  ): Promise<Scan[]> {
    return this.getApi().getScansWithArrays(
      handle as number,
      queryType,
      from,
      to,
      level,
      timeUnit,
      accessions,
      keys,
    );
  }

  getScanMetadata(
    handle: FileHandle,
    index: number,
    tags: string[],
  ): Promise<ScanParam[]> {
    return this.getApi().getScanMetadata(handle as number, index, tags);
  }

  getIonImage(
    handle: FileHandle,
    mz: number,
    tolerance: number,
    level: number,
    onProgress: (fetched: number, total: number, heldBytes: number) => void,
  ): Promise<IonImage> {
    return this.getApi().getIonImage(
      handle as number,
      mz,
      tolerance,
      level,
      onProgress,
    );
  }

  findPeaks(x: Float64Array, y: Float64Array, options: PeakSettings): Peak[] {
    return this.getApi().findPeaks(x, y, options);
  }

  getPeak(
    x: Float64Array,
    y: Float64Array,
    rt: number,
    range: number,
    options: PeakSettings,
  ): Peak {
    return this.getApi().getPeak(x, y, rt, range, options);
  }

  fitPeak(
    x: Float64Array,
    y: Float64Array,
    rt: number,
    shape: number,
    intensity: number,
  ): PeakParameters | null {
    return this.getApi().fitPeak(x, y, rt, shape, intensity);
  }

  drawPeaks(
    peaks: PeakColumns,
    from: number,
    to: number,
    points: number,
  ): DataXY {
    return this.getApi().drawPeaks(peaks, from, to, points);
  }

  findNoiseLevel(y: Float32Array): NoiseLevel {
    return this.getApi().findNoiseLevel(y);
  }

  calculateBaseline(
    y: Float64Array,
    lambda: number,
    maxIterations: number,
  ): Float64Array {
    return this.getApi().calculateBaseline(y, lambda, maxIterations);
  }

  deisotopeFeatures(
    values: Float64Array,
    count: number,
    rtTolerance: number,
    mzTolerance: number,
  ): Feature[] {
    return this.getApi().deisotopeFeatures(
      values,
      count,
      rtTolerance,
      mzTolerance,
    );
  }

  findTargets(
    handle: FileHandle,
    targets: TargetColumns,
    eic: EicSettings,
    options: PeakSettings,
    threads: number,
  ): Promise<Peak[]> {
    return this.getApi().findTargets(
      handle as number,
      targets,
      eic,
      options,
      threads,
    );
  }

  findTransitions(
    handle: FileHandle,
    targets: TransitionColumns,
    options: PeakSettings,
    threads: number,
  ): Peak[] {
    return this.getApi().findTransitions(
      handle as number,
      targets,
      options,
      threads,
    );
  }

  findFeatures(
    handle: FileHandle,
    eic: EicSettings,
    grid: GridSettings,
    options: PeakSettings,
    threads: number,
  ): Promise<Feature[]> {
    return this.getApi().findFeatures(
      handle as number,
      eic,
      grid,
      options,
      threads,
    );
  }
}

function toUint8View(typed: {
  buffer: ArrayBufferLike;
  byteOffset: number;
  byteLength: number;
}): Uint8Array {
  const buffer =
    typed.buffer instanceof ArrayBuffer
      ? typed.buffer
      : new Uint8Array(typed.buffer).buffer;
  return new Uint8Array(buffer, typed.byteOffset, typed.byteLength);
}

class WasmHeap {
  private memory: WebAssembly.Memory;
  private bytes: Uint8Array;
  private view: DataView;
  private readonly alloc: (size: number) => number;
  private readonly free: (ptr: number, size: number) => void;
  readonly decoder: TextDecoder;

  constructor(
    memory: WebAssembly.Memory,
    alloc: (size: number) => number,
    free: (ptr: number, size: number) => void,
  ) {
    this.memory = memory;
    this.bytes = new Uint8Array(memory.buffer);
    this.view = new DataView(memory.buffer);
    this.alloc = alloc;
    this.free = free;
    this.decoder = resolveTextDecoder();
  }

  private refresh(): void {
    if (this.bytes.buffer !== this.memory.buffer) {
      this.bytes = new Uint8Array(this.memory.buffer);
      this.view = new DataView(this.memory.buffer);
    }
  }

  allocAndWrite(data: Uint8Array): [ptr: number, len: number] {
    const ptr = this.alloc(data.length);
    this.refresh();
    this.bytes.set(data, ptr);
    return [ptr, data.length];
  }

  freeMany(allocations: [ptr: number, len: number][]): void {
    for (const [ptr, len] of allocations) this.free(ptr, len);
  }

  readU32(ptr: number): number {
    this.refresh();
    return this.view.getUint32(ptr, true);
  }

  readF64(ptr: number): number {
    this.refresh();
    return this.view.getFloat64(ptr, true);
  }

  readOutputSlot(slotPtr: number): { ptr: number; len: number } {
    this.refresh();
    return {
      ptr: this.view.getUint32(slotPtr, true),
      len: this.view.getUint32(slotPtr + 4, true),
    };
  }

  copyOut(ptr: number, len: number): Uint8Array {
    this.refresh();
    const copy = new Uint8Array(len);
    copy.set(this.bytes.subarray(ptr, ptr + len));
    return copy;
  }

  copyOutAndFree(ptr: number, len: number): Uint8Array {
    const data = this.copyOut(ptr, len);
    this.free(ptr, len);
    return data;
  }

  readBridgeFromSlot(slotPtr: number): ArrayBuffer {
    const { ptr, len } = this.readOutputSlot(slotPtr);
    const bytes = this.copyOutAndFree(ptr, len);
    return bytes.buffer as ArrayBuffer;
  }

  writeBytesAt(ptr: number, data: Uint8Array): void {
    this.refresh();
    this.bytes.set(data, ptr);
  }

  allocPermanent(size: number): number {
    return this.alloc(size);
  }
}
