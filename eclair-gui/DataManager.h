#ifndef ECLAIR_GUI_DATAMANAGER_H
#define ECLAIR_GUI_DATAMANAGER_H

#include "eclair_ffi.rs.h"
#include <imgui.h>

#include <vector>

namespace eclair {
class DataManager {
public:
  DataManager() : manager(make_manager()) {}

  void draw();

  struct PlotData {
    const rust::Slice<const int64_t> x;
    const rust::Slice<const float> y;
  };

  [[nodiscard]] PlotData plot_data(size_t summary_index, int index) const;

  [[nodiscard]] bool names_equal(int index1,int index2) const;

  [[nodiscard]] std::string_view item_name(int index) const;

  [[nodiscard]] std::string item_name_and_location(int index) const;

  [[nodiscard]] std::string item_full_name(int summary_index, int index) const;

  void add_from_files(const std::string &path) {
    manager->add_from_files(path, "");
    item_ids = manager->all_item_ids();
  }

  void add_from_files(const std::vector<std::string> &paths) {
    for (const auto &path : paths) {
      manager->add_from_files(path, "");
    }
    item_ids = manager->all_item_ids();
  }

  void add_from_network(const std::string &server, int port) {
    manager->add_from_network(server, port, "eclair", "");
    item_ids = manager->all_item_ids();
  }

  // Refresh the time data.
  bool refresh();

  [[nodiscard]] bool empty() const { return manager->length() == 0; }

  [[nodiscard]] size_t size() const { return manager->length(); }

private:
  [[nodiscard]] ItemId item(int index) const { return item_ids[index]; }

  // Item filter that combines name, well/group and index filters together.
  [[nodiscard]] bool filter(const ItemId &item_id) const;

  rust::Box<SummaryManager> manager;
  rust::Vec<ItemId> item_ids;

  // Data filtering
  ImGuiTextFilter name_filter;
  ImGuiTextFilter wg_filter;
  ImGuiTextFilter idx_filter;
  ImGuiTextFilter *filters[3] = {&name_filter, &wg_filter, &idx_filter};
};

} // namespace eclair

#endif // ECLAIR_GUI_DATAMANAGER_H
