# quantion

R bindings for quantion, a toolkit for LC-MS data processing.

## Install

```r
remotes::install_github("phenological/quantion", subdir = "wrappers/r")
```

The install builds the native library from its Rust source, so it needs `cargo` and `rustc` (1.88 or newer).

## Quick start

Read an mzML file, calculate the EIC of a target and find its peak.

```r
library(quantion)

sample <- parse_mzml("sample.mzML")
eic <- calculate_eic(sample, 260.103, options = calculate_eic_options(from = 2, to = 3))
peak <- get_peak(eic$x, eic$y, rt = 2.383, options = get_peak_options(range = 0.05))
print(peak)
```

## Documentation

Every function, option and example: [API Documentation](https://phenological.github.io/quantion/)
