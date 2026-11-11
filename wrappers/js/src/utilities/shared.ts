import type { BinaryInput } from "../types/types";

export function toUint8(input: BinaryInput): Uint8Array {
  if (input instanceof Uint8Array) return input;
  return new Uint8Array(input);
}
