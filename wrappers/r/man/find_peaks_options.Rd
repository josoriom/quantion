\name{find_peaks_options}
\alias{find_peaks_options}
\title{Peak-picking settings}
\description{
Peak-picking settings. Every field is optional.
}
\usage{
find_peaks_options(min_integral = NA, min_intensity = 500, min_peak_width_points = 3L,
                   noise = NA, auto_noise = TRUE, auto_baseline = TRUE, lambda = 20,
                   max_iterations = 4L, allow_overlap = FALSE, min_snr = 2, min_r2 = 0,
                   shape = "EMG", kernel_size = 0L)
}
\arguments{
  \item{min_integral}{Drop peaks with a smaller area. Off when not given.}
  \item{min_intensity}{Drop peaks with a lower apex.}
  \item{min_peak_width_points}{Drop peaks with fewer points.}
  \item{noise}{Fixed noise level, used when auto_noise is false.}
  \item{auto_noise}{Estimate the noise from the trace.}
  \item{auto_baseline}{Subtract an airPLS baseline first.}
  \item{lambda}{airPLS smoothness.}
  \item{max_iterations}{Maximum airPLS iterations.}
  \item{allow_overlap}{Let neighbouring peaks share points.}
  \item{min_snr}{Minimum signal-to-noise ratio.}
  \item{min_r2}{Minimum fit quality of the peak shape (0-1). 0 keeps every peak.}
  \item{shape}{Curve used for the fit.}
  \item{kernel_size}{Smoothing window in points. 0 = no smoothing.}
}
\value{
An options object for \code{\link{find_peaks}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#FindPeaksOptions}
}
\examples{
find_peaks_options(min_intensity = 1000, min_snr = 3)
}
\seealso{
\code{\link{find_peaks}()}
}
