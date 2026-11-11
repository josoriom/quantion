\name{calculate_baseline_options}
\alias{calculate_baseline_options}
\title{Settings for calculate_baseline}
\description{
airPLS baseline settings for calculate_baseline.
}
\usage{
calculate_baseline_options(lambda = 20, max_iterations = 4L)
}
\arguments{
  \item{lambda}{Smoothness. Larger = smoother.}
  \item{max_iterations}{Maximum iterations.}
}
\value{
An options object for \code{\link{calculate_baseline}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#CalculateBaselineOptions}
}
\examples{
calculate_baseline_options(lambda = 50)
}
\seealso{
\code{\link{calculate_baseline}()}
}
