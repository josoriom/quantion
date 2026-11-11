import ctypes
from typing import TYPE_CHECKING, Any, Dict, List, Optional

if TYPE_CHECKING:
    from quantion._bridge import _ABI


class Metadata:
    def _get_metadata(self) -> List[Dict[str, str]]:
        raise NotImplementedError

    def get_keys(self) -> List[str]:
        """The keys stored in the model file, in file order."""
        return [row["key"] for row in self._get_metadata()]

    def get_values(self) -> List[str]:
        """The values stored in the model file, in the same order as get_keys."""
        return [row["value"] for row in self._get_metadata()]

    def get_value(self, key: str) -> Optional[str]:
        """The text stored under key in the model file, or None if it is missing."""
        return next((row["value"] for row in self._get_metadata() if row["key"] == key), None)


class Model(Metadata):
    def __init__(self, handle: ctypes.c_void_p, abi: "_ABI") -> None:
        self._handle: Optional[ctypes.c_void_p] = handle
        self._abi = abi
        self._metadata: Optional[List[Dict[str, str]]] = None

    @property
    def handle(self) -> Optional[ctypes.c_void_p]:
        return self._handle

    def _get_metadata(self) -> List[Dict[str, str]]:
        if self._metadata is None:
            from quantion.api import _get_model_metadata

            self._metadata = _get_model_metadata(self)
        return self._metadata

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
            self._abi.model_free(self._handle)
            self._handle = None

    def __del__(self) -> None:
        self.dispose()

    def __enter__(self) -> "Model":
        return self

    def __exit__(self, *_: Any) -> None:
        self.dispose()
