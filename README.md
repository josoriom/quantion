# Quant·ion

<img src="assets/icon.svg" align="right" width="200" alt="Quantion icon">

[![test](https://github.com/phenological/quantion/actions/workflows/test.yml/badge.svg)](https://github.com/phenological/quantion/actions/workflows/test.yml)
[![crates.io](https://img.shields.io/crates/v/quantion)](https://crates.io/crates/quantion)
[![PyPI](https://img.shields.io/pypi/v/quantion)](https://pypi.org/project/quantion/)
[![npm](https://img.shields.io/npm/v/quantion)](https://www.npmjs.com/package/quantion)
[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.22700673.svg)](https://doi.org/10.5281/zenodo.22700673)

**Ionic Toolkit for LCMS data processing.**

A Rust core with bindings for R, Python and JavaScript.

<br clear="right">

## Install

```bash
cargo add quantion
```

## Quick start

Read an mzML file, calculate the EIC of a target and find its peak.

```rust
use quantion::{CalculateEicOptions, GetPeakOptions, calculate_eic, get_peak, parse_mzml};

let mut file = parse_mzml("sample.mzML")?;
let eic = calculate_eic(&mut file, 260.103, CalculateEicOptions { from: 2.0, to: 3.0, ..Default::default() })?;
let peak = get_peak(&eic, 2.383, GetPeakOptions { range: 0.05, ..Default::default() })?;
println!("rt {} intensity {}", peak.rt, peak.intensity);
```

## Other languages

- [R](wrappers/r/README.md)
- [Python](wrappers/python/README.md)
- [JavaScript](wrappers/js/README.md)

## Documentation

Every function, option and example: [API Documentation](https://phenological.github.io/quantion/)
