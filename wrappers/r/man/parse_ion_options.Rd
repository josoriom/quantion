\name{parse_ion_options}
\alias{parse_ion_options}
\title{Settings for parse_ion}
\description{
Settings for parse_ion.
}
\usage{
parse_ion_options(max_cache_size = 0)
}
\arguments{
  \item{max_cache_size}{Cache size in bytes. 0 = no limit.}
}
\value{
An options object for \code{\link{parse_ion}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#ParseIonOptions}
}
\examples{
parse_ion_options(max_cache_size = 64e6)
}
\seealso{
\code{\link{parse_ion}()}
}
