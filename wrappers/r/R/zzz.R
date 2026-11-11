.onLoad <- function(libname, pkgname) {
  library.dynam("quantion", pkgname, libname)
  .Call("C_bind_rust", NULL, PACKAGE = "quantion")
}

.onUnload <- function(libpath) {
  gc()
  try(.Call("C_unbind_rust", NULL, PACKAGE = "quantion"), silent = TRUE)
  library.dynam.unload("quantion", libpath)
}
