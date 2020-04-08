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
        self.fig.layout.width = 1200
        self.fig.layout.height = 800
        self.fig.layout.margin = {"l": 0, "r": 0, "b": 0}
        self.fig.layout.title.x = 0.5
        self.fig.layout.legend = {"orientation": "h"}

        for p in self.data_manager.file_paths():
            dates = self.data_manager.dates[p]
            self.fig.add_scatter(
                x=dates, visible=False, showlegend=True, name=os.path.basename(p)
            )

    def update_traces(self, kw_type, kw_loc, kw_name):
        values = self.data_manager.selected_data(kw_type, kw_loc, kw_name)

        with self.fig.batch_update():
            if len(values) > 0:
                if kw_loc is not None:
                    title = f"{kw_name} @ {kw_loc}"
                else:
                    title = f"{kw_name}"
                self.fig.layout.title.text = title
                self.fig.layout.xaxis.title = "Date"
                self.fig.layout.yaxis.title = f"{kw_name} [{values[0]['unit']}]"
                for trace, v in zip(self.fig.data, values):
                    trace.y = v["values"]
                    trace.visible = True
            else:
                self.fig.layout.title.text = ""
                self.fig.layout.xaxis.title = ""
                self.fig.layout.yaxis.title = ""
                for trace in self.fig.data:
                    trace.y = []
                    trace.visible = False
