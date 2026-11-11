import { describe, test, expect } from "@jest/globals";
import { getIonSource as readSource } from "../utilities/ionSource";

const getIonSource = (input: unknown) => readSource(input as never, "parseIon");

describe("ionSource classifier", () => {
  describe("path source", () => {
    test("empty string throws", () => {
      expect(() => getIonSource("")).toThrow(
        "parseIon: source must not be an empty path",
      );
    });

    test("string with http scheme returns url source", () => {
      const source = getIonSource("http://example.com/file.ion");
      expect(source.kind).toBe("url");
      expect((source as { kind: "url"; url: URL }).url.href).toBe(
        "http://example.com/file.ion",
      );
    });

    test("string with https scheme returns url source", () => {
      const source = getIonSource("https://example.com/file.ion");
      expect(source.kind).toBe("url");
      expect((source as { kind: "url"; url: URL }).url.href).toBe(
        "https://example.com/file.ion",
      );
    });

    test("string with file scheme throws", () => {
      expect(() => getIonSource("file:///data/file.ion")).toThrow(
        "parseIon: source must be a path or an http(s) URL",
      );
    });

    test("string with ftp scheme throws", () => {
      expect(() => getIonSource("ftp://example.com/file.ion")).toThrow(
        "parseIon: source must be a path or an http(s) URL",
      );
    });

    test("string with s3 scheme throws", () => {
      expect(() => getIonSource("s3://bucket/file.ion")).toThrow(
        "parseIon: source must be a path or an http(s) URL",
      );
    });

    test("absolute path returns path source", () => {
      const source = getIonSource("/data/file.ion");
      expect(source).toEqual({ kind: "path", path: "/data/file.ion" });
    });

    test("relative path returns path source", () => {
      const source = getIonSource("./data/file.ion");
      expect(source).toEqual({ kind: "path", path: "./data/file.ion" });
    });

    test("windows absolute path returns path source", () => {
      const source = getIonSource("C:\\data\\file.ion");
      expect(source).toEqual({ kind: "path", path: "C:\\data\\file.ion" });
    });
  });

  describe("url source", () => {
    test("http URL returns url source", () => {
      const url = new URL("http://example.com/file.ion");
      const source = getIonSource(url);
      expect(source.kind).toBe("url");
      expect((source as { kind: "url"; url: URL }).url).toBe(url);
    });

    test("https URL returns url source", () => {
      const url = new URL("https://example.com/file.ion");
      const source = getIonSource(url);
      expect(source.kind).toBe("url");
      expect((source as { kind: "url"; url: URL }).url).toBe(url);
    });

    test("file URL throws", () => {
      expect(() => getIonSource(new URL("file:///data/file.ion"))).toThrow(
        "parseIon: source must be an http(s) URL",
      );
    });

    test("ftp URL throws", () => {
      expect(() => getIonSource(new URL("ftp://example.com/file.ion"))).toThrow(
        "parseIon: source must be an http(s) URL",
      );
    });

    test("data URL throws", () => {
      expect(() =>
        getIonSource(new URL("data:application/octet-stream,x")),
      ).toThrow("parseIon: source must be an http(s) URL");
    });
  });

  describe("buffer source", () => {
    test("Uint8Array returns buffer source", () => {
      const bytes = new Uint8Array([1, 2, 3]);
      const source = getIonSource(bytes);
      expect(source.kind).toBe("buffer");
      expect((source as { kind: "buffer"; bytes: Uint8Array }).bytes).toEqual(
        bytes,
      );
    });

    test("ArrayBuffer returns buffer source", () => {
      const buffer = new ArrayBuffer(4);
      const source = getIonSource(buffer);
      expect(source.kind).toBe("buffer");
      const bytes = (source as { kind: "buffer"; bytes: Uint8Array }).bytes;
      expect(bytes).toBeInstanceOf(Uint8Array);
      expect(bytes.byteLength).toBe(4);
    });
  });

  describe("invalid input", () => {
    test("null throws", () => {
      expect(() => getIonSource(null as any)).toThrow(
        "parseIon: source must be a path, a URL or bytes",
      );
    });

    test("undefined throws", () => {
      expect(() => getIonSource(undefined as any)).toThrow(
        "parseIon: source must be a path, a URL or bytes",
      );
    });

    test("number throws", () => {
      expect(() => getIonSource(42 as any)).toThrow(
        "parseIon: source must be a path, a URL or bytes",
      );
    });

    test("plain object throws", () => {
      expect(() => getIonSource({} as any)).toThrow(
        "parseIon: source must be a path, a URL or bytes",
      );
    });
  });
});
