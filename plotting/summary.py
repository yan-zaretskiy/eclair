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
    completions = Dict(Tuple(String, Tuple(String, Int)), SummaryRecord)

    # Group data, resolved by group name
    groups = Dict(Tuple(String, String), SummaryRecord)

    # Block data, resolved by cell index
    blocks = Dict(Tuple(String, Int), SummaryRecord)

    # Aquifer data, resolved by aquifer index
    aquifers = Dict(Tuple(String, Int), SummaryRecord)

    # Region-to-region flows, resolved by two region indices
    cross_region_flows = Dict(Tuple(String, Tuple(Int, Int)), SummaryRecord)

    dates = Property(depends_on=["start_date", "time"])

    well_names = Property(depends_on="wells")

    @cached_property
    def _get_dates(self):
        return self.start_date + (self.time["TIME"].values * 86400).astype(
            "timedelta64[s]"
        )

    @cached_property
    def _get_well_names(self):
        return set(name for (kw, name) in self.wells.keys())


def decode_start_date(obj):
    if "start_date" in obj:
        dt = obj["start_date"]
        second, microsecond = dt[5] // 1_000_000, dt[5] % 1_000_000
        obj["start_date"] = np.datetime64(
            datetime.datetime(
                *dt[2::-1],
                hour=dt[3],
                minute=dt[4],
                second=second,
                microsecond=microsecond,
            )
        )
    return obj


def ext_hook(code, data):
    """Read code-2 data as float32 numpy arrays."""
    if code == 2:
        return np.frombuffer(data, dtype=">f4")
    return msgpack.ExtType(code, data)


def extract_summary(file_or_bytes):
    """Extract summary data from a file or directly from MessagePack bytes."""
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
        name = item["id"]["name"]
        qualifier = item["id"]["qualifier"]
        kind = qualifier["kind"]
        location = qualifier.get("location")
        index = qualifier.get("index")
        data = SummaryRecord(unit=item["unit"], values=item["values"])

        # global
        if kind == "time":
            summary.time[name] = data
        elif kind == "performance":
            summary.performance[name] = data
        elif kind == "field":
            summary.field[name] = data

        # index-based
        elif kind == "aquifer":
            summary.aquifers[(name, index)] = data
        elif kind == "region":
            summary.regions[(name, index)] = data
        elif kind == "block":
            summary.blocks[(name, index)] = data

        # location-based
        elif kind == "well":
            summary.wells[(name, location)] = data
        elif kind == "group":
            summary.groups[(name, location)] = data

        # cross-region and well completion
        elif kind == "completion":
            summary.completions[
                (name, (location, index))
            ] = data
        elif kind == "cross_region_flow":
            summary.cross_region_flows[
                (name, (qualifier["from"], qualifier["to"]))
            ] = data

    return summary
