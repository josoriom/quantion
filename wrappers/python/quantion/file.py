import ctypes
from typing import TYPE_CHECKING, Any, Optional

if TYPE_CHECKING:
    from quantion._bridge import _ABI


class SampleFile:
    """SampleFile · Inputs

    An opened sample from parse_ion or parse_mzml. Free it with dispose when you are done.

    Documentation: https://phenological.github.io/quantion/?language=python#SampleFile
    """
    def __init__(self, handle: ctypes.c_void_p, abi: "_ABI") -> None:
        self._handle: Optional[ctypes.c_void_p] = handle
        self._abi = abi
        self._is_ion = False

    @property
    def handle(self) -> Optional[ctypes.c_void_p]:
        return self._handle

    def dispose(self) -> None:
        """dispose · Models

        Free the memory held by a sample, model or warp model.

        Warning:
            Not needed in normal use: the library frees a sample, model or warp model by itself once it is no longer used. When processing many samples, dispose each one after using it, so its memory is freed right away instead of piling up.

        Returns:
            Nothing.

        Documentation: https://phenological.github.io/quantion/?language=python#dispose
        """
        if self._handle is not None:
            self._abi.free_mzml(self._handle)
            self._handle = None

    def __del__(self) -> None:
        self.dispose()

    def __enter__(self) -> "SampleFile":
        return self

    def __exit__(self, *_: Any) -> None:
        self.dispose()

    def __repr__(self) -> str:
        if self._handle is None:
            return "<SampleFile disposed>"
        address = ctypes.cast(self._handle, ctypes.c_void_p).value
        return f"<SampleFile handle=0x{address:x}>"
