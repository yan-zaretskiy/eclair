import os

import ipywidgets as wg
import traitlets as tts

from data_manager import DataManager, GLOBAL_TYPES, LOCAL_TYPES


TYPE_TO_STRING = {
    "perf": "Performance",
    "field": "Field",
    "regions": "Regions",
    "aquifers": "Aquifers",
    "wells": "Wells",
    "completions": "Completions",
    "groups": "Groups",
    "cells": "Cells",
}


class DataSelector(tts.HasTraits):
    # an object that we view
    data_manager = tts.Instance(DataManager).tag(sync=True)

    # List of loaded file paths
    file_selector = tts.Instance(
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

    # list of availbale keyword names for a given type/location
    kw_selector = tts.Instance(
        wg.Dropdown,
        kw=dict(
            options=[],
            description="Keyword:",
            disabled=True,
            layout=wg.Layout(width="auto"),
        ),
    ).tag(sync=True)

    # currently unused - option to list either union or intersection of keywords
    # across all loaded files
    use_all_kws = tts.Instance(
        wg.Checkbox, kw=dict(value=False, description="List keywords from all files")
    ).tag(sync=True)

    # dummy trait to signal that data needs to be re-plotted
    request_plot = tts.Int(0).tag(sync=True)

    # cached values of location and kw selectors
    _cached_locs = tts.Dict().tag(sync=True)
    _cached_kws = tts.Dict().tag(sync=True)

    def __init__(self, data_manager, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.data_manager = data_manager

        self.file_selector.options = [
            (os.path.basename(p), p) for p in self.data_manager.file_paths()
        ]

        # setup observers
        self.file_selector.observe(self._file_selected, names="value")
        self.type_selector.observe(self._type_selected, names="value")
        self.loc_selector.observe(self._loc_selected, names="value")
        self.kw_selector.observe(self._kw_selected, names="value")

    def selections(self):
        """Currently selected options."""
        return self.type_selector.value, self.loc_selector.value, self.kw_selector.value

    def view(self):
        """ipywidget to display the DataSelector in a Jupyter Notebook"""
        return wg.VBox(
            [
                self.file_selector,
                self.type_selector,
                self.loc_selector,
                self.kw_selector,
                # self.use_all_kws,
            ],
            layout=wg.Layout(height="auto", width="350px"),
        )

    # Private event handlers
    def _file_selected(self, change):
        """Compute all the common keys and populate the type selector options."""

        # first we let the data manager know
        self.data_manager.selected_paths = self.file_selector.value

        # then we reset the selection cache
        self._cached_locs = {}
        self._cached_kws = {}

        # now we can update the selector widgets
        self.type_selector.disabled = False
        if self.data_manager.common_keys is not None:
            self.type_selector.options = [
                (TYPE_TO_STRING[k], k) for k in self.data_manager.common_keys
            ]
        else:
            # clear and disable all selection widgets
            self.type_selector.options = []
            self.type_selector.disabled = True
            self.kw_selector.options = []
            self.kw_selector.disabled = True
            self.loc_selector.options = []
            self.loc_selector.disabled = True

    def _type_selected(self, change):
        """Populate the location and keyword selectors options."""
        if change["new"] in LOCAL_TYPES:
            self.kw_selector.disabled = True
            self._update_selector(selector=self.loc_selector)
        else:
            self.loc_selector.disabled = True
            self.loc_selector.options = []
            self._update_selector(selector=self.kw_selector)

    def _loc_selected(self, change):
        """Populate the keyword selector options."""
        selection = change["new"]
        self._cached_locs[self.type_selector.value] = selection
        self.kw_selector.disabled = False
        self._update_selector(selector=self.kw_selector)

    def _kw_selected(self, change):
        """Cache the selection and trigger plotting."""
        selection = change["new"]
        cur_type = self.type_selector.value
        if cur_type in LOCAL_TYPES:
            cur_loc = self.loc_selector.value
            self._cached_kws[cur_loc] = selection
        else:
            self._cached_kws[cur_type] = selection

        self.request_plot += 1

    # Private methods
    def _update_selector(self, selector):
        """Update selectors and trigger plotting"""
        if self.data_manager.common_keys is None:
            return

        selector.disabled = False
        cur_type = self.type_selector.value
        cur_type_keys = self.data_manager.common_keys[cur_type]

        if selector == self.loc_selector:
            common_keys = cur_type_keys
            cached_value = self._cached_locs.get(cur_type)
        else:  # a kw selector
            # we need to inspect both types and locations
            if cur_type in LOCAL_TYPES:
                cur_loc = self.loc_selector.value
                common_keys = cur_type_keys[cur_loc]
                cached_value = self._cached_kws.get(cur_loc)
            else:
                common_keys = cur_type_keys
                cached_value = self._cached_kws.get(cur_type)

        selector.options = sorted(
            [(str(k), k) for k in common_keys], key=lambda x: x[1]
        )

        if cached_value is not None:
            selector.value = cached_value

        self.request_plot += 1
