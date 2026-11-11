import type { IonInput } from "../types/types";
import { toUint8 } from "./shared";
import { invalidArgument } from "./errors";

export interface PathSource {
  kind: "path";
  path: string;
}

export interface UrlSource {
  kind: "url";
  url: URL;
}

export interface BufferSource {
  kind: "buffer";
  bytes: Uint8Array;
}

export type IonSource = PathSource | UrlSource | BufferSource;

function isUrlString(value: string): boolean {
  return (
    /^[A-Za-z][A-Za-z0-9+.-]*:/.test(value) && !/^[A-Za-z]:[\\/]/.test(value)
  );
}

function getPathSource(value: string, caller: string): IonSource {
  if (value.length === 0) {
    throw invalidArgument(caller, "source", "must not be an empty path");
  }
  if (/^https?:/i.test(value)) {
    return { kind: "url", url: new URL(value) };
  }
  if (isUrlString(value)) {
    throw invalidArgument(caller, "source", "must be a path or an http(s) URL");
  }
  return { kind: "path", path: value };
}

function isBytes(value: unknown): value is Uint8Array | ArrayBuffer {
  return value instanceof Uint8Array || value instanceof ArrayBuffer;
}

export function getIonSource(input: IonInput, caller: string): IonSource {
  if (typeof input === "string") return getPathSource(input, caller);
  if (input instanceof URL) {
    if (input.protocol !== "http:" && input.protocol !== "https:") {
      throw invalidArgument(caller, "source", "must be an http(s) URL");
    }
    return { kind: "url", url: input };
  }
  if (isBytes(input)) return { kind: "buffer", bytes: toUint8(input) };
  throw invalidArgument(caller, "source", "must be a path, a URL or bytes");
}
