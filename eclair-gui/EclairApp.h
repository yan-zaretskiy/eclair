#ifndef ECLAIR_GUI_ECLAIRAPP_H
#define ECLAIR_GUI_ECLAIRAPP_H

#include "Chart.h"
#include "DataManager.h"

#include <ImGuiFileBrowser.h>
#include <Mahi/Gui.hpp>

#include <tuple>

namespace eclair {
class EclairApp : public mahi::gui::Application {
public:
  EclairApp();

  void update() override;

private:
  std::tuple<bool, bool> draw_main_menu();

  void file_drop_handler(const std::vector<std::string> &paths);

  imgui_addons::ImGuiFileBrowser file_dialog;

  Chart chart;
  DataManager data_manager;
};

} // namespace eclair

#endif // ECLAIR_GUI_ECLAIRAPP_H
