import type { Header } from "../types/types";
import { QuantionError } from "./errors";

export async function uploadIon(
  url: string,
  headers: Header[],
  bytes: Uint8Array,
): Promise<void> {
  const response = await fetch(url, {
    method: "PUT",
    headers: headers.map(
      (header) => [header.name.trim(), header.value] as [string, string],
    ),
    body: new Blob([bytes as BlobPart]),
  }).catch((error: Error) => {
    throw new QuantionError(
      "upload_failed",
      "writeIon",
      `PUT failed: ${error.message}`,
    );
  });
  if (!response.ok) {
    throw new QuantionError(
      "upload_failed",
      "writeIon",
      `PUT returned ${response.status}`,
    );
  }
}
