#ifndef ECLAIR_GUI_ECLAIRAPP_H
#define ECLAIR_GUI_ECLAIRAPP_H

#include "eclair_ffi.rs.h"

#include <ImGuiFileBrowser.h>
#include <Mahi/Gui.hpp>
#include <Mahi/Util.hpp>

#include <tuple>

using namespace mahi::gui;
using namespace mahi::util;

namespace eclair {
class EclairApp : public Application {
public:
  EclairApp();

  void update() override;

private:
  std::tuple<bool, bool> draw_main_menu();

  void draw_chart_window();
  void draw_plot_tooltip(const rust::Vec<TimeStamps> &time,
                         const rust::Vec<TimeSeries> &data);

  void file_drop_handler(const std::vector<std::string> &paths);

  // Item filter that combines name, well/group and index filters together.
  [[nodiscard]] bool PassFilter(const ItemId &item_id) const;

  rust::Box<SummaryManager> manager;
  imgui_addons::ImGuiFileBrowser file_dialog;
  bool items_dirty = true;
  rust::Vec<ItemId> item_ids;

  int plotted_item_row = -1;
  bool is_plot_dirty = false;

  /* Data filtering */
  ImGuiTextFilter name_filter;
  ImGuiTextFilter wg_filter;
  ImGuiTextFilter idx_filter;
};

} // namespace eclair

#endif // ECLAIR_GUI_ECLAIRAPP_H
