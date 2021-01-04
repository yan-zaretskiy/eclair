#include "EclairApp.h"

#include <implot_internal.h>

namespace eclair {
// Helper function to create a splitter widget.
bool Splitter(bool split_vertically, float thickness, float *size1,
              float *size2, float min_size1, float min_size2,
              float splitter_long_axis_size = -1.0f) {
  ImGuiContext &g = *GImGui;
  ImGuiWindow *window = g.CurrentWindow;
  ImGuiID id = window->GetID("##Splitter");
  ImRect bb;
  bb.Min = window->DC.CursorPos +
           (split_vertically ? ImVec2(*size1, 0.0f) : ImVec2(0.0f, *size1));
  bb.Max = bb.Min +
           ImGui::CalcItemSize(split_vertically
                                   ? ImVec2(thickness, splitter_long_axis_size)
                                   : ImVec2(splitter_long_axis_size, thickness),
                               0.0f, 0.0f);
  return ImGui::SplitterBehavior(bb, id,
                                 split_vertically ? ImGuiAxis_X : ImGuiAxis_Y,
                                 size1, size2, min_size1, min_size2, 0.0f);
}

EclairApp::EclairApp()
    : Application(800, 600, "Eclair"), data_manager{}, chart(data_manager) {
  // Logging for the Rust backend.
  enable_logger();

  ImGui::DisableViewports();
  ImGui::DisableDocking();

  // ImPlot styling settings.
  ImPlotStyle &style = ImPlot::GetStyle();
  style.LineWeight = 2.0;
  style.FitPadding = ImVec2(0.05f, 0.05f);
  style.PlotPadding = ImVec2(0, 0);

  // Event handlers.
  on_file_drop.connect(this, &EclairApp::file_drop_handler);
}

void EclairApp::file_drop_handler(const std::vector<std::string> &paths) {
  data_manager.add_from_files(paths);
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
    data_manager.add_from_files(file_dialog.selected_path);
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
      data_manager.add_from_network(host, port);
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

  // Primary window.
  ImGuiViewport *viewport = ImGui::GetMainViewport();
  ImGui::SetNextWindowPos(viewport->GetWorkPos());
  ImGui::SetNextWindowSize(viewport->GetWorkSize());
  ImGui::SetNextWindowViewport(viewport->ID);
  ImGui::PushStyleVar(ImGuiStyleVar_WindowRounding, 0.0f);
  ImGui::PushStyleVar(ImGuiStyleVar_WindowBorderSize, 0.0f);

  ImGuiWindowFlags windowFlags =
      ImGuiWindowFlags_NoDecoration | ImGuiWindowFlags_NoMove |
      ImGuiWindowFlags_NoBringToFrontOnFocus | ImGuiWindowFlags_NoNavFocus;

  ImGui::Begin("Main", nullptr, windowFlags);
  ImGui::PopStyleVar(2);
  static float sz1 = 200;
  // 24 is the empirical width to avoid scrolling. How do I get it properly?
  float sz2 = ImGui::GetCurrentWindow()->Size.x - sz1 - 24;
  Splitter(true, 2.0f, &sz1, &sz2, 100, 400);
  ImGui::BeginChild("Data", ImVec2(sz1, -1.0), false);
  data_manager.draw();
  ImGui::EndChild();
  ImGui::SameLine();
  ImGui::BeginChild("Chart", ImVec2(sz2, -1.0), false,
                    ImGuiWindowFlags_NoScrollbar);

  // Update data.
  data_manager.refresh();

  // Then plot it.
  chart.draw();
  ImGui::EndChild();
  ImGui::End();
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

} // namespace eclair