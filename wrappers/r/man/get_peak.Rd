\name{get_peak}
\alias{get_peak}
\title{Find the one peak near a retention time}
\description{
Find the one peak near rt. With a range, only rt +/- range is searched; without one, the whole trace is searched and the peak is chosen by its size and its distance to rt.
}
\usage{
get_peak(x, y, rt, options = get_peak_options())
}
\arguments{
  \item{x}{Retention times, ascending. At least 3 points. Type: \code{numeric}.}
  \item{y}{Intensities, same length as x. Type: \code{numeric}.}
  \item{rt}{Expected retention time of the peak. Type: \code{numeric}.}
  \item{options}{Settings from \code{\link{get_peak_options}()}: \code{range = 0}, \code{find_peaks}.
    \href{https://phenological.github.io/quantion/?language=r#GetPeakOptions}{GetPeakOptions documentation}
  }
}
\value{
The best peak near rt. No peak gives a Peak with every field 0. Type: \code{Peak list}. Fields: \code{rt}, \code{from}, \code{to}, \code{intensity}, \code{integral}, \code{n_points}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#get_peak}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  url <- "https://raw.githubusercontent.com/phenological/ion-files/main/data/aa_p21_LTR_20_b1_w20.ion"
  sample <- parse_ion(url)
  eic <- calculate_eic(sample, 260.103, options = calculate_eic_options(from = 2, to = 3))
  peak <- get_peak(eic$x, eic$y, rt = 2.383, options = get_peak_options(range = 0.05))
  anywhere <- get_peak(eic$x, eic$y, rt = 2.383)
  print(peak)
  print(anywhere)
}
}
}
\seealso{
\code{\link{get_peak_options}()}, \code{\link{find_peaks}()}
}
