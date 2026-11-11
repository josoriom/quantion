\name{draw_peaks_options}
\alias{draw_peaks_options}
\title{Settings for draw_peaks}
\description{
Settings for draw_peaks.
}
\usage{
draw_peaks_options(from = 0, to = 0, points = 1000L)
}
\arguments{
  \item{from}{Start of the chromatogram. With to, 0 and 0 = fit the range around all the peaks.}
  \item{to}{End of the chromatogram.}
  \item{points}{Number of points drawn between from and to.}
}
\value{
An options object for \code{\link{draw_peaks}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#DrawPeaksOptions}
}
\examples{
draw_peaks_options(from = 2, to = 3, points = 2000L)
}
\seealso{
\code{\link{draw_peaks}()}
}
