if (nzchar(Sys.getenv("QUANTION_ARTIFACTS_ROOT", "")) || nzchar(Sys.getenv("QUANTION_LIB", ""))) {
  message("quantion: the package will load the library at run time - nothing copied in")
  quit(status = 0L)
}

read_value <- function(name) {
  value <- tryCatch(read.dcf("DESCRIPTION")[1, name], error = function(e) "")
  if (length(value) == 0 || is.na(value) || !nzchar(value)) "" else trimws(value)
}

stop_install <- function(...) {
  message(...)
  quit(status = 1L)
}

find_platform <- function() {
  os <- tolower(Sys.info()[["sysname"]])
  cpu <- tolower(Sys.info()[["machine"]])
  if (is.na(cpu) || !nzchar(cpu)) cpu <- ""
  if (os == "darwin") {
    if (grepl("arm64|aarch64", cpu)) return("macos-arm64")
    if (grepl("x86_64|amd64|x86-64", cpu)) return("macos-x86_64")
  }
  if (os == "linux") {
    if (grepl("arm64|aarch64", cpu)) return("linux-arm64")
    if (grepl("x86_64|amd64|x86-64", cpu)) return("linux-x86_64")
  }
  if (os == "windows") {
    if (grepl("arm64|aarch64", cpu)) stop_install("quantion: Windows ARM64 is not supported yet")
    if (grepl("x86_64|amd64|x86-64", cpu)) return("windows-x86_64")
  }
  stop_install(sprintf("quantion: unsupported platform: %s (%s)", os, cpu))
}

find_platform_dir <- function(root, platform) {
  direct <- file.path(root, platform)
  if (dir.exists(direct)) return(direct)
  found <- list.dirs(root, recursive = FALSE, full.names = FALSE)
  found <- found[vapply(found, function(v) dir.exists(file.path(root, v, platform)), logical(1))]
  if (!length(found)) return("")
  best <- found[order(numeric_version(found, strict = FALSE), decreasing = TRUE, na.last = TRUE)][1]
  file.path(root, best, platform)
}

copy_files <- function(source, target, platform) {
  files <- list.files(source, full.names = TRUE)
  if (!length(files)) stop_install("quantion: no files found in ", source)
  dir.create(target, recursive = TRUE, showWarnings = FALSE)
  ok <- file.copy(files, target, overwrite = TRUE)
  if (!all(ok)) stop_install("quantion: failed to copy files into ", target)
  message("quantion: staged ", platform, " into ", target)
}

find_local_files <- function(platform) {
  type <- read_value("RemoteType")
  path <- read_value("RemoteUrl")
  if (type != "local" || !nzchar(path)) return("")
  roots <- c(path, dirname(path), dirname(dirname(path)))
  for (root in roots) {
    source <- find_platform_dir(file.path(root, "artifacts"), platform)
    if (nzchar(source)) return(source)
  }
  ""
}

release_url <- function(version, asset) {
  base <- Sys.getenv("QUANTION_RELEASE_URL", "https://github.com/phenological/quantion/releases/download")
  sprintf("%s/v%s/%s", base, version, asset)
}

library_extension <- function(platform) {
  if (startsWith(platform, "windows")) ".dll"
  else if (startsWith(platform, "macos")) ".dylib"
  else ".so"
}

download_to <- function(url, destination) {
  status <- tryCatch(utils::download.file(url, destination, mode = "wb", quiet = TRUE), error = function(e) 1L)
  identical(status, 0L) && file.exists(destination) && file.size(destination) > 0
}

expected_checksum <- function(version, asset) {
  listing <- tempfile("quantion-sums-")
  on.exit(unlink(listing), add = TRUE)
  if (!download_to(release_url(version, "SHA256SUMS.txt"), listing)) return("")
  for (line in readLines(listing, warn = FALSE)) {
    parts <- strsplit(trimws(line), "\\s+")[[1]]
    if (length(parts) == 2 && parts[2] == asset) return(parts[1])
  }
  ""
}

checksum_matches <- function(path, expected) {
  if (!nzchar(expected)) return(TRUE)
  if (!exists("sha256sum", envir = asNamespace("tools"))) return(TRUE)
  actual <- unname(tools::sha256sum(path))
  identical(tolower(actual), tolower(expected))
}

download_files <- function(platform, target) {
  version <- read_value("Version")
  if (!nzchar(version)) stop_install("quantion: DESCRIPTION has no Version")
  extension <- library_extension(platform)
  asset <- paste0("libquantion-", platform, extension)
  url <- release_url(version, asset)
  dir.create(target, recursive = TRUE, showWarnings = FALSE)
  destination <- file.path(target, paste0("libquantion", extension))
  if (!download_to(url, destination)) {
    unlink(destination)
    stop_install("quantion: failed to download ", url)
  }
  if (!checksum_matches(destination, expected_checksum(version, asset))) {
    unlink(destination)
    stop_install("quantion: checksum mismatch for ", asset)
  }
  message("quantion: downloaded ", asset, " into ", target)
}

repo_artifacts <- function() {
  here <- normalizePath(getwd(), mustWork = FALSE)
  repeat {
    candidate <- file.path(here, "artifacts")
    if (dir.exists(candidate)) return(candidate)
    parent <- dirname(here)
    if (identical(parent, here)) return("")
    here <- parent
  }
}

if (nzchar(repo_artifacts())) {
  message("quantion: building against ", repo_artifacts(), " - nothing copied into the package")
  quit(status = 0L)
}

platform <- find_platform()
target <- file.path("inst", "libs", platform)
env_root <- Sys.getenv("QUANTION_ARTIFACTS_ROOT", "")

if (nzchar(env_root)) {
  source <- find_platform_dir(env_root, platform)
  if (!nzchar(source)) stop_install("quantion: artifacts not found at ", env_root)
  copy_files(source, target, platform)
  quit(status = 0L)
}

source <- find_platform_dir(file.path("..", "..", "artifacts"), platform)
if (nzchar(source)) {
  copy_files(source, target, platform)
  quit(status = 0L)
}

source <- find_local_files(platform)
if (nzchar(source)) {
  copy_files(source, target, platform)
  quit(status = 0L)
}

download_files(platform, target)