\name{find_features}
\alias{find_features}
\title{Find every feature in one file}
\description{
Untargeted: find every feature in one file by scanning an m/z grid.
}
\usage{
find_features(file, options = find_features_options())
}
\arguments{
  \item{file}{The sample to read, from parse_ion or parse_mzml. Type: \code{SampleFile}.}
  \item{options}{Settings from \code{\link{find_features_options}()}: \code{eic}, \code{grid}, \code{find_peaks}, \code{threads = 1L}.
    \href{https://phenological.github.io/quantion/?language=r#FindFeaturesOptions}{FindFeaturesOptions documentation}
  }
}
\value{
Every feature found, without frequency. Type: \code{data.frame of Feature}. Columns: \code{mz}, \code{rt}, \code{from}, \code{to}, \code{intensity}, \code{integral}, \code{n_points}, \code{frequency}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#find_features}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  url <- "https://raw.githubusercontent.com/phenological/ion-files/main/data/aa_p21_LTR_20_b1_w20.ion"
  options <- find_features_options(
    eic     = calculate_eic_options(from = 2.3, to = 2.45),
    grid    = grid_options(start = 255, end = 265),
    threads = 2L
  )

  sample <- parse_ion(url)
  features <- find_features(sample, options = options)

  cat(sprintf("\%d features\\n", nrow(features)))
  count <- 3
  if (nrow(features) < count) {
    count <- nrow(features)
  }
  for (index in seq_len(count)) {
    feature <- features[index, ]
    line <- sprintf("m/z \%.4f  rt \%.3f  intensity \%.0f",
                    feature$mz, feature$rt, feature$intensity)
    cat(line, "\\n", sep = "")
  }
}
}
}
\seealso{
\code{\link{find_features_options}()}, \code{\link{match_features}()}
}
