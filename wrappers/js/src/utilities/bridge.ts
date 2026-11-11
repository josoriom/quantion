export const BRIDGE_MAGIC = 0x42544e51;
export const BRIDGE_LAYOUT_VERSION = 1;
export const BRIDGE_HEADER_BYTES = 32;
export const SECTION_ENTRY_BYTES = 24;

export const PAYLOAD_SCANS = 1;
export const PAYLOAD_ION_IMAGE = 2;

export const PAYLOAD_PEAKS = 3;
export const PAYLOAD_FEATURES = 4;
export const PAYLOAD_FIT_RESULT = 6;
export const PAYLOAD_CONSENSUS_FEATURES = 8;
export const PAYLOAD_WARP_TARGETS = 10;
export const PAYLOAD_MODEL_METADATA = 11;
export const PAYLOAD_SCAN_METADATA = 12;
export const PAYLOAD_FEATURE_ANNOTATIONS = 13;
export const PAYLOAD_FILE_ANNOTATIONS = 14;
export const PAYLOAD_COMPOUND_MATCHES = 15;

export const ELEMENT_F64 = 1;
export const ELEMENT_U32 = 2;
export const ELEMENT_U64 = 3;
export const ELEMENT_U8 = 4;

export const SECTION_POINT_STARTS = 1;
export const SECTION_MZ = 2;
export const SECTION_INTENSITY = 3;
export const SECTION_RT = 4;
export const SECTION_RT_SECONDS = 5;
export const SECTION_BASE_PEAK_MZ = 6;
export const SECTION_SELECTED_ION_MZ = 7;
export const SECTION_BASE_PEAK_INT = 8;
export const SECTION_TOTAL_ION_CURRENT = 9;
export const SECTION_MS_LEVEL = 10;
export const SECTION_POLARITY = 11;
export const SECTION_POSITION_X = 12;
export const SECTION_POSITION_Y = 13;
export const SECTION_POSITION_Z = 14;
export const SECTION_IMAGE_SHAPE = 15;
export const SECTION_IMAGE_DATA = 16;
export const SECTION_IMAGE_COUNTS = 17;
export const SECTION_SCAN_ARRAY_STARTS = 88;
export const SECTION_SCAN_ARRAY_VALUES = 89;
export const SECTION_SCAN_INDEX = 90;

export const SECTION_PEAK_FROM = 18;
export const SECTION_PEAK_TO = 19;
export const SECTION_PEAK_RT = 20;
export const SECTION_PEAK_INTEGRAL = 21;
export const SECTION_PEAK_INTENSITY = 22;
export const SECTION_PEAK_POINT_COUNT = 23;

export const SECTION_FEATURE_MZ = 26;
export const SECTION_FEATURE_RT = 27;
export const SECTION_FEATURE_FROM = 28;
export const SECTION_FEATURE_TO = 29;
export const SECTION_FEATURE_INTENSITY = 30;
export const SECTION_FEATURE_INTEGRAL = 31;
export const SECTION_FEATURE_POINT_COUNT = 32;

export const SECTION_FIT_SHAPE = 46;
export const SECTION_FIT_HEIGHT = 47;
export const SECTION_FIT_CENTER = 48;
export const SECTION_FIT_FWHM = 49;
export const SECTION_FIT_TAIL = 50;
export const SECTION_FIT_R2 = 51;

export const SECTION_CONSENSUS_MZ = 62;
export const SECTION_CONSENSUS_RT = 63;
export const SECTION_CONSENSUS_FROM = 64;
export const SECTION_CONSENSUS_TO = 65;
export const SECTION_CONSENSUS_INTENSITY = 66;
export const SECTION_CONSENSUS_INTEGRAL = 67;
export const SECTION_CONSENSUS_FREQUENCY = 68;
export const SECTION_CONSENSUS_POINT_COUNT = 87;

export const SECTION_WARP_TARGET_ID_STARTS = 79;
export const SECTION_WARP_TARGET_ID_BYTES = 80;
export const SECTION_WARP_TARGET_MZ = 81;
export const SECTION_WARP_TARGET_RT = 82;

export const SECTION_MODEL_METADATA_KEY_STARTS = 83;
export const SECTION_MODEL_METADATA_KEY_BYTES = 84;
export const SECTION_MODEL_METADATA_VALUE_STARTS = 85;
export const SECTION_MODEL_METADATA_VALUE_BYTES = 86;

export const SECTION_SCAN_METADATA_PATH_STARTS = 91;
export const SECTION_SCAN_METADATA_PATH_BYTES = 92;
export const SECTION_SCAN_METADATA_KIND_STARTS = 93;
export const SECTION_SCAN_METADATA_KIND_BYTES = 94;
export const SECTION_SCAN_METADATA_ACCESSION_STARTS = 95;
export const SECTION_SCAN_METADATA_ACCESSION_BYTES = 96;
export const SECTION_SCAN_METADATA_ACCESSION_MISSING = 97;
export const SECTION_SCAN_METADATA_NAME_STARTS = 98;
export const SECTION_SCAN_METADATA_NAME_BYTES = 99;
export const SECTION_SCAN_METADATA_VALUE_STARTS = 100;
export const SECTION_SCAN_METADATA_VALUE_BYTES = 101;
export const SECTION_SCAN_METADATA_VALUE_MISSING = 102;
export const SECTION_SCAN_METADATA_UNIT_STARTS = 103;
export const SECTION_SCAN_METADATA_UNIT_BYTES = 104;
export const SECTION_SCAN_METADATA_UNIT_MISSING = 105;
export const SECTION_SCAN_METADATA_UNIT_ACCESSION_STARTS = 106;
export const SECTION_SCAN_METADATA_UNIT_ACCESSION_BYTES = 107;
export const SECTION_SCAN_METADATA_UNIT_ACCESSION_MISSING = 108;
export const SECTION_FEATURE_ANNOTATION_MZ = 109;
export const SECTION_FEATURE_ANNOTATION_RT = 110;
export const SECTION_FEATURE_ANNOTATION_LABEL_STARTS = 111;
export const SECTION_FEATURE_ANNOTATION_LABEL_BYTES = 112;
export const SECTION_FEATURE_ANNOTATION_LABEL_MISSING = 113;
export const SECTION_FEATURE_ANNOTATION_ID_STARTS = 114;
export const SECTION_FEATURE_ANNOTATION_ID_BYTES = 115;
export const SECTION_FEATURE_ANNOTATION_ID_MISSING = 116;
export const SECTION_FEATURE_ANNOTATION_DATABASE_STARTS = 117;
export const SECTION_FEATURE_ANNOTATION_DATABASE_BYTES = 118;
export const SECTION_FEATURE_ANNOTATION_DATABASE_MISSING = 119;
export const SECTION_FEATURE_ANNOTATION_FREQUENCY = 120;
export const SECTION_FEATURE_ANNOTATION_SCORE = 121;
export const SECTION_FEATURE_ANNOTATION_COSINE = 122;
export const SECTION_FILE_ANNOTATION_FEATURE = 123;
export const SECTION_FILE_ANNOTATION_FILE = 124;
export const SECTION_FILE_ANNOTATION_SCAN_INDEX = 125;
export const SECTION_COMPOUND_MATCH_FEATURE = 126;
export const SECTION_COMPOUND_MATCH_FILE = 127;
export const SECTION_COMPOUND_MATCH_LABEL_STARTS = 128;
export const SECTION_COMPOUND_MATCH_LABEL_BYTES = 129;
export const SECTION_COMPOUND_MATCH_ID_STARTS = 130;
export const SECTION_COMPOUND_MATCH_ID_BYTES = 131;
export const SECTION_COMPOUND_MATCH_DATABASE_STARTS = 132;
export const SECTION_COMPOUND_MATCH_DATABASE_BYTES = 133;
export const SECTION_COMPOUND_MATCH_SCORE = 134;
export const SECTION_COMPOUND_MATCH_COSINE = 135;
export const SECTION_COMPOUND_MATCH_MZ_STARTS = 136;
export const SECTION_COMPOUND_MATCH_MZ_VALUES = 137;
export const SECTION_COMPOUND_MATCH_INTENSITY_STARTS = 138;
export const SECTION_COMPOUND_MATCH_INTENSITY_VALUES = 139;
export const SECTION_COMPOUND_MATCH_METADATA_NAME_STARTS = 140;
export const SECTION_COMPOUND_MATCH_METADATA_NAME_BYTES = 141;
export const SECTION_COMPOUND_MATCH_METADATA_VALUE_STARTS = 142;
export const SECTION_COMPOUND_MATCH_METADATA_VALUE_BYTES = 143;

const ELEMENT_SIZES: Record<number, number> = {
  [ELEMENT_F64]: 8,
  [ELEMENT_U32]: 4,
  [ELEMENT_U64]: 8,
  [ELEMENT_U8]: 1,
};

const SECTION_TYPES: Record<number, number> = {
  [SECTION_POINT_STARTS]: ELEMENT_U64,
  [SECTION_MZ]: ELEMENT_F64,
  [SECTION_INTENSITY]: ELEMENT_F64,
  [SECTION_RT]: ELEMENT_F64,
  [SECTION_RT_SECONDS]: ELEMENT_F64,
  [SECTION_BASE_PEAK_MZ]: ELEMENT_F64,
  [SECTION_SELECTED_ION_MZ]: ELEMENT_F64,
  [SECTION_BASE_PEAK_INT]: ELEMENT_F64,
  [SECTION_TOTAL_ION_CURRENT]: ELEMENT_F64,
  [SECTION_MS_LEVEL]: ELEMENT_F64,
  [SECTION_POLARITY]: ELEMENT_F64,
  [SECTION_POSITION_X]: ELEMENT_F64,
  [SECTION_POSITION_Y]: ELEMENT_F64,
  [SECTION_POSITION_Z]: ELEMENT_F64,
  [SECTION_IMAGE_SHAPE]: ELEMENT_U32,
  [SECTION_IMAGE_DATA]: ELEMENT_F64,
  [SECTION_IMAGE_COUNTS]: ELEMENT_U32,
  [SECTION_SCAN_ARRAY_STARTS]: ELEMENT_U64,
  [SECTION_SCAN_ARRAY_VALUES]: ELEMENT_F64,
  [SECTION_SCAN_INDEX]: ELEMENT_F64,
  [SECTION_PEAK_POINT_COUNT]: ELEMENT_U32,
  [SECTION_FEATURE_POINT_COUNT]: ELEMENT_U32,
  [SECTION_CONSENSUS_POINT_COUNT]: ELEMENT_U32,
};

export interface Section {
  elementType: number;
  byteOffset: number;
  byteLength: number;
}

export interface Bridge {
  buffer: ArrayBuffer;
  payloadKind: number;
  recordCount: number;
  sections: Map<number, Section>;
}

function fail(reason: string): never {
  throw new Error(`quantion bridge: ${reason}`);
}

function toNumber(value: bigint, what: string): number {
  if (value > BigInt(Number.MAX_SAFE_INTEGER))
    fail(`${what} is too large to read`);
  return Number(value);
}

export function readBridge(buffer: ArrayBuffer): Bridge {
  if (buffer.byteLength < BRIDGE_HEADER_BYTES)
    fail("buffer is shorter than the header");
  const view = new DataView(buffer);

  if (view.getUint32(0, true) !== BRIDGE_MAGIC) fail("magic does not match");
  const layoutVersion = view.getUint16(4, true);
  if (layoutVersion !== BRIDGE_LAYOUT_VERSION)
    fail(`layout version ${layoutVersion} is not supported`);

  const payloadKind = view.getUint16(6, true);
  const sectionCount = view.getUint32(8, true);
  const tableOffset = view.getUint32(12, true);
  const totalBytes = view.getBigUint64(16, true);
  const recordCount = view.getBigUint64(24, true);

  if (totalBytes !== BigInt(buffer.byteLength))
    fail("total bytes does not match the buffer");
  if (tableOffset !== BRIDGE_HEADER_BYTES)
    fail("section table does not start after the header");

  const total = buffer.byteLength;
  if (sectionCount * SECTION_ENTRY_BYTES > total - BRIDGE_HEADER_BYTES)
    fail("section table does not fit");

  const sections = new Map<number, Section>();
  let reach = BigInt(BRIDGE_HEADER_BYTES + sectionCount * SECTION_ENTRY_BYTES);
  const totalBig = BigInt(total);

  for (let index = 0; index < sectionCount; index += 1) {
    const start = BRIDGE_HEADER_BYTES + index * SECTION_ENTRY_BYTES;
    const id = view.getUint32(start, true);
    const elementType = view.getUint32(start + 4, true);
    const byteOffset = view.getBigUint64(start + 8, true);
    const byteLength = view.getBigUint64(start + 16, true);

    if (sections.has(id)) fail(`section ${id} appears twice`);
    if (byteOffset > totalBig) fail(`section ${id} starts past the end`);
    if (byteLength > totalBig - byteOffset)
      fail(`section ${id} runs past the end`);
    if (byteOffset % 8n !== 0n)
      fail(`section ${id} is not aligned to eight bytes`);
    if (byteOffset < reach)
      fail(`section ${id} overlaps the section before it`);

    const elementSize = ELEMENT_SIZES[elementType];
    if (elementSize === undefined)
      fail(`section ${id} has an unknown element type`);
    if (byteLength % BigInt(elementSize) !== 0n)
      fail(`section ${id} is not a whole number of elements`);

    const expected = SECTION_TYPES[id];
    if (expected !== undefined && expected !== elementType)
      fail(`section ${id} has the wrong element type`);

    reach = byteOffset + byteLength;
    sections.set(id, {
      elementType,
      byteOffset: toNumber(byteOffset, `section ${id} offset`),
      byteLength: toNumber(byteLength, `section ${id} length`),
    });
  }

  return {
    buffer,
    payloadKind,
    recordCount: toNumber(recordCount, "record count"),
    sections,
  };
}

export function expectKind(bridge: Bridge, wanted: number): void {
  if (bridge.payloadKind !== wanted) {
    fail(`expected payload kind ${wanted} but found ${bridge.payloadKind}`);
  }
}

function takeSection(bridge: Bridge, id: number): Section {
  const section = bridge.sections.get(id);
  if (!section) fail(`section ${id} is missing`);
  return section;
}

export function readF64Section(bridge: Bridge, id: number): Float64Array {
  const section = takeSection(bridge, id);
  return new Float64Array(
    bridge.buffer,
    section.byteOffset,
    section.byteLength / 8,
  );
}

export function readU32Section(bridge: Bridge, id: number): Uint32Array {
  const section = takeSection(bridge, id);
  return new Uint32Array(
    bridge.buffer,
    section.byteOffset,
    section.byteLength / 4,
  );
}

export function readCountSection(bridge: Bridge, id: number): Float64Array {
  const section = takeSection(bridge, id);
  const count = section.byteLength / 8;
  const view = new DataView(bridge.buffer);
  const counts = new Float64Array(count);
  let previous = 0;
  for (let index = 0; index < count; index += 1) {
    const value = toNumber(
      view.getBigUint64(section.byteOffset + index * 8, true),
      `section ${id} entry`,
    );
    if (value < previous) fail(`section ${id} does not increase`);
    counts[index] = value;
    previous = value;
  }
  return counts;
}

export function readU8Section(bridge: Bridge, id: number): Uint8Array {
  const section = takeSection(bridge, id);
  return new Uint8Array(bridge.buffer, section.byteOffset, section.byteLength);
}
