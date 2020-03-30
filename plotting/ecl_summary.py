from traits.api import Array, Date, Dict, HasTraits, Int, List, String, Tuple


class EclSummaryVector(HasTraits):

    # physical units
    units = String()

    # time series data
    data = Array()


class EclSummary(HasTraits):

    # Simulation start date
    start_date = Date()

    # Time data
    timing_data = Dict(String, EclSummaryVector)

    # Performance data
    perf_data = Dict(String, EclSummaryVector)

    # Field data
    field_data = Dict(String, EclSummaryVector)

    # Well data
    well_data = Dict(String, Dict(String, EclSummaryVector))

    # Well completion data
    wellcomp_data = Dict(Tuple(String, int), Dict(String, EclSummaryVector))

    # Group data
    group_data = Dict(String, Dict(String, EclSummaryVector))

    # Cell data
    cell_data = Dict(Int, Dict(String, EclSummaryVector))

    # Region data
    region_data = Dict(Int, Dict(String, EclSummaryVector))
