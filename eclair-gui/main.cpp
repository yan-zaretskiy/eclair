#include "eclair_ffi.rs.h"
#include "rust/cxx.h"

#include <Mahi/Gui.hpp>

#include <iostream>

std::ostream &operator<<(std::ostream &out, const ItemQualifier &q);

using namespace mahi::gui;

class Demo : public Application {
public:
  Demo() : Application() {}

  void update() override {
    // Official ImGui demo (see imgui_demo.cpp for full example)
    static bool open = true;
    ImGui::ShowDemoWindow(&open);
    if (!open)
      quit();
  }
};

int main() {
  auto manager = make_manager();
  manager->add_from_files("../../assets/SPE10.UNSMRY", "SPE10");

  auto n_items = manager->count_items();
  std::cout << "Manager has " << n_items << " items.\n";

  auto item_ids = manager->all_item_ids();
  for (const auto &id : item_ids) {
    std::cout << "Name: " << id.name << ", qualifier: " << id.qualifier
              << ", wg_name: " << id.wg_name << ", index: " << id.index << '\n';
  }

  auto wopr = manager->well_item("WOPR", "P1");
  for (const auto &summary : wopr) {
    std::cout << "Name: " << summary.name << '\n';
    std::cout << "Values: ";
    for (auto v : summary.values) {
      std::cout << v << ' ';
    }
    std::cout << '\n';
  }

  Demo app;
  app.run();
}
