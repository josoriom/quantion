suppressPackageStartupMessages(library(quantion))

describe <- function(name) {
  value <- get(name, envir = asNamespace("quantion"))
  if (!is.function(value)) return(sprintf("value %s: %s", name, class(value)[1]))
  arguments <- formals(value)
  parts <- vapply(names(arguments), function(argument) {
    if (identical(arguments[[argument]], quote(expr = ))) return(argument)
    paste0(argument, " = ", paste(deparse(arguments[[argument]], width.cutoff = 500L), collapse = " "))
  }, character(1))
  sprintf("function %s(%s)", name, paste(parts, collapse = ", "))
}

exports <- sort(getNamespaceExports("quantion"))
methods <- sort(unlist(lapply(c("print"), function(generic) {
  found <- utils::methods(generic)
  found[startsWith(found, paste0(generic, ".quantion"))]
})))
lines <- c(vapply(exports, describe, character(1)), paste("method", methods))
writeLines(lines)
