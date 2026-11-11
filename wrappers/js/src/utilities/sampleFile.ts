import type { Backend, FileHandle } from "./backend";
import { fileHandleRegistry } from "./handleRegistry";

type Cell = ReturnType<typeof fileHandleRegistry.register>;

/**
 * SampleFile · Inputs
 *
 * An opened sample from parse_ion or parse_mzml. Free it with dispose when you are done.
 * @see {@link https://phenological.github.io/quantion/?language=js#SampleFile | Documentation}
 */
export class SampleFile {
  /** @internal */
  _backend: Backend;

  private _cell: Cell | null;

  constructor(handle: FileHandle, backend: Backend) {
    this._backend = backend;

    if (backend.handlesAreGcFinalized) {
      this._cell = { handle, free: () => {} };
    } else {
      this._cell = fileHandleRegistry.register(this, handle, (h) =>
        backend.freeFile(h),
      );
    }
  }

  get _handle(): FileHandle | null {
    return this._cell?.handle ?? null;
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

    if (this._backend.handlesAreGcFinalized) {
      const h = cell.handle;
      if (h != null) {
        this._backend.freeFile(h);
        cell.handle = null;
      }
    } else {
      fileHandleRegistry.release(this, cell);
    }
  }
}
