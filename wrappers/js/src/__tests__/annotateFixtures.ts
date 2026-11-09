import * as fs from "fs";
import * as http from "http";
import * as path from "path";

export const folder = path.join(__dirname, "../../../../core/tests/fixtures/annotate");
export const samples = ["sample_peak_and_ms2", "sample_peak_only", "sample_no_peak"];
export const features = [{ mz: 195.0877, rt: 2.0 }];
export const withoutCandidates = [
  { mz: 350.05, rt: 2.0 },
  { mz: 500.0, rt: 2.0 },
];

export interface FixtureServer {
  url: (name: string) => string;
  requests: Record<string, number>;
  close: () => Promise<void>;
}

export function startServer(): Promise<FixtureServer> {
  const requests: Record<string, number> = {};
  const server = http.createServer((request, response) => {
    const name = path.basename(request.url ?? "");
    const bytes = fs.readFileSync(path.join(folder, name));
    requests[name] = (requests[name] ?? 0) + 1;
    const headers = { "access-control-allow-origin": "*", "access-control-expose-headers": "content-range" };
    const match = (request.headers.range ?? "").match(/bytes=(\d+)-(\d+)/);
    if (!match) {
      response.writeHead(200, { ...headers, "content-length": bytes.length });
      response.end(bytes);
      return;
    }
    const first = Number(match[1]);
    const last = Math.min(Number(match[2]), bytes.length - 1);
    const piece = bytes.subarray(first, last + 1);
    response.writeHead(206, {
      ...headers,
      "content-length": piece.length,
      "content-range": `bytes ${first}-${last}/${bytes.length}`,
    });
    response.end(piece);
  });
  return new Promise((resolve) => {
    server.listen(0, () => {
      const { port } = server.address() as { port: number };
      resolve({
        url: (name) => `http://127.0.0.1:${port}/${name}`,
        requests,
        close: () => new Promise((done) => server.close(() => done())),
      });
    });
  });
}

export function getPlain(value: unknown): unknown {
  if (value instanceof Float64Array) {
    return Array.from(value);
  }
  if (Array.isArray(value)) {
    return value.map(getPlain);
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, getPlain(item)]));
  }
  return value;
}
