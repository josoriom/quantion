\name{write_mzml}
\alias{write_mzml}
\title{Write a sample as mzML}
\description{
Write a sample as mzML. With no out, it returns the bytes. Only a path is allowed, no URL.
}
\usage{
write_mzml(file, options = write_mzml_options())
}
\arguments{
  \item{file}{The sample to write. Type: \code{SampleFile}.}
  \item{options}{Settings from \code{\link{write_mzml_options}()}: \code{out = NULL}, \code{overwrite = FALSE}.
    \href{https://phenological.github.io/quantion/?language=r#WriteMzmlOptions}{WriteMzmlOptions documentation}
  }
}
\value{
The bytes when out is left out; nothing when it is written. Type: \code{raw | NULL (invisible)}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#write_mzml}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  fixtures <- "https://raw.githubusercontent.com/phenological/quantion/main/core/tests/fixtures"
  url <- paste0(fixtures, "/api/api.ion")
  data <- curl::curl_fetch_memory(url)$content
  sample <- parse_ion(data)
  path <- tempfile(fileext = ".mzML")
  write_mzml(sample, options = write_mzml_options(out = path))
  print(file.size(path))
}
}
}
\seealso{
\code{\link{write_mzml_options}()}, \code{\link{write_ion}()}
}
