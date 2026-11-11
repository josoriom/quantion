import type { Backend, WarpModelHandle } from "./backend";
import { fileHandleRegistry } from "./handleRegistry";
import { Metadata } from "./metadata";
import { SampleFile } from "./sampleFile";
import { invalidArgument } from "./errors";
import { checkItems, getRequiredNumber, TARGET_KEYS } from "./options";
import type { PredictedRt, Target } from "../types/types";
import type { ModelEntry } from "../types/internal";

type Cell = ReturnType<typeof fileHandleRegistry.register>;

/**
 * A warp model. Make it once with {@link loadWarp} and use it
 * with predictRts for every file.
 */
export class WarpModel extends Metadata {
  /** @internal */
  _backend: Backend;

  private _cell: Cell | null;

  private metadata: ModelEntry[];

  constructor(
    handle: WarpModelHandle,
    backend: Backend,
    metadata: ModelEntry[],
  ) {
    super();
    this._backend = backend;
    this.metadata = metadata;
    this._cell = fileHandleRegistry.register(this, handle, (h) =>
      backend.freeWarpModel(h),
    );
  }

  get _handle(): WarpModelHandle | null {
    return this._cell?.handle ?? null;
  }

  /**
   * predictRts · Models
   *
   * Predict where each target elutes in one sample.
   *
   * @param file - The sample to read, from parse_ion or parse_mzml.
   * @param targets - Targets with mz and reference rt. id and range are optional and not used. Each Target has `id`, `mz`, `rt`, `range`. See {@link https://phenological.github.io/quantion/?language=js#Target | Target}.
   *   Example: `const target: Target = { id: "compound_1", mz: 260.103, rt: 2.383, range: 0.05 };`
   * @returns One PredictedRt per target, in the same order.
   * @see {@link https://phenological.github.io/quantion/?language=js#predict_rts | Documentation}
   */
  async predictRts(
    file: SampleFile,
    targets: Target[],
  ): Promise<PredictedRt[]> {
    checkItems(targets, TARGET_KEYS, "predictRts", "targets");
    const warp = this.getHandle("predictRts");
    const handle = getFileHandle(file, "predictRts");
    if (targets.length === 0) return [];

    const mzs = Float64Array.from(targets, (target, index) =>
      getRequiredNumber(target.mz, "predictRts", `targets[${index}].mz`),
    );
    const rts = Float64Array.from(targets, (target, index) =>
      getRequiredNumber(target.rt, "predictRts", `targets[${index}].rt`),
    );
    await this._backend.prefetchRts(warp, handle, mzs, rts);
    const values = this._backend.predictRts(warp, handle, mzs, rts);
    return targets.map((_, index) => ({
      rt: values[index * 2],
      uncertainty: values[index * 2 + 1],
    }));
  }

  /**
   * calibrate · Models
   *
   * Turn the retention times seen in anchor samples (LTRs, QCs) into targets for predict_rts. Each id gets the median rt over the anchors.
   *
   * @param anchorSamples - Anchor samples.
   * @param targetLists - For each anchor sample, the targets seen in it with their observed rt. Each Target has `id`, `mz`, `rt`, `range`. See {@link https://phenological.github.io/quantion/?language=js#Target | Target}.
   *   Example: `const target: Target = { id: "compound_1", mz: 260.103, rt: 2.383, range: 0.05 };`
   * @returns One Target per id with the median rt.
   * @see {@link https://phenological.github.io/quantion/?language=js#calibrate | Documentation}
   */
  async calibrate(
    anchorSamples: SampleFile[],
    targetLists: Target[][],
  ): Promise<Target[]> {
    for (let index = 0; index < targetLists.length; index++) {
      const argument = `targetLists[${index}]`;
      checkItems(targetLists[index], TARGET_KEYS, "calibrate", argument);
      for (let item = 0; item < targetLists[index].length; item++) {
        const target = targetLists[index][item];
        getRequiredNumber(target.mz, "calibrate", `${argument}[${item}].mz`);
        getRequiredNumber(target.rt, "calibrate", `${argument}[${item}].rt`);
      }
    }
    const warp = this.getHandle("calibrate");
    const handles = anchorSamples.map((sample) =>
      getFileHandle(sample, "calibrate"),
    );
    for (const [index, targets] of targetLists.entries()) {
      if (index >= handles.length) break;
      await this._backend.prefetchRts(
        warp,
        handles[index],
        Float64Array.from(targets, (target) => target.mz),
        Float64Array.from(targets, (target) => target.rt),
      );
    }
    const targets = targetLists.flat();
    return this._backend.calibrateRts(
      warp,
      handles,
      targetLists.map((list) => list.length),
      targets.map((target) => target.id ?? ""),
      Float64Array.from(targets, (target) => target.mz),
      Float64Array.from(targets, (target) => target.rt),
    );
  }

  /**
   * dispose · Models
   *
   * Free the memory held by a sample, model or warp model.
   *
   * **Warning:** Not needed in normal use: the library frees a sample, model or warp model by itself once it is no longer used. When processing many samples, dispose each one after using it, so its memory is freed right away instead of piling up.
   * @returns Nothing.
   * @see {@link https://phenological.github.io/quantion/?language=js#dispose | Documentation}
   */
  dispose(): void {
    const cell = this._cell;
    if (!cell) return;
    fileHandleRegistry.release(this, cell);
  }

  protected _getMetadata(): ModelEntry[] {
    return this.metadata;
  }

  private getHandle(caller: string): WarpModelHandle {
    const handle = this._handle;
    if (handle === null) {
      throw invalidArgument(caller, "warp", "is not open; it was disposed");
    }
    return handle;
  }
}

function getFileHandle(file: SampleFile, caller: string) {
  if (!(file instanceof SampleFile) || !file._handle) {
    throw invalidArgument(
      caller,
      "file",
      "is not open; it was disposed or never opened",
    );
  }
  return file._handle;
}
