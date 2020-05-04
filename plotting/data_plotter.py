import os

import numpy as np
import plotly.graph_objects as go
import traitlets as tts

from data_manager import DataManager


class DataPlotter(tts.HasTraits):
    # an object that we view
    data_manager = tts.Instance(DataManager).tag(sync=True)

    # a figure holding a graph
    fig = tts.Instance(go.FigureWidget).tag(sync=True)

    def __init__(self, data_manager, *args, **kwargs):
        super().__init__(*args, **kwargs)

        self.data_manager = data_manager
        self.fig = go.FigureWidget()
        self.fig.layout.margin = {"l": 0, "r": 40, "t": 40, "b": 0}
        self.fig.layout.title.x = 0.5
        self.fig.layout.legend = {"orientation": "h"}

        for name in self.data_manager.summary_data.keys():
            dates = self.data_manager.dates[name]
            self.fig.add_scatter(
                x=dates, visible=False, showlegend=True, name=name, mode="lines",
            )

    def update_traces(self, kw_type, kw_loc, kw_name, reference, plot_deltas):
        """Update all scatter plots when selection changes."""
        dm = self.data_manager
        with self.fig.batch_update():
            if len(dm.selected_names) > 0:
                if kw_loc is not None:
                    title = f"{kw_name} @ {kw_loc}"
                else:
                    title = f"{kw_name}"
                self.fig.layout.title.text = title
                self.fig.layout.xaxis.title = "Date"

                data = dm.get(kw_type, kw_name, kw_loc)
                # first we have to pick a reference data
                x_ref, y_ref = 0.0, 0.0
                if plot_deltas:
                    for name in dm.names():
                        if name == reference:
                            dates_ref = dm.dates[name]
                            x_ref = (dates_ref - dates_ref[0]).astype(np.float32)
                            y_ref = data[name].values

                to_plot = None
                for trace, name in zip(self.fig.data, dm.names()):
                    if name in dm.selected_names:
                        to_plot = data[name]
                        if plot_deltas:
                            # we need to interpolate on a 1D datetime grid
                            dates = dm.dates[name]
                            x = (dates - dates[0]).astype(np.float32)
                            trace.y = (
                                to_plot.values - np.interp(x, x_ref, y_ref)
                            ).astype(np.float32)
                        else:
                            trace.y = to_plot.values.astype(np.float32)
                        trace.visible = True
                    else:
                        trace.y = []
                        trace.visible = False
                if to_plot is not None:
                    self.fig.layout.yaxis.title = f"{kw_name} [{to_plot.unit}]"
            else:
                self.fig.layout.title.text = ""
                self.fig.layout.xaxis.title = ""
                self.fig.layout.yaxis.title = ""
                for trace in self.fig.data:
                    trace.y = []
                    trace.visible = False
