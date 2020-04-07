import os

from ipyfilechooser import FileChooser
import ipywidgets as wg
import traitlets as tts

from data_manager import DataManager, NEED_LOCATION

TYPE_TO_BUTTON_VALUE = {
    "perf": "P",
    "field": "F",
    "regions": "R",
    "aquifers": "A",
    "wells": "W",
    "completions": "C",
    "groups": "G",
    "cells": "B",
}


class DataSelector(tts.HasTraits):
    # an object that we view
    data_manager = tts.Instance(DataManager).tag(sync=True)

    fc = tts.Instance(
        FileChooser,
        kw=dict(
            use_dir_icons=True,
            show_hidden=False,
            title="<b>Select an .mpk file to open</b>",
        ),
    ).tag(sync=True)

    file_selector = tts.Instance(
        wg.SelectMultiple,
        kw=dict(
            options=[],
            value=[],
            rows=5,
            description="Open Files:",
            disabled=False,
            layout=wg.Layout(width="95%"),
        ),
    ).tag(sync=True)

    type_selector = tts.Instance(
        wg.ToggleButtons,
        kw=dict(
            options=[(v, k) for (k, v) in TYPE_TO_BUTTON_VALUE.items()],
            description="Type:",
            disabled=True,
            style={"button_width": "29px"},
        ),
    ).tag(sync=True)

    loc_selector = tts.Instance(
        wg.Dropdown,
        kw=dict(
            options=[],
            description="Location:",
            disabled=True,
            layout=wg.Layout(width="95%"),
        ),
    ).tag(sync=True)

    kw_selector = tts.Instance(
        wg.Dropdown,
        kw=dict(
            options=[],
            description="Keyword:",
            disabled=True,
            layout=wg.Layout(width="95%"),
        ),
    ).tag(sync=True)

    use_all_kws = tts.Instance(
        wg.Checkbox, kw=dict(value=False, description="List keywords from all files")
    ).tag(sync=True)

    # dummy trait to signal that data needs to be plotted
    request_plot = tts.Int(0).tag(sync=True)

    # cached values of location and kw selectors
    _cached_locs = tts.Dict().tag(sync=True)
    _cached_kws = tts.Dict().tag(sync=True)

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

        # setup observers
        # Note: `_select` is the Select button of the FileChooser class
        self.fc._select.on_click(self._file_opened)
        self.file_selector.observe(self._file_selected, names="value")
        self.type_selector.observe(self._type_selected, names="value")
        self.loc_selector.observe(self._loc_selected, names="value")
        self.kw_selector.observe(self._kw_selected, names="value")

    def get_selections(self):
        return self.type_selector.value, self.loc_selector.value, self.kw_selector.value

    # Private event handlers
    def _file_opened(self, change):
        """Add a newly open file to the data manager and to the list of files."""
        self.data_manager.add_summary(self.fc.selected)
        self.file_selector.options = [
            (os.path.basename(p), p) for p in self.data_manager.file_paths()
        ]

    def _file_selected(self, change):
        """Compute all the common keys and populate the type selector options."""

        # first we let the data manager know
        self.data_manager.selected_paths = self.file_selector.value

        # then we reset the selection cache
        self._cached_locs = {}
        self._cached_kws = {}

        # now we can update the selector widgets
        self.type_selector.disabled = False
        self.type_selector.options = [
            (TYPE_TO_BUTTON_VALUE[k], k) for k in self.data_manager.common_keys
        ]

    def _type_selected(self, change):
        """Populate the location and keyword selectors options."""
        if change["new"] in NEED_LOCATION:
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
        if cur_type in NEED_LOCATION:
            cur_loc = self.loc_selector.value
            self._cached_kws[cur_loc] = selection
        else:
            self._cached_kws[cur_type] = selection

        self.request_plot += 1

    # Private methods
    def _update_selector(self, selector):
        """Update selectors and trigger plotting"""
        selector.disabled = False
        cur_type = self.type_selector.value
        cur_type_keys = self.data_manager.common_keys[cur_type]

        if selector == self.loc_selector:
            observer = self._loc_selected
            common_keys = cur_type_keys
            cached_value = self._cached_locs.get(cur_type)
        else:  # a kw selector
            observer = self._kw_selected
            # we need to inspect both types and locations
            if cur_type in NEED_LOCATION:
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
