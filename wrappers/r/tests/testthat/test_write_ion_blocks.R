synthetic_scan_count <- 20
megabyte <- 1024 * 1024
two_peaks_base64 <- "AAAAAAAAWUAAAAAAAABpQA=="

write_synthetic_array <- function(accession, name) {
  paste0('<binaryDataArray encodedLength="24">',
         '<cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/>',
         '<cvParam cvRef="MS" accession="MS:1000576" name="no compression"/>',
         sprintf('<cvParam cvRef="MS" accession="%s" name="%s"/>', accession, name),
         '<binary>', two_peaks_base64, '</binary></binaryDataArray>')
}

write_synthetic_spectrum <- function(position) {
  paste0(sprintf('<spectrum index="%d" id="scan=%d" defaultArrayLength="2">', position, position + 1),
         '<cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/>',
         '<scanList count="1"><scan>',
         sprintf('<cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="%s" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/>', format(position * 0.1)),
         '</scan></scanList><binaryDataArrayList count="2">',
         write_synthetic_array("MS:1000514", "m/z array"),
         write_synthetic_array("MS:1000515", "intensity array"),
         '</binaryDataArrayList></spectrum>')
}

open_synthetic_mzml <- function() {
  spectra <- paste(vapply(seq(0, synthetic_scan_count - 1), write_synthetic_spectrum, character(1)), collapse = "")
  parse_mzml(charToRaw(paste0('<?xml version="1.0"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">',
                              '<cvList count="1"><cv id="MS" fullName="x" URI="x"/></cvList><run id="r">',
                              sprintf('<spectrumList count="%d" defaultDataProcessingRef="d">', synthetic_scan_count),
                              spectra, '</spectrumList></run></mzML>')))
}

read_header <- function(ion) {
  block_bytes <- sum(as.numeric(ion[17:24]) * 256^(0:7))
  mz_window <- sum(as.numeric(ion[25:28]) * 256^(0:3))
  c(block_bytes, mz_window)
}

get_peaks <- function(file) {
  scans <- get_scans(file, rt = c(0, 100), options = get_scans_options(level = 0L))
  list(mz = scans$mz, intensity = scans$intensity)
}

test_that("write_ion defaults to 8 MB blocks and 20 m/z windows", {
  options <- write_ion_options()
  expect_identical(c(options$block_size, options$mz_window), c(8, 20))
  expect_identical(read_header(write_ion(open_synthetic_mzml())), c(8 * megabyte, 20))
})

test_that("chosen values are recorded and peaks read back the same", {
  file <- open_synthetic_mzml()
  expected <- get_peaks(file)
  expect_identical(length(expected$mz), as.integer(synthetic_scan_count))
  for (choice in list(c(1, 20), c(2, 5), c(16, 0), c(8, 100))) {
    ion <- write_ion(file, write_ion_options(block_size = choice[1], mz_window = choice[2]))
    expect_identical(read_header(ion), c(choice[1] * megabyte, choice[2]))
    expect_identical(get_peaks(parse_ion(ion)), expected)
  }
})

test_that("a bad block_size is refused", {
  for (block_size in list(0, -1, 1.5, 2^32, "8", NA, NULL, TRUE, NaN, c(1, 2))) {
    expect_error(write_ion_options(block_size = block_size),
                 "write_ion_options: block_size must be a whole number from 1 to 4294967295")
  }
})

test_that("a bad mz_window is refused", {
  for (mz_window in list(-1, 2.5, 2^32, "20", NA, NULL, TRUE, Inf)) {
    expect_error(write_ion_options(mz_window = mz_window),
                 "write_ion_options: mz_window must be a whole number from 0 to 4294967295")
  }
})

test_that("headers keeps its position in write_ion_options", {
  options <- write_ion_options(NULL, FALSE, 12L, FALSE, list(header("Authorization", "Bearer synthetic")), 2, 5)
  expect_identical(options$headers[[1]]$name, "Authorization")
  expect_identical(c(options$block_size, options$mz_window), c(2, 5))
})
