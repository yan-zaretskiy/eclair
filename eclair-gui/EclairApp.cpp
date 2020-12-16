#include "EclairApp.h"
#include "FilteredVector.h"

#include "implot_internal.h"

namespace eclair {

// utils.cpp declarations
std::string to_string(const ItemQualifier &q);
rust::Vec<TimeSeries> get_item_values(const ItemId &item_id,
                                      const rust::Box<SummaryManager> &manager);
std::string item_name(const ItemId &item_id);
std::tuple<double, double> time_range(const rust::Vec<TimeStamps> &times);
std::tuple<double, double> data_range(const rust::Vec<TimeSeries> &data);

EclairApp::EclairApp()
    : Application(800, 600, "Eclair"), manager(make_manager()) {
  // Logging for the Rust backend.
  enable_logger();

  ImGui::DisableViewports();
  ImGui::EnableDocking();

  // ImPlot styling settings.
  ImPlotStyle &style = ImPlot::GetStyle();
  style.LineWeight = 2.0;
  style.FitPadding = ImVec2(0.05f, 0.05f);
  style.PlotPadding = ImVec2(0, 0);

  // Event handlers.
  on_file_drop.connect(this, &EclairApp::file_drop_handler);
}

void EclairApp::file_drop_handler(const std::vector<std::string> &paths) {
  for (auto &path : paths) {
    manager->add_from_files(path, "");
  }
  items_dirty = true;
}

void EclairApp::update() {
  // Window menu.
  const auto [add_from_file, add_from_network] = draw_main_menu();

  // User requested to add Summary data from a file.
  if (add_from_file) {
    ImGui::OpenPopup("Open File");
  }

  if (file_dialog.showFileDialog(
          "Open File", imgui_addons::ImGuiFileBrowser::DialogMode::OPEN,
          ImVec2(700, 310), ".SMSPEC")) {
    manager->add_from_files(file_dialog.selected_path, "");
    items_dirty = true;
  }

  // User requested to add Summary data from a network stream.
  if (add_from_network) {
    ImGui::OpenPopup("Add From Network");
    ImVec2 center = ImGui::GetMainViewport()->GetCenter();
    ImGui::SetNextWindowPos(center, ImGuiCond_Appearing, ImVec2(0.5f, 0.5f));
  }

  if (ImGui::BeginPopupModal("Add From Network", nullptr,
                             ImGuiWindowFlags_AlwaysAutoResize)) {
    static char host[128] = "";
    static int port = 23120;

    ImGui::Text("Enter the network stream address.");
    ImGui::SetNextItemWidth(35.0f);
    ImGui::LabelText("##host_label", "Host:");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(150.0f);
    ImGui::InputText("##host", host, IM_ARRAYSIZE(host));
    ImGui::SameLine();
    ImGui::SetNextItemWidth(35.0f);
    ImGui::LabelText("##port_label", "Port:");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(100.0f);
    ImGui::InputInt("##port", &port, 0);

    ImGui::Dummy(ImVec2(0.0f, 20.0f));
    ImGui::Indent(230);
    if (ImGui::Button("OK", ImVec2(50, 0))) {
      manager->add_from_network(host, port, "eclair", "");
      ImGui::CloseCurrentPopup();
    }
    ImGui::SetItemDefaultFocus();
    ImGui::SameLine();
    if (ImGui::Button("Cancel", ImVec2(50, 0))) {
      ImGui::CloseCurrentPopup();
    }
    ImGui::Unindent(230);

    ImGui::EndPopup();
  }

  // Main dock-space.
  ImGuiViewport *viewport = ImGui::GetMainViewport();
  ImGui::SetNextWindowPos(viewport->GetWorkPos());
  ImGui::SetNextWindowSize(viewport->GetWorkSize());
  ImGui::SetNextWindowViewport(viewport->ID);
  ImGui::PushStyleVar(ImGuiStyleVar_WindowRounding, 0.0f);
  ImGui::PushStyleVar(ImGuiStyleVar_WindowBorderSize, 0.0f);

  ImGuiWindowFlags windowFlags =
      ImGuiWindowFlags_NoDocking | ImGuiWindowFlags_NoTitleBar |
      ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoMove |
      ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoBringToFrontOnFocus |
      ImGuiWindowFlags_NoNavFocus;

  static ImGuiID dockspaceID = 0;
  if (ImGui::Begin("DockSpace", nullptr, windowFlags)) {
    ImGui::PopStyleVar(2);
    dockspaceID = ImGui::GetID("MainDock");
    ImGui::DockSpace(dockspaceID);
  }
  ImGui::End();

  // Data window.
  int selection = -1;
  int to_be_removed = -1;

  ImGui::SetNextWindowDockID(dockspaceID, ImGuiCond_FirstUseEver);
  ImGui::Begin("Data");

  if (manager->length() > 0) {
    if (ImGui::CollapsingHeader("Sources", ImGuiTreeNodeFlags_DefaultOpen)) {
      for (int i = 0; i < manager->length(); i++) {
        auto name = manager->summary_name(i);
        std::string label = fmt::format(ICON_FA_TIMES "##{}", i);
        if (ImGui::SmallButton(label.c_str())) {
          to_be_removed = i;
        }
        ImGui::SameLine();
        ImGui::TextUnformatted(name.data(), name.data() + name.size());
      }
    }

    if (to_be_removed != -1) {
      manager->remove(to_be_removed);
      is_plot_dirty = true;
      items_dirty = true;
      if (manager->length() == 0) {
        plotted_item_row = -1;
      }
    }
  }

  if (manager->length() > 0) {
    if (ImGui::CollapsingHeader("Items", ImGuiTreeNodeFlags_DefaultOpen)) {
      if (items_dirty) {
        item_ids = manager->all_item_ids();
        items_dirty = false;
      }

      static ImGuiTableFlags flags = ImGuiTableFlags_Borders |
                                     ImGuiTableFlags_RowBg |
                                     ImGuiTableFlags_ScrollY;

      const int COLUMNS_COUNT = 4;

      if (ImGui::BeginTable("##table1", COLUMNS_COUNT, flags)) {
        ImGui::TableSetupScrollFreeze(0, 2);
        ImGui::TableSetupColumn("#", ImGuiTableColumnFlags_WidthFixed, 30.0f);
        ImGui::TableSetupColumn("Name");
        ImGui::TableSetupColumn("Well/Group");
        ImGui::TableSetupColumn("Index");

        // header row
        static ImGuiTextFilter *filters[3] = {&name_filter, &wg_filter,
                                              &idx_filter};
        ImGui::TableNextRow(ImGuiTableRowFlags_Headers);
        for (int column = 0; column < COLUMNS_COUNT; column++) {
          ImGui::TableSetColumnIndex(column);
          const char *column_name = ImGui::TableGetColumnName(
              column); // Retrieve name passed to TableSetupColumn()
          ImGui::PushID(column);
          ImGui::TableHeader(column_name);
          if (column > 0) {
            filters[column - 1]->Draw("##filter",
                                      ImGui::GetContentRegionAvail().x);
          }
          ImGui::PopID();
        }

        // data rows
        FilteredVector filtered_items(item_ids, [this](auto &&item) -> bool {
          return PassFilter(std::forward<decltype(item)>(item));
        });

        ImGuiListClipper clipper;
        clipper.Begin(filtered_items.size());
        while (clipper.Step()) {
          for (int row = clipper.DisplayStart; row < clipper.DisplayEnd;
               row++) {
            int real_row = filtered_items.original_idx(row);
            const bool item_is_selected = (selection == real_row);
            const auto &item_id = filtered_items[row];

            ImGui::TableNextRow();
            ImGui::TableNextColumn();
            std::string label = std::to_string(real_row);
            if (ImGui::Selectable(label.c_str(), item_is_selected,
                                  ImGuiSelectableFlags_SpanAllColumns,
                                  ImVec2(0, 0))) {
              selection = real_row;
            }
            if (ImGui::BeginDragDropSource(ImGuiDragDropFlags_None)) {
              ImGui::SetDragDropPayload("DND_PLOT", &real_row, sizeof(int));
              ImGui::TextUnformatted(label.c_str());
              ImGui::EndDragDropSource();
            }

            ImGui::TableNextColumn();
            ImGui::TextUnformatted(item_id.name.data(),
                                   item_id.name.data() + item_id.name.length());
            ImGui::TableNextColumn();
            ImGui::TextUnformatted(item_id.wg_name.data(),
                                   item_id.wg_name.data() +
                                       item_id.wg_name.length());
            ImGui::TableNextColumn();
            if (item_id.index != -1) {
              ImGui::Text("%d", item_id.index);
            }
          }
        }
        ImGui::EndTable();
      }
    }
  }
  ImGui::End();

  ImGui::SetNextWindowDockID(dockspaceID, ImGuiCond_FirstUseEver);
  ImGui::ShowDemoWindow();
  draw_chart_window();
}

std::tuple<bool, bool> EclairApp::draw_main_menu() {
  bool add_from_file = false;
  bool add_from_network = false;

  if (ImGui::BeginMainMenuBar()) {
    if (ImGui::BeginMenu("File")) {
      if (ImGui::MenuItem("Add from file")) {
        add_from_file = true;
      }
      if (ImGui::MenuItem("Add from network")) {
        add_from_network = true;
      }
      ImGui::Separator();
      if (ImGui::MenuItem("Quit")) {
        quit();
      }
      ImGui::EndMenu();
    }
    ImGui::EndMainMenuBar();
  }

  return {add_from_file, add_from_network};
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

void EclairApp::draw_chart_window() {
  ImGui::Begin("Chart", nullptr, ImGuiWindowFlags_NoScrollbar);

  static std::string y_label_str;
  static rust::Vec<TimeStamps> time;
  static rust::Vec<TimeSeries> data;
  static std::vector<std::string> line_names;

  static double min_time, max_time;
  static double min_data, max_data;

  bool has_new_data = manager->refresh();

  if ((is_plot_dirty || has_new_data) && plotted_item_row != -1) {
    y_label_str = item_name(item_ids[plotted_item_row]);

    time = manager->unix_time();
    data = get_item_values(item_ids[plotted_item_row], manager);

    std::tie(min_time, max_time) = time_range(time);
    std::tie(min_data, max_data) = data_range(data);
  }

  const char *x_label = (plotted_item_row == -1) ? nullptr : "Date";
  const char *y_label =
      (plotted_item_row == -1) ? nullptr : y_label_str.c_str();

  if (is_plot_dirty) {
    ImPlot::FitNextPlotAxes(true, true, false, false);
  }
  static bool tooltip = true;
  ImGui::Checkbox("Show Tooltip", &tooltip);
  if (ImPlot::BeginPlot(
          "##DND", x_label, y_label,
          ImVec2(ImGui::GetWindowWidth(),
                 ImGui::GetWindowHeight() - ImGui::GetCursorPosY()),
          ImPlotFlags_NoMousePos, ImPlotAxisFlags_Time)) {
    if (plotted_item_row != -1) {
      for (int s = 0; s < data.size(); ++s) {
        const auto &d = data[s].values;
        if (!d.empty()) {
          const auto &t = time[s].values;
          const auto &name = line_names.emplace_back(manager->summary_name(s));
          ImPlot::PlotLine(name.data(), t.data(), d.data(), t.size());
        }
      }
      is_plot_dirty = false;

      // custom tooltip
      if (tooltip && ImPlot::IsPlotHovered()) {
        draw_plot_tooltip(time, data);
      }
    }

    // make our plot a drag and drop target
    if (ImGui::BeginDragDropTarget()) {
      if (const ImGuiPayload *payload =
              ImGui::AcceptDragDropPayload("DND_PLOT")) {
        int i = *(int *)payload->Data;
        plotted_item_row = i;
        is_plot_dirty = true;
      }
      ImGui::EndDragDropTarget();
    }
    ImPlot::EndPlot();
  }
  ImGui::End();
}

void EclairApp::draw_plot_tooltip(const rust::Vec<TimeStamps> &time,
                                  const rust::Vec<TimeSeries> &data) {
  ImDrawList *draw_list = ImPlot::GetPlotDrawList();
  ImPlotPoint mouse = ImPlot::GetPlotMousePos();

  float tool_l = ImPlot::PlotToPixels(mouse).x - 1.0f;
  float tool_r = ImPlot::PlotToPixels(mouse).x + 1.0f;
  float tool_t = ImPlot::GetPlotPos().y;
  float tool_b = tool_t + ImPlot::GetPlotSize().y;

  // Thin vertical line to indicate current x position.
  ImPlot::PushPlotClipRect();
  draw_list->AddRectFilled(ImVec2(tool_l, tool_t), ImVec2(tool_r, tool_b),
                           IM_COL32(128, 128, 128, 64));
  ImPlot::PopPlotClipRect();

  ImGui::BeginTooltip();
  bool first_time = true;
  const float txt_ht = ImGui::GetTextLineHeight();
  auto date_size = ImGui::CalcTextSize("2020-01-01 00:00:00 ");
  for (int s = 0; s < data.size(); ++s) {
    const auto &d = data[s].values;
    if (!d.empty()) {
      const auto &t = time[s].values;
      auto idx = binary_search(t.data(), t.size(), mouse.x);
      if (idx != -1) {
        if (first_time) {
          ImGui::Indent(txt_ht);
          ImGui::Text("Date/Time");
          ImGui::Unindent(txt_ht);
          ImGui::SameLine();
          ImGui::Indent(txt_ht + date_size.x);
          ImGui::Text("Value");
          ImGui::Unindent(txt_ht + date_size.x);
          first_time = false;
        }
        char buff[32];
        ImPlot::FormatDateTime(ImPlotTime::FromDouble(t[idx]), buff, 32,
                               ImPlotDateTimeFmt{ImPlotDateFmt_DayMoYr,
                                                 ImPlotTimeFmt_HrMinS, true,
                                                 true});
        auto curr_cursor = ImGui::GetCursorScreenPos();
        auto color =
            ImColor(ImPlot::GetCurrentPlot()->Items.GetByIndex(s)->Color);
        ImGui::GetWindowDrawList()->AddRectFilled(
            curr_cursor + ImVec2(2, 2),
            curr_cursor + ImVec2(txt_ht - 2, txt_ht - 2), color, 1);
        ImGui::Indent(txt_ht);
        ImGui::Text("%s %g", buff, d[idx]);
        ImGui::Unindent(txt_ht);
      }
    }
  }

  ImGui::EndTooltip();
}

bool EclairApp::PassFilter(const ItemId &item_id) const {
  bool pass_name_filter = name_filter.PassFilter(
      item_id.name.data(), item_id.name.data() + item_id.name.size());

  bool pass_wg_filter = wg_filter.PassFilter(
      item_id.wg_name.data(), item_id.wg_name.data() + item_id.wg_name.size());

  std::string idx_str =
      (item_id.index == -1) ? "" : fmt::format("{}", item_id.index);

  bool pass_idx_filter =
      idx_filter.PassFilter(idx_str.data(), idx_str.data() + idx_str.size());

  return pass_name_filter && pass_wg_filter && pass_idx_filter;
}

} // namespace eclair