import type { Backend, ModelHandle } from "./backend";
import { fileHandleRegistry } from "./handleRegistry";
import { Metadata } from "./metadata";
import { invalidArgument } from "./errors";
import type { ModelEntry } from "../types/internal";

type Cell = ReturnType<typeof fileHandleRegistry.register>;

/**
 * A loaded ONNX model. Load it with {@link loadModel} and read its stored keys and values.
 */
export class Model extends Metadata {
  /** @internal */
  _backend: Backend;

  private _cell: Cell | null;

  private metadata: ModelEntry[] | null = null;

  constructor(handle: ModelHandle, backend: Backend) {
    super();
    this._backend = backend;
    this._cell = fileHandleRegistry.register(this, handle, (h) =>
      backend.freeModel(h),
    );
  }

  get _handle(): ModelHandle | null {
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
    fileHandleRegistry.release(this, cell);
  }

  /** @internal */
  _getMetadata(caller: string): ModelEntry[] {
    if (this.metadata === null) {
      const handle = this._handle;
      if (handle === null) {
        throw invalidArgument(caller, "model", "is not open; it was disposed");
      }
      this.metadata = this._backend.modelMetadata(handle);
    }
    return this.metadata;
  }
}
