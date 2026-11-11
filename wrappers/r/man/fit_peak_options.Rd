\name{fit_peak_options}
\alias{fit_peak_options}
\title{Settings for fit_peak}
\description{
Settings for fit_peak.
}
\usage{
fit_peak_options(shape = "EMG", intensity = 0)
}
\arguments{
  \item{shape}{Curve to fit.}
  \item{intensity}{Apex intensity to start the fit from. 0 = take it from the peak at rt.}
}
\value{
An options object for \code{\link{fit_peak}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#FitPeakOptions}
}
\examples{
fit_peak_options(shape = "GAUSSIAN")
}
\seealso{
\code{\link{fit_peak}()}
}
