import type { ModelEntry } from "../types/internal";

export abstract class Metadata {
  protected abstract _getMetadata(caller: string): ModelEntry[];

  /**
   * The keys stored in the model file, in file order.
   */
  getKeys(): string[] {
    return this._getMetadata("getKeys").map((row) => row.key);
  }

  /**
   * The values stored in the model file, in the same order as {@link getKeys}.
   */
  getValues(): string[] {
    return this._getMetadata("getValues").map((row) => row.value);
  }

  /**
   * The text stored under `key` in the model file, or `undefined` if it is missing.
   */
  getValue(key: string): string | undefined {
    return this._getMetadata("getValue").find((row) => row.key === key)?.value;
  }
}
