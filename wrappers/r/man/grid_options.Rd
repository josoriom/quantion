\name{grid_options}
\alias{grid_options}
\title{The m/z grid searched for features}
\description{
The m/z grid that find_features and match_features search.
}
\usage{
grid_options(start = 40, end = 1000, step_size = 0.005)
}
\arguments{
  \item{start}{Lowest m/z.}
  \item{end}{Highest m/z.}
  \item{step_size}{Step between grid m/z values (Da).}
}
\value{
An options object for \code{\link{find_features_options}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#GridOptions}
}
\examples{
grid_options(start = 100, end = 600)
}
\seealso{
\code{\link{find_features_options}()}
}
