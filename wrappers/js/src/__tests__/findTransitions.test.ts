import { describe, it, expect, beforeAll } from "@jest/globals";
import * as path from "path";
import { NodeBackend } from "../utilities/backendNode";
import { setBackend, parseIon, findTransitions } from "../utilities/api";

const srmFile = path.join(__dirname, "../../../../core/tests/fixtures/srm.ion");
const alanine = 89.04768;
const serine = 90.0555;

function getNodeBackend(): NodeBackend | null {
  try {
    return new NodeBackend(process);
  } catch {
    return null;
  }
}

const nodeBackend = getNodeBackend();
const describeWithNode = nodeBackend ? describe : describe.skip;

function getTarget(precursor: number, product: number) {
  return { id: "alanine", precursor, product, rt: 2.385, range: 0.1 };
}

describeWithNode("findTransitions", () => {
  beforeAll(() => {
    setBackend(nodeBackend!);
  });

  it("finds the transition of a target", async () => {
    const [row] = findTransitions(await parseIon(srmFile), [
      getTarget(alanine, alanine),
    ]);
    expect(row.intensity).toBeGreaterThan(0);
    expect(Math.abs(row.rt - 2.385)).toBeLessThan(0.05);
  });

  it("finds nothing for a wrong product", async () => {
    const [row] = findTransitions(await parseIon(srmFile), [
      getTarget(alanine, serine),
    ]);
    expect(row.intensity).toBe(0);
  });
});
