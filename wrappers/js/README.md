# quantion

JavaScript bindings for quantion, a toolkit for LC-MS data processing. Works in Node and in the browser.

## Install

```bash
npm install quantion
```

## Quick start

Read an mzML file, calculate the EIC of a target and find its peak.

```js
import { calculateEic, getPeak, parseMzml } from "quantion";

const file = await parseMzml("sample.mzML");
const eic = await calculateEic(file, 260.103, { from: 2, to: 3 });
const peak = getPeak(eic.x, eic.y, 2.383, { range: 0.05 });
console.log(peak);
```

## Documentation

Every function, option and example: [API Documentation](https://phenological.github.io/quantion/)
