import datetime

import msgpack
import numpy as np

from traits.api import (
    Array,
    cached_property,
    Dict,
    HasTraits,
    Instance,
    Int,
    Property,
    String,
    Tuple,
)


class SummaryRecord(HasTraits):
    unit = String
    values = Array

    def __repr__(self):
        return f'SummaryRecord(unit="{self.unit}",\n values={self.values})'


class Summary(HasTraits):
    # Simulation start date
    start_date = Instance(np.datetime64)

    # Map of region indices to names
    region_names = Dict(Int, String)

    # Time data
    time = Dict(String, SummaryRecord)

    # Performance data
    performance = Dict(String, SummaryRecord)

    # Field data
    field = Dict(String, SummaryRecord)

    # Region data, resolved by index
    regions = Dict(Tuple(String, Int), SummaryRecord)

    # Well data, resolved by well name
    wells = Dict(Tuple(String, String), SummaryRecord)

    # Completion data, resolved by well name and cell index
    completions = Dict(Tuple(String, String, Int), SummaryRecord)

    # Group data, resolved by group name
    groups = Dict(Tuple(String, String), SummaryRecord)

    # Block data, resolved by cell index
    blocks = Dict(Tuple(String, Int), SummaryRecord)

    # Aquifer data, resolved by aquifer index
    aquifers = Dict(Tuple(String, Int), SummaryRecord)

    # Region-to-region flows, resolved by two region indices
    cross_region_flows = Dict(Tuple(String, Int, Int), SummaryRecord)

    dates = Property(depends_on=["start_date", "time"])

    @cached_property
    def _get_dates(self):
        return self.start_date + (self.time["TIME"].values * 86400).astype(
            "timedelta64[s]"
        )


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


def load_summary(file_or_bytes):
    """Load summary data from a file or directly from bytes."""
    if isinstance(file_or_bytes, str):
        with open(file_or_bytes, "rb") as fp:
            raw_bytes = fp.read()
    elif isinstance(file_or_bytes, bytes):
        raw_bytes = file_or_bytes
    else:
        raise TypeError("Input type has to be either a str or bytes.")

    unpacked = msgpack.unpackb(
        raw_bytes,
        strict_map_key=False,
        use_list=False,
        ext_hook=ext_hook,
        object_hook=decode_start_date,
    )

    summary = Summary(start_date=unpacked["start_date"])

    # If we have region names, store them in the summary
    if "region_names" in unpacked:
        summary.region_names = unpacked["region_names"]

    for item in unpacked["items"]:
        item_id = item["id"]
        kind = item_id["kind"]
        data = SummaryRecord(unit=item["unit"], values=item["values"])

        # global
        if kind == "time":
            summary.time[item_id["name"]] = data
        elif kind == "performance":
            summary.performance[item_id["name"]] = data
        elif kind == "field":
            summary.field[item_id["name"]] = data

        # index-based
        elif kind == "aquifer":
            summary.aquifers[(item_id["name"], item_id["index"])] = data
        elif kind == "region":
            summary.regions[(item_id["name"], item_id["index"])] = data
        elif kind == "block":
            summary.blocks[(item_id["name"], item_id["index"])] = data

        # location-based
        elif kind == "well":
            summary.wells[(item_id["name"], item_id["location"])] = data
        elif kind == "group":
            summary.groups[(item_id["name"], item_id["location"])] = data

        # cross-region and well completion
        elif kind == "completion":
            summary.completions[
                (item_id["name"], item_id["location"], item_id["index"])
            ] = data
        elif kind == "cross_region_flow":
            summary.cross_region_flows[
                (item_id["name"], item_id["from"], item_id["to"])
            ] = data

    return summary
