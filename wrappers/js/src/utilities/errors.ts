const FIELD_SEPARATOR = "\u001f";

export const ERROR_CODES = {
  invalid_argument: 1,
  missing_argument: 2,
  unknown_option: 3,
  remote_file: 4,
  file_not_found: 5,
  parse_failed: 6,
  model_mismatch: 7,
  file_exists: 8,
  upload_failed: 9,
  internal: 99,
} as const;

/**
 * ErrorKind · Errors
 *
 * What went wrong, as a name and a fixed number code.
 *
 * Fields:
 * - `"invalid_argument"`: A value is out of range or the wrong type.
 * - `"missing_argument"`: A required value is missing.
 * - `"unknown_option"`: An option key that does not exist.
 * - `"remote_file"`: Needs the whole file, but a URL was given.
 * - `"file_not_found"`: The path does not exist.
 * - `"parse_failed"`: The file is not a valid Ionic (.ion), mzML or ONNX file.
 * - `"model_mismatch"`: The model has no warp, or a target is not in it.
 * - `"file_exists"`: The output file exists and overwrite is false.
 * - `"upload_failed"`: The upload was rejected, with the HTTP status.
 * - `"internal"`: A bug in quantion. Please report it.
 * @see {@link https://phenological.github.io/quantion/?language=js#ErrorKind | Documentation}
 */
export type ErrorKind = keyof typeof ERROR_CODES;

/**
 * QuantionError · Errors
 *
 * The error every function raises, with its kind, code, function, argument and message.
 *
 * Fields:
 * - `kind`: What went wrong.
 * - `code`: Fixed number for the kind.
 * - `function`: Where: the public function that failed.
 * - `argument`: Which input, such as targets[3].range.
 * - `message`: Why, in plain words. Starts with the function name.
 * @see {@link https://phenological.github.io/quantion/?language=js#QuantionError | Documentation}
 */
export class QuantionError extends Error {
  readonly kind: ErrorKind;
  readonly code: number;
  readonly function: string;
  readonly argument: string | undefined;

  constructor(
    kind: ErrorKind,
    functionName: string,
    message: string,
    argument?: string,
  ) {
    super(`${functionName}: ${message}`);
    this.name = "QuantionError";
    this.kind = kind;
    this.code = ERROR_CODES[kind] ?? ERROR_CODES.internal;
    this.function = functionName;
    this.argument = argument || undefined;
  }
}

export function readNativeError(text: string): QuantionError {
  const [code, kind, functionName, argument, message] =
    text.split(FIELD_SEPARATOR);
  if (message === undefined) {
    return new QuantionError(
      "internal",
      "quantion",
      `the call failed with code ${code} (this is a bug in quantion, please report it)`,
    );
  }
  const knownKind = (kind in ERROR_CODES ? kind : "internal") as ErrorKind;
  const jsArgument = toJsName(argument);
  let jsMessage = message;
  if (argument) jsMessage = jsMessage.split(argument).join(jsArgument);
  for (const name of FUNCTION_NAMES) {
    jsMessage = jsMessage.replace(
      new RegExp(`\\b${name}\\b`, "g"),
      toJsName(name),
    );
  }
  return new QuantionError(
    knownKind,
    toJsName(functionName),
    jsMessage,
    jsArgument,
  );
}

const FUNCTION_NAMES = [
  "parse_ion",
  "parse_mzml",
  "write_ion",
  "write_mzml",
  "get_scans",
  "get_scan_metadata",
  "calculate_eic",
  "get_ion_image",
  "find_peaks",
  "get_peak",
  "fit_peak",
  "draw_peaks",
  "find_noise_level",
  "calculate_baseline",
  "find_targets",
  "find_transitions",
  "find_features",
  "match_features",
  "deisotope_features",
  "load_model",
  "load_warp",
  "predict_rts",
  "calibrate",
];

function toJsName(name: string): string {
  return name.replace(/_([a-z0-9])/g, (_, letter: string) =>
    letter.toUpperCase(),
  );
}

export function missingArgument(
  functionName: string,
  argument: string,
): QuantionError {
  return new QuantionError(
    "missing_argument",
    functionName,
    `${argument} is required`,
    argument,
  );
}

export function invalidArgument(
  functionName: string,
  argument: string,
  message: string,
): QuantionError {
  return new QuantionError(
    "invalid_argument",
    functionName,
    `${argument} ${message}`,
    argument,
  );
}

export function toQuantionError(error: unknown): unknown {
  const text = (error as { quantion?: unknown } | null)?.quantion;
  return typeof text === "string" ? readNativeError(text) : error;
}
