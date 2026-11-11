\name{get_scans_options}
\alias{get_scans_options}
\title{Settings for get_scans}
\description{
Settings for get_scans.
}
\usage{
get_scans_options(level = 1L, time_unit = "MINUTES",
                  arrays = list(list(key = "mz", accession = "MS:1000514"),
                                list(key = "intensity", accession = "MS:1000515")))
}
\arguments{
  \item{level}{MS level. 0 = every level.}
  \item{time_unit}{Unit of rt in the query.}
  \item{arrays}{Which arrays to read from each scan, as keys with accessions. Every other field of Scan always comes back. See \href{https://phenological.github.io/quantion/?language=r#ScanArray}{ScanArray}.}
}
\value{
An options object for \code{\link{get_scans}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#GetScansOptions}
}
\examples{
get_scans_options(level = 2L, time_unit = "SECONDS")
}
\seealso{
\code{\link{get_scans}()}
}
