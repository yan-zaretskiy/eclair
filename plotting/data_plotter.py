import os

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
        self.fig.layout.margin = {"l": 0, "r": 0, "t": 40, "b": 0}
        self.fig.layout.title.x = 0.5
        self.fig.layout.legend = {"orientation": "h"}

        for p in self.data_manager.file_paths():
            dates = self.data_manager.dates[p]
            self.fig.add_scatter(
                x=dates, visible=False, showlegend=True, name=os.path.basename(p)
            )

    def update_traces(self, kw_type, kw_loc, kw_name):
        """Update all scatter plots when selection changes."""
        dm = self.data_manager
        with self.fig.batch_update():
            if len(dm.selected_paths) > 0:
                if kw_loc is not None:
                    title = f"{kw_name} @ {kw_loc}"
                else:
                    title = f"{kw_name}"
                self.fig.layout.title.text = title
                self.fig.layout.xaxis.title = "Date"

                value = None
                for trace, path in zip(self.fig.data, dm.file_paths()):
                    if path in dm.selected_paths:
                        value = dm.get_data(path, kw_type, kw_loc, kw_name)
                        trace.y = value["values"]
                        trace.visible = True
                    else:
                        trace.y = []
                        trace.visible = False
                if value is not None:
                    self.fig.layout.yaxis.title = f"{kw_name} [{value['unit']}]"
            else:
                self.fig.layout.title.text = ""
                self.fig.layout.xaxis.title = ""
                self.fig.layout.yaxis.title = ""
                for trace in self.fig.data:
                    trace.y = []
                    trace.visible = False
