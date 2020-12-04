#include "eclair_ffi.rs.h"

#include "ImGuiFileBrowser.h"
#include <Mahi/Gui.hpp>

using namespace mahi::gui;
using namespace mahi::util;

std::string to_string(const ItemQualifier &q);
rust::Vec<TimeSeries> get_item_values(const ItemId &item_id,
                                      const rust::Box<SummaryManager> &manager);
std::string item_name(const ItemId &item_id);
std::tuple<double, double> time_range(const rust::Vec<TimeStamps> &times);
std::tuple<double, double> data_range(const rust::Vec<TimeSeries> &data);

class EclairApp : public Application {
public:
  EclairApp() : Application(800, 600, "Eclair"), manager(make_manager()) {
    ImGui::DisableViewports();
    ImGui::EnableDocking();

    ImPlotStyle& style = ImPlot::GetStyle();
    style.LineWeight = 2.0;

    on_file_drop.connect(this, &EclairApp::file_drop_handler);
  }

  void file_drop_handler(const std::vector<std::string> &paths) {
    for (auto& path: paths) {
      manager->add_from_files(path, "");
    }
    items_dirty = true;
  }

  void update() override {
    // Window menu.
    bool open = false;
    if (ImGui::BeginMainMenuBar()) {
      if (ImGui::BeginMenu("File")) {
        if (ImGui::MenuItem("Add from file")) {
          open = true;
        }
        ImGui::MenuItem("Add from network");
        ImGui::Separator();
        if (ImGui::MenuItem("Quit")) {
        }
        ImGui::EndMenu();
      }
      ImGui::EndMainMenuBar();
    }

    if (open) {
      ImGui::OpenPopup("Open File");
    }

    if (file_dialog.showFileDialog(
            "Open File", imgui_addons::ImGuiFileBrowser::DialogMode::OPEN,
            ImVec2(700, 310), ".SMSPEC")) {
      manager->add_from_files(file_dialog.selected_path, "");
      items_dirty = true;
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
    ImGui::SetNextWindowDockID(dockspaceID, ImGuiCond_FirstUseEver);
    ImGui::Begin("Data");

    if (manager->length() > 0) {
      if (ImGui::CollapsingHeader("Sources", ImGuiTreeNodeFlags_DefaultOpen)) {
        for (int i = 0; i < manager->length(); i++) {
          auto name = manager->summary_name(i);
          ImGui::TextUnformatted(name.data(), name.data() + name.size());
        }
      }
      if (ImGui::CollapsingHeader("Items", ImGuiTreeNodeFlags_DefaultOpen)) {
        if (items_dirty) {
          item_ids = manager->all_item_ids();
          items_dirty = false;
        }

        static ImGuiTableFlags flags =
            ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
            ImGuiTableFlags_ScrollY | ImGuiTableFlags_ColumnsWidthFixed;

        if (ImGui::BeginTable("##table1", 4, flags)) {
          ImGui::TableSetupScrollFreeze(0, 1);
          ImGui::TableSetupColumn("#");
          ImGui::TableSetupColumn("Name");
          //          ImGui::TableSetupColumn("Type");
          ImGui::TableSetupColumn("Well/Group");
          ImGui::TableSetupColumn("Index");
          ImGui::TableHeadersRow();

          int i = 0;
          for (const auto &item_id : item_ids) {
            const bool item_is_selected = (selection == i);
            ImGui::TableNextRow();
            ImGui::TableNextColumn();
            char label[32];
            sprintf(label, "%02d", i);
            if (ImGui::Selectable(label, item_is_selected,
                                  ImGuiSelectableFlags_SpanAllColumns,
                                  ImVec2(0, 0))) {
              selection = i;
            }
            if (ImGui::BeginDragDropSource(ImGuiDragDropFlags_None)) {
              ImGui::SetDragDropPayload("DND_PLOT", &i, sizeof(int));
              ImGui::TextUnformatted(label);
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
            ImGui::Text("%d", item_id.index);
            i++;
          }
          ImGui::EndTable();
        }
      }
    }

    ImGui::End();

    ImGui::SetNextWindowDockID(dockspaceID, ImGuiCond_FirstUseEver);
    ImGui::Begin("Chart");

    static std::optional<std::string> y_label_str = std::nullopt;
    static std::optional<rust::Vec<TimeStamps>> time = std::nullopt;
    static std::optional<rust::Vec<TimeSeries>> data = std::nullopt;
    const double adj_ratio = 0.02;
    static double min_time, max_time;
    static double min_data, max_data;

    if (is_plot_dirty) {
      y_label_str = item_name(item_ids[plotted_item_row]);

      time = manager->unix_time();
      data = get_item_values(item_ids[plotted_item_row], manager);

      std::tie(min_time, max_time) = time_range(time.value());
      std::tie(min_data, max_data) = data_range(data.value());
    }

    const char *x_label = (plotted_item_row == -1) ? nullptr : "Date";
    const char *y_label =
        (plotted_item_row == -1) ? nullptr : y_label_str.value().c_str();

    if (plotted_item_row != -1) {
      auto dx = adj_ratio * (max_time - min_time);
      auto dy = adj_ratio * (max_data - min_data);
      dy = (dy == 0) ? 0.5 : dy;
      ImPlot::SetNextPlotLimits(
          min_time - dx, max_time + dx, min_data - dy, max_data + dy,
          is_plot_dirty ? ImGuiCond_Always : ImGuiCond_Once);
    }

    if (ImPlot::BeginPlot(
            "##DND", x_label, y_label,
            ImVec2(ImGui::GetWindowWidth(), ImGui::GetWindowHeight() * 0.92f),
            ImPlotFlags_NoMousePos, ImPlotAxisFlags_Time)) {
      if (plotted_item_row != -1) {
        for (int s = 0; s < data.value().size(); ++s) {
          const auto &d = data.value()[s];
          if (!d.values.empty()) {
            const auto &t = time.value()[s];
            const auto line_name = std::string(manager->summary_name(s));
            ImPlot::PlotLine(line_name.data(), t.values.data(), d.values.data(),
                             t.values.size());
          }
        }
        is_plot_dirty = false;
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

    ImGui::SetNextWindowDockID(dockspaceID, ImGuiCond_FirstUseEver);
  }

private:
  rust::Box<SummaryManager> manager;
  imgui_addons::ImGuiFileBrowser file_dialog;
  bool items_dirty = true;
  rust::Vec<ItemId> item_ids;

  int plotted_item_row = -1;
  bool is_plot_dirty = false;
};

int main() {
  EclairApp app;
  app.run();
  return 0;
}
