\name{parse_ion}
\alias{parse_ion}
\title{Open an Ionic file (.ion)}
\description{
Open an Ionic file (.ion) from a path, a URL or bytes. Paths and URLs are read lazily: only the bytes a query needs are fetched.
}
\usage{
parse_ion(source, options = parse_ion_options())
}
\arguments{
  \item{source}{Where the file is: a path, an http(s) URL, or raw bytes. Type: \code{character | raw}.}
  \item{options}{Settings from \code{\link{parse_ion_options}()}: \code{max_cache_size = 0}.
    \href{https://phenological.github.io/quantion/?language=r#ParseIonOptions}{ParseIonOptions documentation}
  }
}
\value{
The opened sample. Type: \code{SampleFile}.
}
\section{Warning}{
The sample is freed by the library once it is no longer used, so dispose is not needed in normal use. When processing many samples, dispose each one after using it, so its memory is freed right away instead of piling up.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#parse_ion}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  url <- "https://raw.githubusercontent.com/phenological/ion-files/main/data/aa_p21_LTR_20_b1_w20.ion"
  sample <- parse_ion(url)
  cached <- parse_ion(url, options = parse_ion_options(max_cache_size = 64e6))
  print(sample)

  dispose(sample)
  dispose(cached)
}
}
}
\seealso{
\code{\link{parse_mzml}()}, \code{\link{write_ion}()}, \code{\link{dispose}()}
}
