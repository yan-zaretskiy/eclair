import datetime
import warnings

import msgpack
import numpy as np
import pandas as pd

from traits.api import (
    Bool,
    cached_property,
    HasTraits,
    Dict,
    Property,
    Tuple,
    Unicode,
)


def ext_hook(code, data):
    """Read code-2 data as float32 numpy arrays."""
    if code == 2:
        return np.frombuffer(data, dtype=np.float32)
    return msgpack.ExtType(code, data)


def load_summary(path):
    """Load summary data from a file."""
    with open(path, "rb") as fp:
        raw_bytes = fp.read()

    return msgpack.unpackb(
        raw_bytes, strict_map_key=False, use_list=False, ext_hook=ext_hook
    )


def get_dates(summary):
    """Extract a list of dates from a summary."""
    time_data = summary["time"]
    if "YEAR" in time_data and "MONTH" in time_data and "DAY" in time_data:
        # easy path first
        return [
            datetime.datetime(y, m, d)
            for (y, m, d) in zip(
                time_data["YEAR"]["values"],
                time_data["MONTH"]["values"],
                time_data["DAY"]["values"],
            )
        ]
    else:
        # do all the work ourselves
        start_date = datetime.datetime(*reversed(summary["start_date"]))
        return [
            start_date + datetime.timedelta(seconds=int(d * 86400))
            for d in time_data["TIME"]["values"]
        ]


def common_keys(summaries, union=False):
    """Recursively extract common keys from a list of summaries."""
    summaries = [s for s in summaries if isinstance(s, dict) and "unit" not in s]
    if len(summaries) == 0:
        return None

    keys = set(summaries[0].keys())
    for s in summaries[1:]:
        if union:
            keys &= s.keys()
        else:
            keys |= s.keys()
    keys -= {"start_date", "time"}

    return {k: common_keys([s.get(k) for s in summaries]) for k in keys}


NEED_LOCATION = {"regions", "aquifers", "wells", "completions", "groups", "cells"}


def convert_to_date(array, fmt="%m-%d-%Y"):
    # If array is a np.ndarray with type == np.datetime64, the array can be
    # returned as such. If it is an np.ndarray of dtype 'object' then conversion
    # to string is tried according to the fmt parameter.

    if isinstance(array, np.ndarray) and np.issubdtype(array.dtype, np.datetime64):
        # no need to perform any conversion in this case
        return array
    elif isinstance(array, list) or (
        isinstance(array, np.ndarray) and array.dtype == "object"
    ):
        return_value = []
        # Pandas to_datetime handles all the cases where the passed in
        # data could be any of the combinations of
        #            [list, nparray] X [python_datetime, np.datetime]
        # Because of the coerce=True flag, any non-compatible datetime type
        # will be converted to pd.NaT. By this comparison, we can figure
        # out if it is date castable or not.
        if len(np.shape(array)) == 2:
            for elem in array:
                temp_val = pd.to_datetime(
                    elem, errors="coerce", infer_datetime_format=True
                )
                temp_val = elem if (temp_val[0] == np.datetime64("NaT")) else temp_val
                return_value.append(temp_val)
        elif isinstance(array, list):
            temp_val = pd.to_datetime(
                array, errors="coerce", infer_datetime_format=True
            )
            return_value = array if (temp_val[0] == np.datetime64("NaT")) else temp_val
        else:
            temp_val = pd.to_datetime(
                array, errors="coerce", infer_datetime_format=True
            )
            temp_val = array if (temp_val[0] == np.datetime64("NaT")) else temp_val
            return_value = temp_val
        return return_value
    elif isinstance(array, np.ndarray):
        warnings.warn("Array could not be converted into a date")
        return array


class DataManager(HasTraits):
    """Class that holds a collection of summary data."""

    all_keywords = Bool(False)
    selected_paths = Tuple(Unicode)

    summary_data = Dict()
    dates = Dict()

    common_keys = Property(depends_on=["summary_data, add_keywords, selected_paths"])

    def add_summary(self, path):
        """Add new piece of summary data to the collection from a file."""
        if path is None:
            return

        self.summary_data[path] = load_summary(path)
        self.dates[path] = get_dates(self.summary_data[path])

    def get_dates(self, path):
        """Get a vector of dates for a given summary."""
        return self.dates[path]

    def file_paths(self):
        """Return all file paths for the loaded summary data."""
        return list(self.summary_data.keys())

    def unload_files(self, paths):
        """Delete unnecessary data."""
        for p in paths:
            del self.summary_data[p]
            del self.dates[p]

    def get_data(self, path, kw_type, kw_loc, kw_name):
        """Given the three keys, grabs the corresponding data vector."""
        if path not in self.summary_data:
            return None
        res = self.summary_data[path]

        if kw_type not in res:
            return None
        res = res[kw_type]

        if kw_type in NEED_LOCATION:
            res = res.get(kw_loc)
        if res is None:
            return None

        return res.get(kw_name)

    @cached_property
    def _get_common_keys(self):
        """Either a union of an intersection of all keys present in summary dictionaries."""
        return common_keys(
            [self.summary_data[p] for p in self.selected_paths], self.all_keywords
        )
