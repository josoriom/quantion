import { NodeBackend } from "./utilities/backendNode";
import { setBackend, setInitPromise } from "./utilities/api";

const backend = new NodeBackend(process);
setBackend(backend);

const _ready = Promise.resolve();
setInitPromise(_ready);

export async function init(): Promise<void> {
  await _ready;
}

export {
  parseIon,
  parseMzml,
  writeIon,
  writeMzml,
  loadModel,
  loadWarp,
  getScans,
  getScanMetadata,
  calculateEic,
  getIonImage,
  findPeaks,
  getPeak,
  fitPeak,
  drawPeaks,
  findNoiseLevel,
  calculateBaseline,
  findTargets,
  findTransitions,
  findFeatures,
  matchFeatures,
  deisotopeFeatures,
} from "./utilities/api";
export { SampleFile } from "./utilities/sampleFile";
export { Model } from "./utilities/model";
export { WarpModel } from "./utilities/warpModel";
export { QuantionError, ERROR_CODES, type ErrorKind } from "./utilities/errors";
export * from "./types/types";
