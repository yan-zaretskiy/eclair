#include "cxx.h"

#include "Chart.h"
#include "DataManager.h"

#include <Mahi/Gui.hpp>
#include <Mahi/Util.hpp>
#include <implot_internal.h>

#include <chrono>
#include <string>

using namespace mahi::gui;
using namespace mahi::util;

namespace eclair {

void Chart::reset() {
  for (auto &axis : items_ids) {
    axis.fill(-1);
  }
  y_labels.fill("");
  needs_refit = true;
}

bool Chart::is_empty() {
  for (auto &axis : items_ids) {
    for (auto &id : axis) {
      if (id != -1) {
        return false;
      }
    }
  }
  return true;
}

bool Chart::add_item_to_axis(int item_index, int axis, bool append) {
  auto &axis_items = items_ids[axis];
  if (append && axis_items[0] != -1) {
    auto empty_it = std::find(std::begin(axis_items), std::end(axis_items), -1);
    if (empty_it != std::end(axis_items) &&
        data_manager.names_equal(item_index, axis_items[0])) {
      *empty_it = item_index;
      // If we could successfully append an item, we need to change the y-label.
      y_labels[axis] = data_manager.item_name(item_index);
      needs_refit = true;
      return true;
    }
    return false;
  } else {
    axis_items[0] = item_index;
    for (int i = 1; i < N_ITEMS; ++i) {
      axis_items[i] = -1;
    }
    y_labels[axis] = data_manager.item_name_and_location(item_index);
    needs_refit = true;
    return true;
  }
}

template <typename T> size_t binary_search(const T *arr, int count, T x) {
  size_t x_lo = 0, x_hi = count - 1;

  while ((x_hi - x_lo) > 1) {
    size_t ix = (x_lo + x_hi) >> 1;
    if (x >= arr[ix]) {
      x_lo = ix;
    } else {
      x_hi = ix;
    }
  }
  return x_lo;
}

// void draw_plot_tooltip(const rust::Vec<TimeStamps> &time,
//                       const rust::Vec<TimeSeries> &data) {
//  ImDrawList *draw_list = ImPlot::GetPlotDrawList();
//  ImPlotPoint mouse = ImPlot::GetPlotMousePos();
//
//  float tool_l = ImPlot::PlotToPixels(mouse).x - 1.0f;
//  float tool_r = ImPlot::PlotToPixels(mouse).x + 1.0f;
//  float tool_t = ImPlot::GetPlotPos().y;
//  float tool_b = tool_t + ImPlot::GetPlotSize().y;
//
//  // Thin vertical line to indicate current x position.
//  ImPlot::PushPlotClipRect();
//  draw_list->AddRectFilled(ImVec2(tool_l, tool_t), ImVec2(tool_r, tool_b),
//                           IM_COL32(128, 128, 128, 64));
//  ImPlot::PopPlotClipRect();
//
//  ImGui::BeginTooltip();
//  bool first_time = true;
//  static const float txt_ht = ImGui::GetTextLineHeight();
//  static const auto date_size = ImGui::CalcTextSize("2020-01-01 00:00:00 ");
//  for (int s = 0; s < data.size(); ++s) {
//    const auto &d = data[s].values;
//    if (!d.empty()) {
//      const auto &t = time[s].values;
//      auto idx = binary_search(t.data(), t.size(), mouse.x);
//      if (idx != -1) {
//        if (first_time) {
//          ImGui::Indent(txt_ht);
//          ImGui::Text("Date/Time");
//          ImGui::Unindent(txt_ht);
//          ImGui::SameLine();
//          ImGui::Indent(txt_ht + date_size.x);
//          ImGui::Text("Value");
//          ImGui::Unindent(txt_ht + date_size.x);
//          first_time = false;
//        }
//        char buff[32];
//        ImPlot::FormatDateTime(ImPlotTime::FromDouble(t[idx]), buff, 32,
//                               ImPlotDateTimeFmt{ImPlotDateFmt_DayMoYr,
//                                                 ImPlotTimeFmt_HrMinS, true,
//                                                 true});
//        auto curr_cursor = ImGui::GetCursorScreenPos();
//        auto color =
//            ImColor(ImPlot::GetCurrentPlot()->Items.GetByIndex(s)->Color);
//        ImGui::GetWindowDrawList()->AddRectFilled(
//            curr_cursor + ImVec2(2, 2),
//            curr_cursor + ImVec2(txt_ht - 2, txt_ht - 2), color, 1);
//        ImGui::Indent(txt_ht);
//        ImGui::Text("%s %g", buff, d[idx]);
//        ImGui::Unindent(txt_ht);
//      }
//    }
//  }
//
//  ImGui::EndTooltip();
//}

ImPlotPoint ToPoint(void *data, int idx) {
  auto *pd = (DataManager::PlotData *)data;
  return {static_cast<double>(pd->x[idx]), pd->y[idx]};
}

bool schedule_deletion(int &id) {
  static bool first_time = true;
  static auto last_deletion = std::chrono::steady_clock::now();
  auto now = std::chrono::steady_clock::now();
  std::chrono::duration<double> diff = now - last_deletion;
  if (!first_time && diff.count() < 0.1) {
    return false;
  } else {
    last_deletion = now;
    first_time = false;
    id = -1;
    return true;
  }
}

void Chart::draw() {
  if (!is_empty() && data_manager.empty()) {
    reset();
  }

  bool empty = is_empty();
  if (empty) {
    ImPlot::SetNextPlotLimitsX(0, 1, ImGuiCond_Always);
    ImPlot::SetNextPlotLimitsY(0, 1, ImGuiCond_Always, 0);
    ImPlot::SetNextPlotLimitsY(0, 1, ImGuiCond_Always, 1);
  }

  if (needs_refit) {
    ImPlot::FitNextPlotAxes(true, true, true, false);
  }
  //  ImGui::Checkbox("Show Tooltip", &tooltip);
  const char *x_label = empty ? nullptr : "Date";
  // This is not correct.
  const char *y_label =
      empty || y_labels[0].empty() ? nullptr : y_labels[0].c_str();
  const char *y2_label =
      empty || y_labels[1].empty() ? nullptr : y_labels[1].c_str();

  if (ImPlot::BeginPlot(
          "##DND", x_label, y_label,
          ImVec2(ImGui::GetWindowWidth(),
                 ImGui::GetWindowHeight() - ImGui::GetCursorPosY()),
          ImPlotFlags_NoMousePos | ImPlotFlags_YAxis2, ImPlotAxisFlags_Time,
          ImPlotFlags_None, ImPlotFlags_None, ImPlotFlags_None, y2_label)) {
    if (!empty) {
      bool deleted_smth = false;
      int counter = 0;
      for (int i = 0; i < N_AXES; ++i) {
        auto &axis = items_ids[i];
        for (auto &id : axis) {
          if (id != -1) {
            for (int s = 0; s < data_manager.size(); ++s) {
              const auto &name = data_manager.item_full_name(s, id);
              auto pd = data_manager.plot_data(s, id);
              ImPlot::SetPlotYAxis(i);
              auto col = ImPlot::GetColormapColor(counter);
              counter += 1;
              ImPlot::PushStyleColor(ImPlotCol_Line, col);
              ImPlot::PlotLineG(name.c_str(), ToPoint, &pd, pd.x.size());
              ImPlot::PopStyleColor();
              if (ImPlot::IsLegendEntryHovered(name.c_str()) &&
                  ImGui::GetIO().KeysDown[GLFW_KEY_D]) {
                // I need to put some time pressure on the deletion, because
                // frame rendering is way faster than a key press.
                deleted_smth = schedule_deletion(id);
              }
            }
          }
        }
      }
      needs_refit = deleted_smth;
      // custom tooltip
      //      if (tooltip && ImPlot::IsPlotHovered()) {
      //        draw_plot_tooltip(time, data);
      //      }
    }

    // make our plot a drag and drop target
    if (ImGui::BeginDragDropTarget()) {
      if (const ImGuiPayload *payload =
              ImGui::AcceptDragDropPayload("DND_PLOT")) {
        bool append = ImGui::GetIO().KeyCtrl;
        int i = *(int *)payload->Data;
        int destination = 0;
        // set specific y-axis if hovered
        for (int y = 0; y < N_AXES; y++) {
          if (ImPlot::IsPlotYAxisHovered(y))
            destination = y;
        }
        add_item_to_axis(i, destination, append);
      }
      ImGui::EndDragDropTarget();
    }
    ImPlot::EndPlot();
  }
}

} // namespace eclair