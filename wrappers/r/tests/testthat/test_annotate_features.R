sample_file <- sample_ion_path()
folder <- if (nzchar(sample_file)) file.path(dirname(dirname(sample_file)), "annotate") else ""
sample_names <- c("sample_peak_and_ms2", "sample_peak_only", "sample_no_peak")
features <- data.frame(mz = 195.0877, rt = 2.0)
without_candidates <- data.frame(mz = c(350.05, 500.0), rt = c(2.0, 2.0))

get_files <- function() lapply(sample_names, function(name) parse_ion(file.path(folder, paste0(name, ".ion"))))

get_options <- function(model = "model", ...) {
  annotate_features_options(library = file.path(folder, "library.ion"),
                            model = file.path(folder, paste0(model, ".onnx")), ...)
}

test_that("the best compound names the feature", {
  skip_if(!nzchar(folder), "no fixtures next to the package")
  first <- annotate_features(get_files(), features, get_options())
  expect_equal(nrow(first), 1)
  expect_equal(c(first$label, first$id, first$database), c("Caffeine", "ENTRY_1", "SOURCE_A"))
  expect_equal(first$frequency, 1 / 3)
  expect_gt(first$score, 0.99)
  expect_gt(first$cosine, 0.99)
})

test_that("each file keeps its best matches in order", {
  skip_if(!nzchar(folder), "no fixtures next to the package")
  files <- annotate_features(get_files(), features, get_options())$files[[1]]
  expect_equal(files$file, c(0, 1, 2))
  expect_false(is.na(files$scan_index[1]))
  expect_true(is.na(files$scan_index[2]))
  matches <- files$matches[[1]]
  expect_equal(matches$label, c("Caffeine", "Compound Y"))
  expect_gte(matches$score[1], matches$score[2])
  expect_equal(matches$mz[[1]], c(110.0713, 123.0427, 138.0662))
  expect_equal(matches$metadata[[1]], c(family = "FAMILY_C", note = "synthetic 1"))
})

test_that("matches limits the list of each file", {
  skip_if(!nzchar(folder), "no fixtures next to the package")
  files <- annotate_features(get_files(), features, get_options(matches = 1))$files[[1]]
  expect_equal(files$matches[[1]]$label, "Caffeine")
})

test_that("a feature without candidates has no label", {
  skip_if(!nzchar(folder), "no fixtures next to the package")
  annotations <- annotate_features(get_files(), without_candidates, get_options())
  expect_true(all(is.na(annotations$label)))
  expect_true(all(is.na(annotations$score)))
  expect_equal(annotations$frequency, c(0, 0))
  for (files in annotations$files) expect_true(all(vapply(files$matches, nrow, 0L) == 0))
})

test_that("no features give no annotations and no files are refused", {
  skip_if(!nzchar(folder), "no fixtures next to the package")
  expect_equal(nrow(annotate_features(get_files(), data.frame(mz = numeric(), rt = numeric()), get_options())), 0)
  expect_error(annotate_features(list(), features, get_options()), "files")
})

test_that("invalid options and models with missing or invalid keys are refused", {
  skip_if(!nzchar(folder), "no fixtures next to the package")
  expect_error(annotate_features(get_files(), features, get_options(matches = 0)), "matches")
  expect_error(annotate_features(get_files(), features, get_options(rt_range = 0)), "rt_range")
  expect_error(annotate_features(get_files(), features, get_options(min_relative_intensity = 2)), "min_relative_intensity")
  expect_error(annotate_features_options(library = ""), "library")
  expect_error(annotate_features_options(color = "red"), "color")
  expect_error(annotate_features(get_files(), features, get_options("model_without_label")), "label")
  expect_error(annotate_features(get_files(), features, get_options("model_with_bad_max_peaks")), "max_peaks")
})

test_that("a library from a URL gives the same result", {
  skip_if_not_installed("callr")
  skip_if(!nzchar(folder), "no fixtures next to the package")
  local <- annotate_features(get_files(), features, get_options())
  with_served_file(file.path(folder, "library.ion"), TRUE, function(url) {
    options <- annotate_features_options(library = url, model = file.path(folder, "model.onnx"))
    expect_identical(annotate_features(get_files(), features, options), local)
  })
})
