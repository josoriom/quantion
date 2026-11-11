\name{dispose}
\alias{dispose}
\title{Free native memory}
\description{
Free the memory held by a sample, model or warp model.
}
\usage{
dispose(object)
}
\arguments{
  \item{object}{What to free. Type: \code{SampleFile | Model | WarpModel}.}
}
\value{
Nothing. Type: \code{NULL (invisible)}.
}
\section{Warning}{
Not needed in normal use: the library frees a sample, model or warp model by itself once it is no longer used. When processing many samples, dispose each one after using it, so its memory is freed right away instead of piling up.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#dispose}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  url <- "https://raw.githubusercontent.com/phenological/ion-files/main/data/aa_p21_LTR_20_b1_w20.ion"
  sample <- parse_ion(url)
  dispose(sample)
}
}
}
