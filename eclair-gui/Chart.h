#ifndef ECLAIR_GUI_CHART_H
#define ECLAIR_GUI_CHART_H

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
    print_rows();
  }

  // Draw the chart to the ImGui window.
  void draw();

  // Reset the chart.
  void reset();

private:
  // Empty if no axis has any data.
  bool is_empty();

  bool add_item_to_axis(int item_index, int axis, bool append);

  static constexpr int N_AXES = 2;
  static constexpr int N_ITEMS = 4;

  template <typename T>
  using AxesCollection = std::array<std::array<T, N_ITEMS>, N_AXES>;

  DataManager &data_manager;

  bool needs_refit = true;

  // y labels
  std::array<std::string, N_AXES> y_labels{};

  // There are 2 axes per chart and at most 4 items per axis.
  AxesCollection<int> items_ids{};

  bool tooltip = true;
};

} // namespace eclair

#endif // ECLAIR_GUI_CHART_H
