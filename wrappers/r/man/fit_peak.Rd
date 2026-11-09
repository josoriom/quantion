\name{fit_peak}
\alias{fit_peak}
\title{Fit a curve to a peak}
\description{
Fit a Gaussian or EMG curve to the peak near rt. rt says which peak; its apex intensity is the starting point unless you force one.
}
\usage{
fit_peak(x, y, rt, options = fit_peak_options())
}
\arguments{
  \item{x}{Retention times, ascending. At least 3 points. Type: \code{numeric}.}
  \item{y}{Intensities, same length as x. Type: \code{numeric}.}
  \item{rt}{Expected retention time of the peak. Type: \code{numeric}.}
  \item{options}{Settings from \code{\link{fit_peak_options}()}: \code{shape = "EMG"}, \code{intensity = 0}.
    \href{https://phenological.github.io/quantion/?language=r#FitPeakOptions}{FitPeakOptions documentation}
  }
}
\value{
The fitted curve, or none when no peak is found or the fit fails. Type: \code{PeakParameters list | NULL}. Fields: \code{height}, \code{center}, \code{fwhm}, \code{tail}, \code{r2}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#fit_peak}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  url <- "https://raw.githubusercontent.com/phenological/ion-files/main/data/aa_p21_LTR_20_b1_w20.ion"
  sample <- parse_ion(url)
  eic <- calculate_eic(sample, 260.103, options = calculate_eic_options(from = 2, to = 3))
  params <- fit_peak(eic$x, eic$y, rt = 2.383)
  gaussian <- fit_peak(eic$x, eic$y, rt = 2.383, options = fit_peak_options(shape = "GAUSSIAN"))
  print(params)
  print(gaussian)
}
}
}
\seealso{
\code{\link{fit_peak_options}()}, \code{\link{draw_peaks}()}
}
