import { describe, it, expect } from "@jest/globals";
import * as path from "path";
import { findBundledFiles, isMusl } from "../utilities/backendNode";

function getProcess(
  platform: string,
  arch: string,
  glibc?: string,
): NodeJS.Process {
  return {
    platform,
    arch,
    report: { getReport: () => ({ header: { glibcVersionRuntime: glibc } }) },
  } as unknown as NodeJS.Process;
}

const folder = "/package/native";

function getAll(): boolean {
  return true;
}

function getNone(): boolean {
  return false;
}

describe("the bundled native build", () => {
  it("is found for a supported platform", () => {
    const files = findBundledFiles(
      getProcess("darwin", "arm64"),
      folder,
      getAll,
    );
    expect(files.addon).toBe(path.join(folder, "macos-arm64", "quantion.node"));
    expect(files.library).toBe(
      path.join(folder, "macos-arm64", "libquantion.dylib"),
    );
  });

  it("is found for Linux with glibc", () => {
    const files = findBundledFiles(
      getProcess("linux", "x64", "2.39"),
      folder,
      getAll,
    );
    expect(files.library).toBe(
      path.join(folder, "linux-x86_64", "libquantion.so"),
    );
  });

  it("says when Linux uses musl", () => {
    expect(isMusl(getProcess("linux", "x64"))).toBe(true);
    expect(() =>
      findBundledFiles(getProcess("linux", "x64"), folder, getAll),
    ).toThrow(
      "quantion: there is no native build for Linux with musl (for example Alpine). Use the WASM build, or set QUANTION_LIB to a libquantion built for this system.",
    );
  });

  it("says when the platform has no build", () => {
    expect(() =>
      findBundledFiles(getProcess("freebsd", "x64"), folder, getAll),
    ).toThrow("quantion: there is no native build for freebsd-x64.");
  });

  it("says when the files are missing", () => {
    expect(() =>
      findBundledFiles(getProcess("win32", "x64"), folder, getNone),
    ).toThrow("quantion: there is no native build for win32-x64.");
  });
});
