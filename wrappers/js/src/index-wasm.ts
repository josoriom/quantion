import { WasmBackend } from "./utilities/backendWasm";
import { setBackend, setInitPromise } from "./utilities/api";

const backend = new WasmBackend();
const _ready = backend.init().then(() => setBackend(backend));
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
  annotateFeatures,
} from "./utilities/api";
export { SampleFile } from "./utilities/sampleFile";
export { Model } from "./utilities/model";
export { WarpModel } from "./utilities/warpModel";
export { QuantionError, ERROR_CODES, type ErrorKind } from "./utilities/errors";
export * from "./types/types";
