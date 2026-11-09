import asyncifyModule from "onnxruntime-web/ort-wasm-simd-threaded.asyncify.mjs?url";
import asyncifyWasm from "onnxruntime-web/ort-wasm-simd-threaded.asyncify.wasm?url";
import plainModule from "onnxruntime-web/ort-wasm-simd-threaded.mjs?url";
import plainWasm from "onnxruntime-web/ort-wasm-simd-threaded.wasm?url";

export const runtimeFiles = {
  "ort-wasm-simd-threaded.asyncify.mjs": asyncifyModule,
  "ort-wasm-simd-threaded.asyncify.wasm": asyncifyWasm,
  "ort-wasm-simd-threaded.mjs": plainModule,
  "ort-wasm-simd-threaded.wasm": plainWasm,
};
