#include "eclair_ffi.rs.h"

#include "ImGuiFileBrowser.h"
#include <Mahi/Gui.hpp>

#include <tuple>

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

    ImPlotStyle &style = ImPlot::GetStyle();
    style.LineWeight = 2.0;
    style.FitPadding = ImVec2(0.05f, 0.05f);

    on_file_drop.connect(this, &EclairApp::file_drop_handler);
  }

  void file_drop_handler(const std::vector<std::string> &paths) {
    for (auto &path : paths) {
      manager->add_from_files(path, "");
    }
    items_dirty = true;
  }

  void update() override {
    // Window menu.
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
        }
        ImGui::EndMenu();
      }
      ImGui::EndMainMenuBar();
    }

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

    if (ImGui::BeginPopupModal("Add From Network", NULL,
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
          if (ImGui::SmallButton(ICON_FA_TIMES)) {
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

          ImGuiListClipper clipper;
          clipper.Begin(item_ids.size());
          while (clipper.Step()) {
            for (int row = clipper.DisplayStart; row < clipper.DisplayEnd;
                 row++) {
              const bool item_is_selected = (selection == row);
              const auto &item_id = item_ids[row];
              ImGui::TableNextRow();
              ImGui::TableNextColumn();
              char label[32];
              sprintf(label, "%02d", row);
              if (ImGui::Selectable(label, item_is_selected,
                                    ImGuiSelectableFlags_SpanAllColumns,
                                    ImVec2(0, 0))) {
                selection = row;
              }
              if (ImGui::BeginDragDropSource(ImGuiDragDropFlags_None)) {
                ImGui::SetDragDropPayload("DND_PLOT", &row, sizeof(int));
                ImGui::TextUnformatted(label);
                ImGui::EndDragDropSource();
              }

              ImGui::TableNextColumn();
              ImGui::TextUnformatted(item_id.name.data(),
                                     item_id.name.data() +
                                         item_id.name.length());
              ImGui::TableNextColumn();
              ImGui::TextUnformatted(item_id.wg_name.data(),
                                     item_id.wg_name.data() +
                                         item_id.wg_name.length());
              ImGui::TableNextColumn();
              ImGui::Text("%d", item_id.index);
            }
          }
          ImGui::EndTable();
        }
      }
    }

    ImGui::End();

    ImGui::SetNextWindowDockID(dockspaceID, ImGuiCond_FirstUseEver);
    ImGui::Begin("Chart");

    static std::string y_label_str;
    static rust::Vec<TimeStamps> time;
    static rust::Vec<TimeSeries> data;

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

    if (ImPlot::BeginPlot(
            "##DND", x_label, y_label,
            ImVec2(ImGui::GetWindowWidth(), ImGui::GetWindowHeight() * 0.92f),
            ImPlotFlags_NoMousePos, ImPlotAxisFlags_Time)) {
      if (plotted_item_row != -1) {
        for (int s = 0; s < data.size(); ++s) {
          const auto &d = data[s];
          if (!d.values.empty()) {
            const auto &t = time[s];
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
