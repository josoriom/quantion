\name{find_targets}
\alias{find_targets}
\title{Find the peak of each target}
\description{
For each target, build its EIC at mz and find its peak near rt +/- range.
}
\usage{
find_targets(file, targets, options = find_targets_options())
}
\arguments{
  \item{file}{The sample to read, from parse_ion or parse_mzml. Type: \code{SampleFile}.}
  \item{targets}{What to look for. A target without range searches its whole EIC. Type: \code{data.frame of Target}. Each Target has \code{id}, \code{mz}, \code{rt}, \code{range}.
    Example: \code{targets <- data.frame(id = "compound_1", mz = 260.103, rt = 2.383, range = 0.05)}
    \href{https://phenological.github.io/quantion/?language=r#Target}{Target documentation}
  }
  \item{options}{Settings from \code{\link{find_targets_options}()}: \code{eic}, \code{find_peaks}, \code{threads = 1L}.
    \href{https://phenological.github.io/quantion/?language=r#FindTargetsOptions}{FindTargetsOptions documentation}
  }
}
\value{
One Peak per target, in the same order. A target with no peak gets a Peak with every field 0. Type: \code{data.frame of Peak}. Columns: \code{rt}, \code{from}, \code{to}, \code{intensity}, \code{integral}, \code{n_points}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#find_targets}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  url <- "https://raw.githubusercontent.com/phenological/ion-files/main/data/aa_p21_LTR_20_b1_w20.ion"
  targets <- data.frame(
    id    = c("compound_1", "compound_2", "compound_3"),
    mz    = c(260.1034, 317.1250, 188.0818),
    rt    = c(2.383, 1.903, 3.060),
    range = c(0.05, 0.05, 0.05)
  )
  options <- find_targets_options(
    eic        = calculate_eic_options(from = 1.5, to = 3.5),
    find_peaks = find_peaks_options(min_intensity = 1000),
    threads    = 2L
  )

  sample <- parse_ion(url)
  peaks <- find_targets(sample, targets, options = options)

  for (index in seq_len(nrow(peaks))) {
    peak <- peaks[index, ]
    line <- sprintf("\%s: rt \%.3f  intensity \%.0f  integral \%.0f",
                    targets$id[index], peak$rt, peak$intensity, peak$integral)
    cat(line, "\\n", sep = "")
  }
}
}
}
\seealso{
\code{\link{find_targets_options}()}, \code{\link{find_transitions}()}
}
