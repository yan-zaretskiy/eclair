import os

import ipywidgets as wg
import traitlets as tts

from data_manager import DataManager, LOCAL_TYPES


class DataSelector(tts.HasTraits):
    # an object that we view
    data_manager = tts.Instance(DataManager).tag(sync=True)

    # List of loaded file paths
    name_selector = tts.Instance(
        wg.SelectMultiple,
        kw=dict(
            options=[],
            value=[],
            rows=5,
            description="Open Files:",
            disabled=False,
            layout=wg.Layout(width="auto"),
        ),
    ).tag(sync=True)

    # Type of a keyword to plot
    type_selector = tts.Instance(
        wg.RadioButtons,
        kw=dict(
            options=[],
            description="Type:",
            disabled=True,
            layout=wg.Layout(width="auto"),
        ),
    ).tag(sync=True)

    # Keyword location for those that need it (well, group and so on)
    loc_selector = tts.Instance(
        wg.Dropdown,
        kw=dict(
            options=[],
            description="Location:",
            disabled=True,
            layout=wg.Layout(width="auto"),
        ),
    ).tag(sync=True)

    # list of available keyword names for a given type/location
    kw_selector = tts.Instance(
        wg.Dropdown,
        kw=dict(
            options=[],
            description="Keyword:",
            disabled=True,
            layout=wg.Layout(width="auto"),
        ),
    ).tag(sync=True)

    # reference summary for plotting deltas
    ref_selector = tts.Instance(
        wg.Dropdown,
        kw=dict(
            options=[],
            description="Reference:",
            disabled=True,
            layout=wg.Layout(width="auto"),
        ),
    ).tag(sync=True)

    # plot values or their deltas wrt to a reference
    plot_deltas = tts.Instance(
        wg.Checkbox, kw=dict(value=False, description="Plot deltas")
    ).tag(sync=True)

    # dummy trait to signal that data needs to be re-plotted
    request_plot = tts.Int(0).tag(sync=True)

    def __init__(self, data_manager, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.data_manager = data_manager

        self.name_selector.options = [n for n in self.data_manager.summary_data.keys()]

        # setup observers
        self.name_selector.observe(self._name_selected, names="value")
        self.type_selector.observe(self._type_selected, names="value")
        self.loc_selector.observe(self._loc_selected, names="value")
        self.kw_selector.observe(self._kw_selected, names="value")
        self.ref_selector.observe(self._request_plot, names="value")
        self.plot_deltas.observe(self._request_plot, names="value")

    def selections(self):
        """Currently selected options."""
        return (
            self.type_selector.value,
            self.loc_selector.value,
            self.kw_selector.value,
            self.ref_selector.value,
            self.plot_deltas.value,
        )

    # Private event handlers
    def _name_selected(self, change):
        """Compute all the common keys and populate the type selector options."""

        # first we let the data manager know
        self.data_manager.selected_names = self.name_selector.value

        # now we can update the selector widgets
        self.type_selector.disabled = False
        ck = self.data_manager.common_keys()
        if ck is not None:
            self.type_selector.options = sorted(
                [(k.capitalize(), k) for k in ck], key=lambda x: x[0],
            )
            self._propagate_type_selection(self.type_selector.value)

            self.ref_selector.disabled = False
            self.ref_selector.options = [v for v in self.name_selector.value]
        else:
            # clear and disable all selection widgets
            self.type_selector.options = []
            self.type_selector.disabled = True
            self.kw_selector.options = []
            self.kw_selector.disabled = True
            self.loc_selector.options = []
            self.loc_selector.disabled = True
            self.ref_selector.options = []
            self.ref_selector.disabled = True

    def _type_selected(self, change):
        """Populate the location and keyword selectors options."""
        self._propagate_type_selection(change["new"])

    def _loc_selected(self, change):
        """Populate the keyword selector options."""
        self._update_selector(selector=self.kw_selector)

    def _kw_selected(self, change):
        """Trigger plotting."""
        self.request_plot += 1

    # Private methods
    def _propagate_type_selection(self, value):
        """Populate the location and keyword selectors options."""
        if value in LOCAL_TYPES:
            self._update_selector(selector=self.loc_selector)
        else:
            self.loc_selector.disabled = True
            self.loc_selector.options = []
            self._update_selector(selector=self.kw_selector)

    def _update_selector(self, selector):
        """Update selectors and trigger plotting"""
        ck = self.data_manager.common_keys()
        if ck is None:
            return

        selector.disabled = False
        cur_type = self.type_selector.value
        cur_type_keys = ck[cur_type]

        if selector == self.loc_selector:
            selector.options = sorted(
                list(set((str(loc), loc) for kw, loc in cur_type_keys)),
                key=lambda x: x[1],
            )

        # we need to inspect both types and locations
        if cur_type in LOCAL_TYPES:
            self.kw_selector.options = sorted(
                [
                    (str(kw), kw)
                    for kw, loc in cur_type_keys
                    if loc == self.loc_selector.value
                ],
                key=lambda x: x[1],
            )
        else:
            self.kw_selector.options = sorted(
                [(str(kw), kw) for kw in cur_type_keys], key=lambda x: x[1]
            )

        self.request_plot += 1

    def _request_plot(self, change):
        """So that I can make this an observer."""
        self.request_plot += 1
