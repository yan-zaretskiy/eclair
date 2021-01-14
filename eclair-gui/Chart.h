#ifndef ECLAIR_GUI_CHART_H
#define ECLAIR_GUI_CHART_H

#include <cassert>
#include "eclair_ffi.rs.h"

#include <array>
#include <string>
#include <vector>

namespace eclair {

class DataManager;

class Chart {
public:
  explicit Chart(DataManager &data_manager) : data_manager(data_manager) {
    reset();
  }

  // Draw the chart to the ImGui window.
  void draw();

  // Reset the chart.
  void reset();

private:
  // Empty if no axis has any data.
  bool is_empty();

  bool add_item_to_axis(int item_index, int axis, bool append);

  void refresh_axes_labels_and_limits();

  DataManager &data_manager;

  static constexpr int N_AXES = 2;
  static constexpr int N_ITEMS = 4;

  template <typename T>
  using AxesCollection = std::array<std::array<T, N_ITEMS>, N_AXES>;

  // y labels
  std::array<std::string, N_AXES> y_labels{};

  // item names
  AxesCollection<std::vector<std::string>> item_names;

  // There are 2 axes per chart and at most 4 items per axis.
  AxesCollection<int> item_ids{};

  bool tooltip = true;

  bool needs_refit = true;

  bool was_d_released = true;
};

} // namespace eclair

#endif // ECLAIR_GUI_CHART_H
