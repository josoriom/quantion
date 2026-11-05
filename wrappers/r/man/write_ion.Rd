\name{write_ion}
\alias{write_ion}
\title{Write a sample as an Ionic file (.ion)}
\description{
Write a sample as an Ionic file (.ion). With no out, it returns the bytes. A path writes a file; a URL uploads it in one PUT request.
}
\usage{
write_ion(file, options = write_ion_options())
}
\arguments{
  \item{file}{The sample to write. Type: \code{SampleFile}.}
  \item{options}{Settings from \code{\link{write_ion_options}()}: \code{out = NULL}, \code{overwrite = FALSE}, \code{level = 12L}, \code{f32_compress = FALSE}, \code{block_size = 8L}, \code{mz_window = 20L}, \code{headers}.
    \href{https://phenological.github.io/quantion/?language=r#WriteIonOptions}{WriteIonOptions documentation}
  }
}
\value{
The bytes when out is left out; nothing when it is written or uploaded. Type: \code{raw | NULL (invisible)}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#write_ion}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  fixtures <- "https://raw.githubusercontent.com/phenological/quantion/main/core/tests/fixtures"
  url <- paste0(fixtures, "/api/api.ion")
  data <- curl::curl_fetch_memory(url)$content
  sample <- parse_ion(data)

  bytes <- write_ion(sample)

  write_ion(sample, options = write_ion_options(out = tempfile(fileext = ".ion")))
  print(length(bytes))
}
}
}
\seealso{
\code{\link{write_ion_options}()}, \code{\link{header}()}, \code{\link{write_mzml}()}
}
