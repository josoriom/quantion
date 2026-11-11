import ctypes
from typing import TYPE_CHECKING, Any, Dict, List, Optional, Sequence

from quantion.model import Metadata

if TYPE_CHECKING:
    from quantion._bridge import _ABI
    from quantion.file import SampleFile


class WarpModel(Metadata):
    def __init__(
        self, handle: ctypes.c_void_p, abi: "_ABI", metadata: List[Dict[str, str]]
    ) -> None:
        self._handle: Optional[ctypes.c_void_p] = handle
        self._abi = abi
        self._metadata = metadata

    @property
    def handle(self) -> Optional[ctypes.c_void_p]:
        return self._handle

    def _get_metadata(self) -> List[Dict[str, str]]:
        return self._metadata

    def predict_rts(self, file: "SampleFile", targets: Sequence[Dict[str, Any]]) -> List[Dict[str, float]]:
        """predict_rts · Models

        Predict where each target elutes in one sample.

        Args:
            file: The sample to read, from parse_ion or parse_mzml.
            targets: Targets with mz and reference rt. id and range are optional and not used. Each Target has id, mz, rt, range. See Target: https://phenological.github.io/quantion/?language=python#Target
                Example: target: Target = {"id": "compound_1", "mz": 260.103, "rt": 2.383, "range": 0.05}

        Returns:
            One PredictedRt per target, in the same order.

        Documentation: https://phenological.github.io/quantion/?language=python#predict_rts
        """
        from quantion.api import _predict_rts

        return _predict_rts(self, file, targets)

    def calibrate(
        self,
        anchor_samples: Sequence["SampleFile"],
        target_lists: Sequence[Sequence[Dict[str, Any]]],
    ) -> List[Dict[str, Any]]:
        """calibrate · Models

        Turn the retention times seen in anchor samples (LTRs, QCs) into targets for predict_rts. Each id gets the median rt over the anchors.

        Args:
            anchor_samples: Anchor samples.
            target_lists: For each anchor sample, the targets seen in it with their observed rt. Each Target has id, mz, rt, range. See Target: https://phenological.github.io/quantion/?language=python#Target
                Example: target: Target = {"id": "compound_1", "mz": 260.103, "rt": 2.383, "range": 0.05}

        Returns:
            One Target per id with the median rt.

        Documentation: https://phenological.github.io/quantion/?language=python#calibrate
        """
        from quantion.api import _calibrate_rts

        return _calibrate_rts(self, anchor_samples, target_lists)

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
            self._abi.warp_model_free(self._handle)
            self._handle = None

    def __del__(self) -> None:
        self.dispose()

    def __enter__(self) -> "WarpModel":
        return self

    def __exit__(self, *_: Any) -> None:
        self.dispose()
