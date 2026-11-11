\name{calculate_eic_options}
\alias{calculate_eic_options}
\title{Settings for an EIC}
\description{
Which scans are read and how wide the m/z window is. Used by calculate_eic, and as the eic group of find_targets, find_features and match_features. The wider of the ppm and absolute m/z windows is used.
}
\usage{
calculate_eic_options(from = 0, to = Inf, ppm_tolerance = 20, mz_tolerance = 0.005,
                      time_unit = "MINUTES")
}
\arguments{
  \item{from}{Start of the retention-time range read.}
  \item{to}{End of the retention-time range read. The default reads to the end of the run.}
  \item{ppm_tolerance}{Tolerance in ppm.}
  \item{mz_tolerance}{Absolute tolerance in Da.}
  \item{time_unit}{Unit of from, to and rt values given.}
}
\value{
An options object for \code{\link{calculate_eic}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#CalculateEicOptions}
}
\examples{
calculate_eic_options(from = 0.5, to = 5, ppm_tolerance = 10)
}
\seealso{
\code{\link{calculate_eic}()}
}
