export function encodeTargetIds(ids: string[]): {
  packedBytes: Uint8Array;
  offsets: Uint32Array;
  lengths: Uint32Array;
} {
  const encoder = new TextEncoder();
  const encoded = ids.map((id) => encoder.encode(id));
  const totalBytes = encoded.reduce((sum, e) => sum + e.length, 0);
  const packedBytes = new Uint8Array(totalBytes);
  const offsets = new Uint32Array(ids.length);
  const lengths = new Uint32Array(ids.length);
  let cursor = 0;
  for (let i = 0; i < ids.length; i++) {
    offsets[i] = cursor >>> 0;
    lengths[i] = encoded[i].length >>> 0;
    packedBytes.set(encoded[i], cursor);
    cursor += encoded[i].length;
  }
  return { packedBytes, offsets, lengths };
}
