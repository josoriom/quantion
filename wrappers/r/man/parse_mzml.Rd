\name{parse_mzml}
\alias{parse_mzml}
\title{Open an mzML file}
\description{
Open an mzML file from a path or bytes. It is always loaded fully into memory. URLs are refused.
}
\usage{
parse_mzml(source)
}
\arguments{
  \item{source}{Where the file is: a path or raw bytes. Type: \code{character | raw}.}
}
\value{
The loaded sample. Type: \code{SampleFile}.
}
\section{Warning}{
The sample is freed by the library once it is no longer used, so dispose is not needed in normal use. When processing many samples, dispose each one after using it, so its memory is freed right away instead of piling up.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#parse_mzml}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  fixtures <- "https://raw.githubusercontent.com/phenological/quantion/main/core/tests/fixtures"
  url <- paste0(fixtures, "/api/api.ion")
  data <- curl::curl_fetch_memory(url)$content
  path <- tempfile(fileext = ".mzML")
  write_mzml(parse_ion(data), options = write_mzml_options(out = path))
  sample <- parse_mzml(path)
  print(sample)
}
}
}
\seealso{
\code{\link{parse_ion}()}, \code{\link{write_mzml}()}
}
