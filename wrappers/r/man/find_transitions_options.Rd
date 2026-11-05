\name{find_transitions_options}
\alias{find_transitions_options}
\title{Settings for find_transitions}
\description{
Settings for find_transitions.
}
\usage{
find_transitions_options(find_peaks = find_peaks_options(), threads = 1L)
}
\arguments{
  \item{find_peaks}{Settings for peak picking. See \code{\link{find_peaks_options}()}.}
  \item{threads}{Number of threads.}
}
\value{
An options object for \code{\link{find_transitions}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#FindTransitionsOptions}
}
\examples{
find_transitions_options(threads = 2L)
}
\seealso{
\code{\link{find_transitions}()}
}
