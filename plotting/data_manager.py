import datetime

import msgpack
import numpy as np

from traits.api import Bool, cached_property, HasTraits, Dict, Property, Tuple


def decode_start_date(obj):
    if "start_date" in obj:
        dt = obj["start_date"]
        obj["start_date"] = np.datetime64(
            datetime.datetime(*dt[2::-1], hour=dt[3], minute=dt[4], microsecond=dt[5])
        )
    return obj


def ext_hook(code, data):
    """Read code-2 data as float32 numpy arrays."""
    if code == 2:
        return np.frombuffer(data, dtype=">f4")
    return msgpack.ExtType(code, data)


def load_summary(path):
    """Load summary data from a file."""
    with open(path, "rb") as fp:
        raw_bytes = fp.read()

    return msgpack.unpackb(
        raw_bytes,
        strict_map_key=False,
        use_list=False,
        ext_hook=ext_hook,
        object_hook=decode_start_date,
    )


def get_dates(summary):
    """Extract a list of dates from a summary."""
    return summary["start_date"] + (summary["time"]["TIME"]["values"] * 84600).astype(
        "timedelta64[s]"
    )


def common_keys(summaries, union=False):
    """Recursively extract common keys from a list of summaries."""
    summaries = [s for s in summaries if isinstance(s, dict) and "unit" not in s]
    if len(summaries) == 0:
        return None

    keys = set(summaries[0].keys())
    for s in summaries[1:]:
        if union:
            keys |= s.keys()
        else:
            keys &= s.keys()
    keys -= {"start_date", "time"}

    return {k: common_keys([s.get(k) for s in summaries]) for k in keys}


GLOBAL_TYPES = {"performance", "field"}

LOCAL_TYPES = {"regions", "aquifers", "wells", "completions", "groups", "blocks"}


class DataManager(HasTraits):
    """Class that holds a collection of summary data."""

    # Tuple is needed by the DataSelector, so I used it instead of a List
    selected_paths = Tuple()

    # actual summary data mapped to a file path
    summary_data = Dict()

    # extracted datetime array per file path
    dates = Dict()

    # currently untested
    all_keywords = Bool(False)

    # currently untested
    common_keys = Property(depends_on=["summary_data, add_keywords, selected_paths"])

    def add_summary(self, path):
        """Add new piece of summary data to the collection from a file."""
        if path is None:
            return

        self.summary_data[path] = load_summary(path)
        self.dates[path] = get_dates(self.summary_data[path])

    def file_paths(self):
        """Return all file paths for the loaded summary data."""
        return self.summary_data.keys()

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

        if kw_type in LOCAL_TYPES:
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

    @classmethod
    def build(cls, paths):
        dm = cls()
        for p in paths:
            dm.add_summary(p)
        return dm
