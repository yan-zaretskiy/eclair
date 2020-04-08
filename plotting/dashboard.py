from data_selector import DataSelector
from data_plotter import DataPlotter

from ipywidgets import HBox


def make_dashboard(data_manager):
    selector = DataSelector(data_manager=data_manager)
    plotter = DataPlotter(data_manager=data_manager)

    def replot(change):
        kw_type = selector.type_selector.value
        kw_loc = selector.loc_selector.value
        kw_name = selector.kw_selector.value
        plotter.update_traces(kw_type, kw_loc, kw_name)

    selector.observe(replot, names=["request_plot"])

    return HBox([selector.view(), plotter.fig])
