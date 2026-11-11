find_sample_root <- function() {
  here <- normalizePath(getwd(), mustWork = FALSE)
  repeat {
    if (file.exists(file.path(here, "core", "tests", "fixtures", "api", "api.ion")))
      return(here)
    parent <- dirname(here)
    if (identical(parent, here)) return("")
    here <- parent
  }
}

sample_ion_path <- function() {
  root <- find_sample_root()
  if (!nzchar(root)) return("")
  file.path(root, "core", "tests", "fixtures", "api", "api.ion")
}

write_sample_bytes <- function(size) {
  path <- tempfile(fileext = ".bin")
  writeBin(as.raw(rep(1:255, length.out = size)), path)
  path
}

file_url <- function(path) paste0("file://", path)

serve_file <- function(port, path, honour_range) {
  body <- readBin(path, "raw", file.info(path)$size)
  total <- length(body)
  server <- serverSocket(port)
  on.exit(close(server), add = TRUE)
  repeat {
    con <- tryCatch(socketAccept(server, blocking = TRUE, open = "a+b", timeout = 30),
                    error = function(e) NULL)
    if (is.null(con)) break
    lines <- character(0)
    repeat {
      line <- readLines(con, n = 1, warn = FALSE)
      if (!length(line) || !nzchar(trimws(line))) break
      lines <- c(lines, trimws(line))
    }
    if (!length(lines)) {
      close(con)
      next
    }
    asked <- grep("^Range:", lines, ignore.case = TRUE, value = TRUE)
    first <- 0
    last <- total - 1
    partial <- FALSE
    if (honour_range && length(asked)) {
      numbers <- regmatches(asked[1], regexec("bytes=([0-9]+)-([0-9]*)", asked[1]))[[1]]
      if (length(numbers) == 3) {
        first <- as.numeric(numbers[2])
        last <- if (nzchar(numbers[3])) as.numeric(numbers[3]) else total - 1
        if (last > total - 1) last <- total - 1
        partial <- TRUE
      }
    }
    piece <- body[seq(first + 1, last + 1)]
    head_text <- paste0(
      if (partial) "HTTP/1.1 206 Partial Content\r\n" else "HTTP/1.1 200 OK\r\n",
      "Content-Length: ", length(piece), "\r\n",
      if (partial) paste0("Content-Range: bytes ", first, "-", last, "/", total, "\r\n") else "",
      "Accept-Ranges: bytes\r\n",
      "Connection: close\r\n\r\n"
    )
    writeBin(charToRaw(head_text), con)
    writeBin(piece, con)
    flush(con)
    close(con)
  }
}

find_free_port <- function() {
  for (attempt in seq_len(50)) {
    port <- sample(20000:60000, 1)
    free <- tryCatch({
      socket <- serverSocket(port)
      close(socket)
      TRUE
    }, error = function(e) FALSE)
    if (free) return(port)
  }
  0
}

with_served_file <- function(path, honour_range, work) {
  port <- find_free_port()
  if (port == 0) skip("no free port for a local server")

  child <- callr::r_bg(serve_file, args = list(port, path, honour_range), supervise = TRUE)
  on.exit(child$kill(), add = TRUE)

  url <- sprintf("http://127.0.0.1:%d/sample.ion", port)
  size <- file.info(path)$size
  ready <- FALSE
  for (attempt in seq_len(60)) {
    Sys.sleep(0.1)
    ready <- tryCatch({
      suppressWarnings(quantion:::.fetch_range(url, 0, size))
      TRUE
    }, error = function(e) FALSE)
    if (ready) break
  }
  if (!ready) skip("the local server did not answer")

  work(url)
}
