import { describe, it, expect, beforeAll, afterAll } from "@jest/globals";
import * as fs from "fs";
import * as http from "http";
import * as path from "path";
import { NodeBackend } from "../utilities/backendNode";
import { setBackend, loadModel, loadWarp, parseIon } from "../utilities/api";

const fixtures = path.join(__dirname, "../../../../core/tests/fixtures");
const modelPath = path.join(fixtures, "shift_model.onnx");
const coveringFile = path.join(fixtures, "api/api.ion");
const emptyFile = path.join(fixtures, "test.ion");
const targets = [
  { id: "first", mz: 250.1, rt: 1.2 },
  { id: "second", mz: 310.2, rt: 3.1 },
];

function getNodeBackend(): NodeBackend | null {
  try {
    return new NodeBackend(process);
  } catch {
    return null;
  }
}

const nodeBackend = getNodeBackend();
const describeWithNode = nodeBackend ? describe : describe.skip;

async function getWarp(source: string | Uint8Array) {
  return loadWarp(await loadModel(source));
}

describeWithNode("warp model", () => {
  let server: http.Server;
  let modelUrl: string;
  let fileUrl: string;

  beforeAll((done) => {
    setBackend(nodeBackend!);
    const modelBytes = fs.readFileSync(modelPath);
    const fileBytes = fs.readFileSync(coveringFile);
    server = http.createServer((request, response) => {
      if (request.url === "/missing.onnx") {
        response.writeHead(404);
        response.end("not found");
        return;
      }
      if (request.url !== "/api.ion") {
        response.writeHead(200, { "content-length": modelBytes.length });
        response.end(modelBytes);
        return;
      }
      const match = (request.headers.range ?? "").match(/bytes=(\d+)-(\d+)/);
      if (!match) {
        response.writeHead(200, { "content-length": fileBytes.length });
        response.end(fileBytes);
        return;
      }
      const first = Number(match[1]);
      const last = Math.min(Number(match[2]), fileBytes.length - 1);
      const piece = fileBytes.subarray(first, last + 1);
      response.writeHead(206, {
        "content-length": piece.length,
        "content-range": `bytes ${first}-${last}/${fileBytes.length}`,
      });
      response.end(piece);
    });
    server.listen(0, () => {
      const address = server.address() as { port: number };
      modelUrl = `http://127.0.0.1:${address.port}/shift_model.onnx`;
      fileUrl = `http://127.0.0.1:${address.port}/api.ion`;
      done();
    });
  });

  afterAll((done) => {
    server.close(() => done());
  });

  it("gives one prediction per target, in order", async () => {
    const warp = await getWarp(modelPath);
    const file = await parseIon(coveringFile);
    const predicted = await warp.predictRts(file, targets);
    expect(predicted).toHaveLength(targets.length);
    for (const prediction of predicted) {
      expect(Object.keys(prediction).sort()).toEqual(["rt", "uncertainty"]);
      expect(Number.isFinite(prediction.rt)).toBe(true);
    }
    file.dispose();
    warp.dispose();
  });

  it("predicts the same from a path, bytes and a URL", async () => {
    const file = await parseIon(coveringFile);
    const fromPath = await (await getWarp(modelPath)).predictRts(file, targets);
    const fromBytes = await (
      await getWarp(new Uint8Array(fs.readFileSync(modelPath)))
    ).predictRts(file, targets);
    const fromUrl = await (await getWarp(modelUrl)).predictRts(file, targets);
    expect(fromBytes).toEqual(fromPath);
    expect(fromUrl).toEqual(fromPath);
    file.dispose();
  });

  it("predicts the same on a remote file as on a local one", async () => {
    const warp = await getWarp(modelPath);
    const local = await parseIon(coveringFile);
    const remote = await parseIon(fileUrl);
    expect(await warp.predictRts(remote, targets)).toEqual(
      await warp.predictRts(local, targets),
    );
    local.dispose();
    remote.dispose();
  });

  it("names a target without mz by its index", async () => {
    const warp = await getWarp(modelPath);
    const file = await parseIon(coveringFile);
    await expect(
      warp.predictRts(file, [targets[0], { id: "broken", mz: NaN, rt: 2 }]),
    ).rejects.toThrow("predictRts: target 2 has no valid m/z or rt");
    file.dispose();
  });

  it("refuses a file whose scans do not cover the model range", async () => {
    const warp = await getWarp(modelPath);
    const file = await parseIon(emptyFile);
    await expect(warp.predictRts(file, targets)).rejects.toThrow(
      "predictRts: the file has no scans, the warp model needs",
    );
    file.dispose();
  });

  it("refuses a model URL that answers with an HTTP error", async () => {
    await expect(
      loadModel(modelUrl.replace("shift_model.onnx", "missing.onnx")),
    ).rejects.toThrow("loadModel: the file could not be read: GET");
  });

  it("gives the keys and values stored in the model", async () => {
    const warp = await getWarp(modelPath);
    const keys = warp.getKeys();
    expect(warp.getValues()[keys.indexOf("points")]).toBe("64");
    expect(warp.getValue("points")).toBe("64");
    expect(warp.getValue("missing")).toBeUndefined();
    warp.dispose();
  });

  it("calibrates with one sample by each rt's predicted shift", async () => {
    const warp = await getWarp(modelPath);
    const file = await parseIon(coveringFile);
    const predicted = await warp.predictRts(file, targets);
    const calibrated = await warp.calibrate([file], [targets]);
    calibrated.forEach((reference, index) => {
      expect(reference.id).toBe(targets[index].id);
      expect(reference.rt).toBe(2 * targets[index].rt - predicted[index].rt);
    });
    file.dispose();
    warp.dispose();
  });

  it("calibrates with the median over the samples", async () => {
    const warp = await getWarp(modelPath);
    const file = await parseIon(coveringFile);
    const early = [{ ...targets[0], rt: targets[0].rt - 0.2 }, targets[1]];
    const late = [{ ...targets[0], rt: targets[0].rt + 0.2 }, targets[1]];
    const middle = await warp.calibrate([file], [targets]);
    expect(
      await warp.calibrate([file, file, file], [early, targets, late]),
    ).toEqual(middle);
    file.dispose();
    warp.dispose();
  });

  it("needs one target list per sample to calibrate", async () => {
    const warp = await getWarp(modelPath);
    const file = await parseIon(coveringFile);
    await expect(warp.calibrate([file], [targets, targets])).rejects.toThrow(
      "got 1 samples and 2 target lists",
    );
    file.dispose();
    warp.dispose();
  });

  it("needs an id for each target to calibrate", async () => {
    const warp = await getWarp(modelPath);
    const file = await parseIon(coveringFile);
    await expect(
      warp.calibrate([file], [[targets[0], { mz: 310.2, rt: 3.1 }]]),
    ).rejects.toThrow("calibrate: target 2 in list 1 has no id");
    file.dispose();
    warp.dispose();
  });

  it("keeps the warp working after the model is disposed", async () => {
    const model = await loadModel(modelPath);
    const warp = loadWarp(model);
    const file = await parseIon(coveringFile);
    model.dispose();
    expect(await warp.predictRts(file, targets)).toHaveLength(targets.length);
    expect(warp.getValue("points")).toBe("64");
    file.dispose();
    warp.dispose();
  });

  it("keeps the model working after the warp is made", async () => {
    const model = await loadModel(modelPath);
    loadWarp(model).dispose();
    expect(model.getValue("points")).toBe("64");
    expect(model.getKeys()).toEqual((await getWarp(modelPath)).getKeys());
    model.dispose();
  });

  it("needs a model to make a warp", () => {
    expect(() => loadWarp(modelPath as never)).toThrow(
      "loadWarp: model must be an open Model from loadModel",
    );
  });

  it("refuses a missing model file", async () => {
    await expect(
      loadModel(path.join(fixtures, "missing.onnx")),
    ).rejects.toThrow("loadModel: the file could not be read");
  });
});
