from traits.api import (
    Dict,
    HasTraits,
    Instance,
    String,
    Tuple,
)

from summary import Summary

GLOBAL_TYPES = {"performance", "field"}
LOCAL_TYPES = {
    "regions",
    "aquifers",
    "wells",
    "completions",
    "groups",
    "blocks",
    "cross_region_flows",
}


class DataManager(HasTraits):
    """Class that holds a collection of summary data."""

    # Tuple is needed by the DataSelector, so I used it instead of a List
    selected_names = Tuple()

    # actual summary data mapped to a name
    summary_data = Dict(String, Instance(Summary))

    # extracted datetime array per name
    dates = Dict()

    def add_summary(self, summary, name):
        """Add new piece of summary data to the collection from a file."""
        self.summary_data[name] = summary
        self.dates[name] = summary.dates

    def names(self):
        """Return all loaded summary names."""
        return list(self.summary_data.keys())

    def unload_summary(self, names):
        """Delete unnecessary data."""
        for n in names:
            del self.summary_data[n]
            del self.dates[n]

    def common_keys(self):
        """Keys common to all selected summaries."""
        names = self.selected_names
        if len(names) == 0:
            return None

        result = {}
        for t in GLOBAL_TYPES | LOCAL_TYPES:
            keys = getattr(self.summary_data[names[0]], t).keys()
            for n in names[1:]:
                keys &= getattr(self.summary_data[n], t).keys()

            if len(keys) > 0:
                result[t] = keys

        return result

    def get(self, kw_type, kw_name, kw_loc=None):
        """Get a specific keyword data for selected summaries."""
        result = {}
        for n in self.selected_names:
            all_data = getattr(self.summary_data[n], kw_type)
            if kw_loc is None:
                data = all_data[kw_name]
            else:
                data = all_data[(kw_name, kw_loc)]
            result[n] = data

        return result
