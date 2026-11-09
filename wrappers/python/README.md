# quantion

Python bindings for quantion, a toolkit for LC-MS data processing.

## Install

```bash
pip install quantion
```

## Quick start

Read an mzML file, calculate the EIC of a target and find its peak.

```python
from quantion import calculate_eic, get_peak, parse_mzml

file = parse_mzml("sample.mzML")
eic = calculate_eic(file, 260.103, {"from": 2, "to": 3})
peak = get_peak(eic["x"], eic["y"], 2.383, {"range": 0.05})
print(peak)
```

## Documentation

Every function, option and example: [API Documentation](https://phenological.github.io/quantion/)
