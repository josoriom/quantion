import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import path from "node:path";

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const quantion = require(path.join(root, "wrappers/js/lib/index-node.js"));

const FROM_RT = 0.0;
const TO_RT = 5.0;
const EIC_PPM = 20.0;
const EIC_MZ = 0.005;
const GRID_START = 89.0;
const GRID_END = 91.0;
const GRID_STEP = 0.005;
const ROI_RANGE = 0.05;
const THREADS = 1;

const FEATURE_MIN_INTENSITY = 500.0;
const FEATURE_MIN_PEAK_WIDTH_POINTS = 5;

const SELF_CHECK_VALUES = [1.0, 2.0, 3.0];

const CHANNELS = [
  {
    name: "mz_2",
    mass: 90.0555,
    minIntensity: 500.0,
    minPeakWidthPoints: 5,
    targets: [
      ["met_1", 2.885],
      ["met_2", 2.552],
      ["met_3", 2.465],
    ],
  },
  {
    name: "mz_1",
    mass: 89.04768,
    minIntensity: 500.0,
    minPeakWidthPoints: 5,
    targets: [
      ["met_1", 2.385],
      ["met_2", 2.18],
      ["met_3", 2.08],
    ],
  },
];

function number(value) {
  const asNumber = Number(value);
  if (Number.isNaN(asNumber)) return "nan";
  if (asNumber === Infinity) return "inf";
  if (asNumber === -Infinity) return "-inf";
  return String(asNumber);
}

function digest(values) {
  let hash = 0x811c9dc5;
  const view = new DataView(new ArrayBuffer(8));
  for (const value of values) {
    view.setFloat64(0, Number(value), true);
    for (let byte = 0; byte < 8; byte += 1) {
      hash ^= view.getUint8(byte);
      hash = Math.imul(hash, 0x01000193) >>> 0;
    }
  }
  return hash.toString(16).padStart(8, "0");
}

function eicOptions() {
  return {
    from: FROM_RT,
    to: TO_RT,
    ppmTolerance: EIC_PPM,
    mzTolerance: EIC_MZ,
  };
}

function peakOptions(channel) {
  return {
    minIntensity: channel.minIntensity,
    minPeakWidthPoints: channel.minPeakWidthPoints,
    autoNoise: true,
    autoBaseline: true,
  };
}

function chromPeakOptions(channel) {
  return {
    minIntensity: channel.minIntensity,
    minPeakWidthPoints: channel.minPeakWidthPoints,
    autoNoise: false,
    autoBaseline: false,
  };
}

function featurePeakOptions() {
  return {
    minIntensity: FEATURE_MIN_INTENSITY,
    minPeakWidthPoints: FEATURE_MIN_PEAK_WIDTH_POINTS,
    autoNoise: true,
    autoBaseline: true,
  };
}

function sharedPeakLines(prefix, peak, out) {
  out.push(`${prefix}.rt = ${number(peak.rt)}`);
  out.push(`${prefix}.from = ${number(peak.from)}`);
  out.push(`${prefix}.to = ${number(peak.to)}`);
  out.push(`${prefix}.integral = ${number(peak.integral)}`);
  out.push(`${prefix}.intensity = ${number(peak.intensity)}`);
}

function peakLines(prefix, peak, out) {
  sharedPeakLines(prefix, peak, out);
  out.push(`${prefix}.n_points = ${peak.nPoints}`);
}

async function reportChannel(file, transitions, channel, out) {
  const name = channel.name;
  const eic = await quantion.calculateEic(file, channel.mass, eicOptions());
  const x = eic.x;
  const y = eic.y;

  out.push(`${name}.calculate_eic.points = ${x.length}`);
  out.push(`${name}.calculate_eic.x.fnv1a = ${digest(x)}`);
  out.push(`${name}.calculate_eic.y.fnv1a = ${digest(y)}`);
  const noise = quantion.findNoiseLevel(y);
  out.push(`${name}.find_noise_level.intensity = ${number(noise.intensity)}`);
  out.push(`${name}.find_noise_level.width = ${noise.width}`);

  const found = quantion.findPeaks(x, y, peakOptions(channel));
  out.push(`${name}.find_peaks.count = ${found.length}`);
  found.forEach((peak, index) =>
    peakLines(`${name}.find_peaks[${index}]`, peak, out),
  );

  const target = channel.targets[2][1];
  const single = quantion.getPeak(x, y, target, {
    range: ROI_RANGE,
    findPeaks: peakOptions(channel),
  });
  peakLines(`${name}.get_peak`, single, out);

  const targets = channel.targets.map(([id, rt]) => ({
    id,
    rt,
    mz: channel.mass,
    range: ROI_RANGE,
  }));
  const rows = await quantion.findTargets(file, targets, {
    eic: eicOptions(),
    findPeaks: peakOptions(channel),
    threads: THREADS,
  });
  rows.forEach((row, position) =>
    sharedPeakLines(`${name}.find_targets.${targets[position].id}`, row, out),
  );

  const chromTargets = channel.targets.map(([id, rt]) => ({
    id,
    precursor: channel.mass,
    product: channel.mass,
    rt,
    range: ROI_RANGE,
  }));
  const chromRows = quantion.findTransitions(transitions, chromTargets, {
    findPeaks: chromPeakOptions(channel),
    threads: THREADS,
  });
  chromRows.forEach((row, position) => {
    const prefix = `${name}.find_transitions.${channel.targets[position][0]}`;
    out.push(`${prefix}.rt = ${number(row.rt)}`);
    out.push(`${prefix}.from = ${number(row.from)}`);
    out.push(`${prefix}.to = ${number(row.to)}`);
    out.push(`${prefix}.intensity = ${number(row.intensity)}`);
    out.push(`${prefix}.area = ${number(row.integral)}`);
  });
}

function featureLines(label, rows, out, withPoints) {
  out.push(`${label}.count = ${rows.length}`);
  rows.forEach((row, index) => {
    const name = `${label}[${index}]`;
    out.push(`${name}.mz = ${number(row.mz)}`);
    out.push(`${name}.rt = ${number(row.rt)}`);
    out.push(`${name}.from = ${number(row.from)}`);
    out.push(`${name}.to = ${number(row.to)}`);
    out.push(`${name}.integral = ${number(row.integral)}`);
    out.push(`${name}.intensity = ${number(row.intensity)}`);
    if (withPoints) {
      out.push(`${name}.n_points = ${row.nPoints}`);
    }
  });
}

async function main() {
  const directory = process.argv[2] ?? "core/tests/fixtures/api";
  const fixture = path.join(directory, "api.ion");

  const file = await quantion.parseIon(readFileSync(fixture));
  const transitions = await quantion.parseIon(
    readFileSync(path.join(path.dirname(directory), "srm.ion")),
  );
  const out = [];
  out.push(`fnv1a.self_check = ${digest(SELF_CHECK_VALUES)}`);

  const scans = await quantion.getScans(file, {
    rt: { from: FROM_RT, to: TO_RT },
  });
  out.push(`parse_ion.scans = ${scans.length}`);
  const lastIndex = scans[scans.length - 1].index;
  out.push(`get_scans.first.index = ${scans[0].index}`);
  out.push(`get_scans.last.index = ${lastIndex}`);
  const rows = await quantion.getScanMetadata(file, lastIndex);
  out.push(`get_scan_metadata.count = ${rows.length}`);
  rows.forEach((row, position) => {
    const fields = [row.path, row.kind, row.accession, row.name, row.value, row.unit, row.unitAccession];
    out.push(`get_scan_metadata.${position} = ${fields.map((field) => field ?? "NA").join(" | ")}`);
  });
  const scanRows = await quantion.getScanMetadata(file, lastIndex, { tags: ["scan"] });
  out.push(`get_scan_metadata.scan.count = ${scanRows.length}`);

  for (const channel of CHANNELS) {
    await reportChannel(file, transitions, channel, out);
  }

  const grid = { start: GRID_START, end: GRID_END, stepSize: GRID_STEP };
  const featureOptions = {
    eic: eicOptions(),
    grid,
    findPeaks: featurePeakOptions(),
    threads: THREADS,
  };

  const found = await quantion.findFeatures(file, featureOptions);
  featureLines("find_features", found, out, true);

  const consensus = quantion.matchFeatures(directory, featureOptions);
  featureLines("match_features", consensus, out, false);
  featureLines("deisotope_features", quantion.deisotopeFeatures(consensus), out, false);

  const warp = quantion.loadWarp(await quantion.loadModel(path.join(directory, "..", "shift_model.onnx")));
  const targets = CHANNELS.flatMap((channel) =>
    channel.targets.map(([id, rt]) => ({ id: `${channel.name}.${id}`, mz: channel.mass, rt })),
  );
  const predicted = await warp.predictRts(file, targets);
  out.push(`predict_rts.count = ${predicted.length}`);
  targets.forEach((target, index) => {
    out.push(`predict_rts.${target.id}.rt = ${number(predicted[index].rt)}`);
    out.push(`predict_rts.${target.id}.uncertainty = ${number(predicted[index].uncertainty)}`);
  });

  process.stdout.write(out.join("\n") + "\n");
}

main().catch((error) => {
  process.stderr.write(String(error?.stack ?? error) + "\n");
  process.exit(1);
});
