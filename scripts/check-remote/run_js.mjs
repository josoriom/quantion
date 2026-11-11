import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import path from "node:path";
import fs from "node:fs";

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const quantion = require(path.join(root, "wrappers/js/lib/index-node.js"));

const MASS = 90.0555;
const FROM_RT = 0.0;
const TO_RT = 5.0;
const EIC_PPM = 20.0;
const EIC_MZ = 0.005;

function sameBits(left, right) {
  if (left.length !== right.length) return false;
  const view = new DataView(new ArrayBuffer(16));
  for (let index = 0; index < left.length; index += 1) {
    view.setFloat64(0, left[index]);
    view.setFloat64(8, right[index]);
    if (view.getBigUint64(0) !== view.getBigUint64(8)) return false;
  }
  return true;
}

async function stats(url) {
  const base = new URL(url);
  const response = await fetch(`${base.origin}/stats`);
  return response.json();
}

async function reset(url) {
  const base = new URL(url);
  await fetch(`${base.origin}/reset`);
}

async function main() {
  const url = process.env.QUANTION_REMOTE_URL;
  const fixture = path.join(root, "core/tests/fixtures/api/api.ion");
  if (!url) {
    process.stderr.write("QUANTION_REMOTE_URL is not set\n");
    return 1;
  }

  await reset(url);
  const remote = await quantion.parseIon(new URL(url));
  const opening = await stats(url);

  await reset(url);
  const local = await quantion.parseIon(fixture);

  const eic = { from: FROM_RT, to: TO_RT, ppmTolerance: EIC_PPM, mzTolerance: EIC_MZ };
  const eicRemote = await quantion.calculateEic(remote, MASS, eic);
  const eicLocal = await quantion.calculateEic(local, MASS, eic);
  const oneQuery = await stats(url);

  const total = fs.statSync(fixture).size;
  const same =
    sameBits(eicRemote.x, eicLocal.x) && sameBits(eicRemote.y, eicLocal.y);

  console.log(`file_bytes = ${total}`);
  console.log(`opening_bytes = ${opening.bytes_sent}`);
  console.log(`opening_requests = ${opening.requests}`);
  console.log(`query_bytes = ${oneQuery.bytes_sent}`);
  console.log(`query_requests = ${oneQuery.requests}`);
  console.log(`matches_local = ${same ? "yes" : "no"}`);

  const model = path.join(root, "core/tests/fixtures/shift_model.onnx");
  const warp = quantion.loadWarp(await quantion.loadModel(model));
  const target = { id: "a", mz: 89.04768, rt: 2.385, range: 0.1 };
  const window = { from: FROM_RT, to: TO_RT };
  const calls = {
    calculate_eic: (file) => quantion.calculateEic(file, MASS, window),
    get_scans: (file) => quantion.getScans(file, { rt: { from: 2.3, to: 2.5 } }),
    find_targets: (file) => quantion.findTargets(file, [target], { eic: window }),
    find_transitions: (file) => quantion.findTransitions(file, [{ id: "a", precursor: 89.04768, product: 89.04768, rt: 2.385, range: 0.1 }]),
    find_features: (file) => quantion.findFeatures(file, { eic: window }),
    write_ion: (file) => quantion.writeIon(file),
    write_mzml: (file) => quantion.writeMzml(file),
    predict_rts: (file) => warp.predictRts(file, [{ mz: 250.1, rt: 1.2 }]),
  };
  for (const [name, call] of Object.entries(calls)) {
    console.log(`fresh_${name} = ${await compare(call, url, fixture)}`);
  }
  console.log(`upload = ${await checkUpload(url, local, eic, eicLocal)}`);
  return 0;
}

async function checkUpload(url, local, eic, eicLocal) {
  const target = url.slice(0, url.lastIndexOf("/")) + "/uploaded.ion";
  try {
    await quantion.writeIon(local, {
      out: target,
      headers: [{ name: "x-check", value: "upload" }],
    });
    const uploaded = await quantion.calculateEic(
      await quantion.parseIon(target),
      MASS,
      eic,
    );
    const same =
      sameBits(uploaded.x, eicLocal.x) && sameBits(uploaded.y, eicLocal.y);
    return same ? "same" : "different";
  } catch (error) {
    return `error: ${error.message}`;
  }
}

function toText(value) {
  return JSON.stringify(value, (_, item) =>
    ArrayBuffer.isView(item) ? Array.from(item) : typeof item === "bigint" ? String(item) : item,
  );
}

async function compare(call, url, fixture) {
  let remote;
  try {
    remote = await call(await quantion.parseIon(new URL(url)));
  } catch (error) {
    if (error instanceof quantion.QuantionError && error.kind === "remote_file") return "refused";
    return "error " + String(error?.message ?? error).split("\n")[0].slice(0, 80);
  }
  const local = await call(await quantion.parseIon(fixture));
  return toText(remote) === toText(local) ? "same" : "differs";
}

main().then((code) => process.exit(code)).catch((error) => {
  process.stderr.write(String(error?.stack ?? error) + "\n");
  process.exit(1);
});
