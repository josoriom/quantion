sample_file <- sample_ion_path()

test_that("an rt range gives every scan inside it", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  scans <- get_scans(parse_ion(sample_file), rt = c(2.3, 2.5))
  expect_gt(nrow(scans), 0)
  expect_true(all(scans$rt >= 2.3 & scans$rt <= 2.5))
})

test_that("one rt gives the closest scan", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  scans <- get_scans(parse_ion(sample_file), rt = 2.385)
  expect_equal(nrow(scans), 1)
  expect_equal(scans$rt, 2.385, tolerance = 0.05)
  expect_true(all(c("level", "precursor_mz", "base_peak_intensity", "x", "y", "z") %in% names(scans)))
})

test_that("a query with neither rt nor mz is refused", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  expect_error(get_scans(parse_ion(sample_file)), "rt or mz must be given")
})

test_that("three numbers are refused", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  expect_error(get_scans(parse_ion(sample_file), rt = c(1, 2, 3)), "get_scans: rt must be one number or c\\(from, to\\)")
})

encode_base64 <- function(bytes) {
  letters_used <- strsplit("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/", "")[[1]]
  values <- as.integer(bytes)
  pieces <- character()
  for (start in seq(1, length(values), by = 3)) {
    group <- values[start:min(start + 2, length(values))]
    filled <- c(group, rep(0L, 3 - length(group)))
    number <- filled[1] * 65536 + filled[2] * 256 + filled[3]
    digits <- c(number %/% 262144, (number %/% 4096) %% 64, (number %/% 64) %% 64, number %% 64)
    chars <- letters_used[digits + 1]
    if (length(group) < 3) chars[(length(group) + 2):4] <- "="
    pieces <- c(pieces, paste(chars, collapse = ""))
  }
  paste(pieces, collapse = "")
}

get_fingerprint <- function(position) (seq(0, 255) - 128) / 256 + position

write_array <- function(accession, name, value, values) {
  encoded <- encode_base64(writeBin(as.numeric(values), raw(), size = 8, endian = "little"))
  sprintf(paste0('<binaryDataArray arrayLength="%d" encodedLength="%d">',
                 '<cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/>',
                 '<cvParam cvRef="MS" accession="MS:1000576" name="no compression" value=""/>',
                 '<cvParam cvRef="MS" accession="%s" name="%s" value="%s"/><binary>%s</binary></binaryDataArray>'),
          length(values), nchar(encoded), accession, name, value, encoded)
}

write_spectrum <- function(position) {
  paste0(sprintf('<spectrum index="%d" id="scan=%d" defaultArrayLength="3">', position, position + 1),
         '<cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="2"/>',
         '<cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>',
         sprintf('<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="%d" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList>', position + 1),
         sprintf('<precursorList count="1"><precursor><selectedIonList count="1"><selectedIon><cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="%d"/></selectedIon></selectedIonList></precursor></precursorList>', 300 + position),
         '<binaryDataArrayList count="3">',
         write_array("MS:1000514", "m/z array", "", c(100.5, 200.25, 300.125)),
         write_array("MS:1000515", "intensity array", "", c(10, 20, 30)),
         write_array("MS:1000786", "non-standard data array", "ms2_fingerprint", get_fingerprint(position)),
         '</binaryDataArrayList></spectrum>')
}

open_arrays_file <- function() {
  spectra <- paste(vapply(0:2, write_spectrum, character(1)), collapse = "")
  text <- paste0('<?xml version="1.0"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">',
                 '<cvList count="1"><cv id="MS" fullName="x" URI="x"/></cvList><run id="r">',
                 '<spectrumList count="3" defaultDataProcessingRef="d">', spectra, '</spectrumList></run></mzML>')
  parse_ion(write_ion(parse_mzml(charToRaw(text))))
}

fingerprint_array <- list(key = "fingerprint", accession = "MS:1000786")
whole_run <- c(0, 10)

get_rts <- function(rt, mz) {
  get_scans(open_arrays_file(), rt = rt, mz = mz, options = get_scans_options(level = 2L))$rt
}

test_that("an rt range and an mz range keep scans inside both", {
  expect_equal(get_rts(c(0.5, 2.5), c(300.5, 302.5)), 2)
})

test_that("an rt range and the closest mz give one scan inside the range", {
  expect_equal(get_rts(c(1.5, 3.5), 300), 2)
})

test_that("the closest rt and an mz range give the nearest scan at that precursor", {
  expect_equal(get_rts(1, c(300.5, 302.5)), 2)
})

test_that("the closest rt and the closest mz pick the closest precursor first", {
  expect_equal(get_rts(3, 300.2), 1)
})

test_that("default arrays give mz and intensity only", {
  scans <- get_scans(open_arrays_file(), rt = whole_run, options = get_scans_options(level = 2L))
  expect_equal(nrow(scans), 3)
  expect_equal(scans$mz[[1]], c(100.5, 200.25, 300.125))
  expect_equal(scans$intensity[[1]], c(10, 20, 30))
  expect_null(scans$fingerprint)
})

test_that("a custom array reads back exactly with negatives", {
  options <- get_scans_options(level = 2L, arrays = list(fingerprint_array))
  scans <- get_scans(open_arrays_file(), rt = whole_run, options = options)
  expect_equal(nrow(scans), 3)
  for (index in 1:3) {
    expect_identical(scans$fingerprint[[index]], get_fingerprint(index - 1))
    expect_length(scans$mz[[index]], 0)
    expect_equal(scans$precursor_mz[index], 299 + index)
  }
  expect_true(any(scans$fingerprint[[1]] < 0))
})

test_that("a custom array can come with mz and intensity", {
  arrays <- list(list(key = "mz", accession = "MS:1000514"), list(key = "intensity", accession = "MS:1000515"), fingerprint_array)
  options <- get_scans_options(level = 2L, arrays = arrays)
  scans <- get_scans(open_arrays_file(), mz = c(300.5, 301.5), options = options)
  expect_equal(nrow(scans), 1)
  expect_equal(scans$mz[[1]], c(100.5, 200.25, 300.125))
  expect_identical(scans$fingerprint[[1]], get_fingerprint(1))
})

test_that("bad arrays are refused", {
  bad <- list(
    list(),
    "mz",
    list(list(key = "", accession = "MS:1000786")),
    list(list(key = "rt", accession = "MS:1000786")),
    list(list(key = "fingerprint", accession = "1000786")),
    list(list(key = "mz", accession = "MS:1000786")),
    list(list(key = "masses", accession = "MS:1000514")),
    list(fingerprint_array, fingerprint_array),
    list(list(key = "fingerprint")),
    list(list(key = "fingerprint", accession = "MS:1000786", name = "x"))
  )
  for (arrays in bad) expect_error(get_scans_options(arrays = arrays), class = "quantion_error")
})
