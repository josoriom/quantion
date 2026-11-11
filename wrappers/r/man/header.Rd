\name{header}
\alias{header}
\title{An HTTP header for an upload}
\description{
One HTTP header sent with an upload in \code{\link{write_ion}()}. A line break, or Host, Content-Length or Transfer-Encoding, gives an \code{invalid_argument} error.
}
\usage{
header(name, value)
}
\arguments{
  \item{name}{Header name, such as \code{"Authorization"}.}
  \item{value}{Header value.}
}
\value{
A header object.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#Header}
}
\examples{
header("Authorization", "Bearer <token>")
}
\seealso{
\code{\link{write_ion_options}()}
}
