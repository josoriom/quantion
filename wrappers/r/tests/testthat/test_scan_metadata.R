scan_count <- 6
two_peaks_base64 <- "AAAAAAAAWUAAAAAAAABpQA=="
mz_unit <- 'unitCvRef="MS" unitAccession="MS:1000040" unitName="m/z"'

get_level <- function(position) if (position %% 2 == 0) 1 else 2

get_library_id <- function(position) sprintf("SYNTHETIC%03d", position)

write_peaks <- function(accession, name) {
  paste0('<binaryDataArray encodedLength="24">',
         '<cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/>',
         '<cvParam cvRef="MS" accession="MS:1000576" name="no compression"/>',
         sprintf('<cvParam cvRef="MS" accession="%s" name="%s"/>', accession, name),
         '<binary>', two_peaks_base64, '</binary></binaryDataArray>')
}

write_precursor <- function(position) {
  if (get_level(position) == 1) return("")
  mz <- 300 + position
  paste0(sprintf('<precursorList count="1"><precursor spectrumRef="scan=%d"><isolationWindow>', position),
         sprintf('<cvParam cvRef="MS" accession="MS:1000827" name="isolation window target m/z" value="%d" %s/>', mz, mz_unit),
         '</isolationWindow><selectedIonList count="2"><selectedIon>',
         sprintf('<cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="%d" %s/>', mz, mz_unit),
         '</selectedIon><selectedIon><cvParam cvRef="MS" accession="MS:1000041" name="charge state" value="2"/>',
         '</selectedIon></selectedIonList><activation>',
         '<cvParam cvRef="MS" accession="MS:1000133" name="collision-induced dissociation"/>',
         '</activation></precursor></precursorList>')
}

write_library_spectrum <- function(position) {
  level <- get_level(position)
  start <- position + 1
  paste0(sprintf('<spectrum index="%d" id="scan=%d" defaultArrayLength="2" dataProcessingRef="synthetic_processing">', position, start),
         '<referenceableParamGroupRef ref="common_params"/>',
         sprintf('<cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="%d"/>', level),
         '<cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/>',
         '<cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>',
         sprintf('<userParam name="library_id" value="%s"/>', get_library_id(position)),
         sprintf('<userParam name="formula" value="C%dH%d"/>', start, level),
         '<userParam name="library_id" value="REPEATED"/>',
         '<scanList count="1"><cvParam cvRef="MS" accession="MS:1000795" name="no combination"/>',
         '<scan instrumentConfigurationRef="synthetic_instrument">',
         sprintf('<cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="%d" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/>', start),
         '<userParam name="scan_note" value="synthetic note"/>',
         '<scanWindowList count="1"><scanWindow>',
         sprintf('<cvParam cvRef="MS" accession="MS:1000501" name="scan window lower limit" value="50" %s/>', mz_unit),
         '</scanWindow></scanWindowList></scan></scanList>',
         write_precursor(position),
         '<binaryDataArrayList count="2">',
         write_peaks("MS:1000514", "m/z array"),
         write_peaks("MS:1000515", "intensity array"),
         '</binaryDataArrayList></spectrum>')
}

write_library_mzml <- function() {
  spectra <- paste(vapply(seq(0, scan_count - 1), write_library_spectrum, character(1)), collapse = "")
  charToRaw(paste0('<?xml version="1.0"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">',
                   '<cvList count="1"><cv id="MS" fullName="x" URI="x"/></cvList>',
                   '<referenceableParamGroupList count="1"><referenceableParamGroup id="common_params">',
                   '<userParam name="group_note" value="shared"/></referenceableParamGroup></referenceableParamGroupList>',
                   '<run id="r">',
                   sprintf('<spectrumList count="%d" defaultDataProcessingRef="d">', scan_count),
                   spectra, '</spectrumList></run></mzML>'))
}

row <- function(path, kind, accession, name, value, unit = NA_character_, unit_accession = NA_character_) {
  data.frame(path = path, kind = kind, accession = accession, name = name, value = value,
             unit = unit, unit_accession = unit_accession, stringsAsFactors = FALSE)
}

expected_rows <- function(position) {
  level <- as.character(get_level(position))
  start <- as.character(position + 1)
  mz <- as.character(300 + position)
  scan <- "spectrum/scanList/scan[0]"
  rows <- list(
    row("spectrum", "ATTRIBUTE", NA, "index", as.character(position)),
    row("spectrum", "ATTRIBUTE", NA, "id", paste0("scan=", start)),
    row("spectrum", "ATTRIBUTE", NA, "defaultArrayLength", "2"),
    row("spectrum", "ATTRIBUTE", NA, "dataProcessingRef", "synthetic_processing"),
    row("spectrum/referenceableParamGroupRef[0]", "ATTRIBUTE", NA, "ref", "common_params"),
    row("spectrum", "CV", "MS:1000511", "ms level", level),
    row("spectrum", "CV", "MS:1000127", "centroid spectrum", NA),
    row("spectrum", "CV", "MS:1000130", "positive scan", NA),
    row("spectrum", "USER", NA, "library_id", get_library_id(position)),
    row("spectrum", "USER", NA, "formula", sprintf("C%sH%s", start, level)),
    row("spectrum", "USER", NA, "library_id", "REPEATED"),
    row("spectrum/scanList", "CV", "MS:1000795", "no combination", NA),
    row(scan, "ATTRIBUTE", NA, "instrumentConfigurationRef", "synthetic_instrument"),
    row(scan, "CV", "MS:1000016", "scan start time", start, "minute", "UO:0000031"),
    row(scan, "USER", NA, "scan_note", "synthetic note"),
    row(paste0(scan, "/scanWindowList/scanWindow[0]"), "CV", "MS:1000501", "scan window lower limit", "50", "m/z", "MS:1000040")
  )
  if (level == "2") {
    precursor <- "spectrum/precursorList/precursor[0]"
    rows <- c(rows, list(
      row(precursor, "ATTRIBUTE", NA, "spectrumRef", paste0("scan=", position)),
      row(paste0(precursor, "/isolationWindow"), "CV", "MS:1000827", "isolation window target m/z", mz, "m/z", "MS:1000040"),
      row(paste0(precursor, "/selectedIonList/selectedIon[0]"), "CV", "MS:1000744", "selected ion m/z", mz, "m/z", "MS:1000040"),
      row(paste0(precursor, "/selectedIonList/selectedIon[1]"), "CV", "MS:1000041", "charge state", "2"),
      row(paste0(precursor, "/activation"), "CV", "MS:1000133", "collision-induced dissociation", NA)
    ))
  }
  arrays <- list(c("MS:1000514", "m/z array"), c("MS:1000515", "intensity array"))
  for (number in seq_along(arrays)) {
    array <- sprintf("spectrum/binaryDataArrayList/binaryDataArray[%d]", number - 1)
    rows <- c(rows, list(
      row(array, "ATTRIBUTE", NA, "encodedLength", "24"),
      row(array, "CV", "MS:1000523", "64-bit float", NA),
      row(array, "CV", "MS:1000576", "no compression", NA),
      row(array, "CV", arrays[[number]][1], arrays[[number]][2], NA)
    ))
  }
  result <- do.call(rbind, rows)
  result$accession <- as.character(result$accession)
  result$value <- as.character(result$value)
  rownames(result) <- NULL
  result
}

only_tags <- function(...) get_scan_metadata_options(tags = c(...))

open_library_files <- function() {
  mzml <- parse_mzml(write_library_mzml())
  list(mzml = mzml, ion = parse_ion(write_ion(mzml)))
}

every_scan <- c(0, 100)

test_that("each scan has its position in the spectrum list", {
  for (file in open_library_files()) {
    all <- get_scans(file, rt = every_scan, options = get_scans_options(level = 0L))
    expect_identical(all$index, as.numeric(0:5))
    level_two <- get_scans(file, rt = every_scan, options = get_scans_options(level = 2L))
    expect_identical(level_two$index, c(1, 3, 5))
  }
})

test_that("the index is set when reading chosen arrays", {
  arrays <- list(list(key = "intensity", accession = "MS:1000515"))
  for (file in open_library_files()) {
    scans <- get_scans(file, rt = every_scan, options = get_scans_options(level = 2L, arrays = arrays))
    expect_identical(scans$index, c(1, 3, 5))
  }
})

test_that("the metadata has every tag in tree order", {
  for (file in open_library_files()) {
    for (position in seq(0, scan_count - 1)) {
      expect_identical(get_scan_metadata(file, position), expected_rows(position))
    }
  }
})

test_that("the metadata is the same from mzML and ion", {
  files <- open_library_files()
  for (position in seq(0, scan_count - 1)) {
    expect_identical(get_scan_metadata(files$mzml, position), get_scan_metadata(files$ion, position))
  }
})

test_that("found scans match their metadata by index", {
  file <- open_library_files()$ion
  scans <- get_scans(file, rt = every_scan, options = get_scans_options(level = 2L))
  for (index in scans$index) {
    rows <- get_scan_metadata(file, index)
    expect_identical(rows$value[rows$name == "library_id"][1], get_library_id(index))
  }
})

test_that("kind is uppercase", {
  expect_setequal(unique(get_scan_metadata(open_library_files()$ion, 1)$kind), c("ATTRIBUTE", "CV", "USER"))
})

test_that("a missing field is NA", {
  rows <- get_scan_metadata(open_library_files()$ion, 0)
  centroid <- rows[rows$name == "centroid spectrum", ]
  expect_true(is.na(centroid$value))
  expect_true(is.na(centroid$unit))
  expect_identical(centroid$accession, "MS:1000127")
})

test_that("tags keep only the chosen tags", {
  for (file in open_library_files()) {
    expect_identical(get_scan_metadata(file, 1, only_tags("selectedIon"))$name, c("selected ion m/z", "charge state"))
    expect_identical(get_scan_metadata(file, 1, only_tags("activation", "scan"))$path,
                     c(rep("spectrum/scanList/scan[0]", 3), "spectrum/precursorList/precursor[0]/activation"))
  }
})

test_that("a tag that is not there gives no rows", {
  rows <- get_scan_metadata(open_library_files()$ion, 0, only_tags("precursor"))
  expect_identical(nrow(rows), 0L)
  expect_identical(names(rows), c("path", "kind", "accession", "name", "value", "unit", "unit_accession"))
})

test_that("bad tags are refused", {
  for (tags in list("cvParam", "Scan", "scan[0]", "", NA_character_, 1)) {
    expect_error(get_scan_metadata_options(tags = tags), "get_scan_metadata_options: tags must be tag names from")
  }
})

test_that("options must come from get_scan_metadata_options", {
  expect_error(get_scan_metadata(open_library_files()$ion, 0, list(tags = "scan")),
               "get_scan_metadata: options must be built with get_scan_metadata_options")
})

test_that("an index out of range is refused", {
  for (file in open_library_files()) {
    expect_error(get_scan_metadata(file, scan_count), "get_scan_metadata: index must be less than the scan count")
  }
})

test_that("an index that is not a whole number 0 or more is refused", {
  file <- open_library_files()$ion
  for (index in list(-1, 1.5, NA, Inf, "1", TRUE, c(1, 2), NULL)) {
    expect_error(get_scan_metadata(file, index), "get_scan_metadata: index must be a whole number 0 or more")
  }
})

test_that("index cannot be an array key", {
  arrays <- list(list(key = "index", accession = "MS:1000786"))
  expect_error(get_scans(open_library_files()$ion, rt = every_scan, options = get_scans_options(arrays = arrays)),
               "arrays.key cannot be index")
})

test_that("the metadata over a URL reads only the planned bytes", {
  skip_if_not_installed("callr")
  path <- tempfile(fileext = ".ion")
  on.exit(unlink(path), add = TRUE)
  writeBin(write_ion(parse_mzml(write_library_mzml())), path)
  with_served_file(path, TRUE, function(url) {
    for (position in c(0, 3, scan_count - 1)) {
      file <- parse_ion(url)
      held_after_open <- length(attr(file, "quantion_source")$held)
      expect_identical(get_scan_metadata(file, position), expected_rows(position))
      expect_gt(length(attr(file, "quantion_source")$held), held_after_open)
    }
  })
})
