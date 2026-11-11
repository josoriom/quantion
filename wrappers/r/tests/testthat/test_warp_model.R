sample_file <- sample_ion_path()
fixtures <- if (nzchar(sample_file)) dirname(dirname(sample_file)) else ""
model_file <- file.path(fixtures, "shift_model.onnx")
empty_file <- file.path(fixtures, "test.ion")
get_warp <- function(source) load_warp(load_model(source))

targets <- data.frame(id = c("first", "second"), mz = c(250.1, 310.2), rt = c(1.2, 3.1))

test_that("a warp model gives one prediction per target, in order", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  warp <- get_warp(model_file)
  bin <- parse_ion(sample_file)
  predicted <- predict_rts(warp, bin, targets)
  expect_equal(names(predicted), c("rt", "uncertainty"))
  expect_equal(nrow(predicted), nrow(targets))
  expect_true(all(is.finite(predicted$rt)))
  dispose(warp)
  dispose(bin)
})

test_that("a warp model from bytes or a URL predicts like one from a path", {
  skip_if_not_installed("callr")
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  bin <- parse_ion(sample_file)
  from_path <- predict_rts(get_warp(model_file), bin, targets)
  bytes <- readBin(model_file, "raw", file.info(model_file)$size)
  expect_identical(predict_rts(get_warp(bytes), bin, targets), from_path)
  with_served_file(model_file, FALSE, function(url) {
    expect_identical(predict_rts(get_warp(url), bin, targets), from_path)
  })
})

test_that("a remote file predicts like a local one", {
  skip_if_not_installed("callr")
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  warp <- get_warp(model_file)
  local <- predict_rts(warp, parse_ion(sample_file), targets)
  with_served_file(sample_file, TRUE, function(url) {
    expect_identical(predict_rts(warp, parse_ion(url), targets), local)
  })
})

test_that("a target without mz is named by its index", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  broken <- data.frame(id = c("first", "broken"), mz = c(250.1, NA), rt = c(1.2, 2.0))
  expect_error(predict_rts(get_warp(model_file), parse_ion(sample_file), broken),
               "predict_rts: target 2 has no valid m/z or rt")
})

test_that("a file whose scans do not cover the model range is refused", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  expect_error(predict_rts(get_warp(model_file), parse_ion(empty_file), targets),
               "predict_rts: the file has no scans, the warp model needs")
})

test_that("a disposed warp model cannot be used", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  warp <- get_warp(model_file)
  dispose(warp)
  expect_error(predict_rts(warp, parse_ion(sample_file), targets), "disposed warp model")
})

test_that("keys and values come from the model", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  warp <- get_warp(model_file)
  expect_equal(get_values(warp)[get_keys(warp) == "points"], "64")
  expect_equal(get_value(warp, "points"), "64")
  expect_null(get_value(warp, "missing"))
})

test_that("calibrate with one sample moves each rt by its predicted shift", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  warp <- get_warp(model_file)
  bin <- parse_ion(sample_file)
  predicted <- predict_rts(warp, bin, targets)
  calibrated <- calibrate(warp, list(bin), list(targets))
  expect_identical(calibrated$id, targets$id)
  expect_identical(calibrated$rt, 2 * targets$rt - predicted$rt)
})

test_that("calibrate takes the median over the samples", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  warp <- get_warp(model_file)
  bin <- parse_ion(sample_file)
  early <- targets
  early$rt[1] <- early$rt[1] - 0.2
  late <- targets
  late$rt[1] <- late$rt[1] + 0.2
  middle <- calibrate(warp, list(bin), list(targets))
  expect_identical(calibrate(warp, list(bin, bin, bin), list(early, targets, late)), middle)
})

test_that("calibrate needs one target list per sample", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  bin <- parse_ion(sample_file)
  expect_error(calibrate(get_warp(model_file), list(bin), list(targets, targets)),
               "got 1 samples and 2 target lists")
})

test_that("calibrate needs an id for each target", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  broken <- data.frame(id = c("first", ""), mz = c(250.1, 310.2), rt = c(1.2, 3.1))
  expect_error(calibrate(get_warp(model_file), list(parse_ion(sample_file)), list(broken)),
               "calibrate: target 2 in list 1 has no id")
})

test_that("a model and a warp can be disposed twice", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  model <- load_model(model_file)
  warp <- load_warp(model)
  dispose(model)
  dispose(model)
  dispose(warp)
  expect_silent(dispose(warp))
})

test_that("the warp keeps working after the model is disposed", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  model <- load_model(model_file)
  warp <- load_warp(model)
  dispose(model)
  expect_equal(nrow(predict_rts(warp, parse_ion(sample_file), targets)), nrow(targets))
  expect_equal(get_value(warp, "points"), "64")
})

test_that("the model keeps working after the warp is made", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  model <- load_model(model_file)
  dispose(load_warp(model))
  expect_equal(get_value(model, "points"), "64")
  expect_equal(get_keys(model), get_keys(get_warp(model_file)))
})

test_that("load_warp needs a model", {
  expect_error(load_warp(model_file), "load_warp: model must be a model from load_model")
})

test_that("a missing model file is refused", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  expect_error(load_model(file.path(fixtures, "missing.onnx")), "load_model: the file could not be read")
})
