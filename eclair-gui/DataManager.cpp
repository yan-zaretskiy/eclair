#include "DataManager.h"
#include "FilteredVector.h"

#include <Mahi/Gui.hpp>
#include <Mahi/Util.hpp>

#include <sstream>

using namespace mahi::gui;
using namespace mahi::util;

namespace eclair {
DataManager::PlotData DataManager::plot_data(size_t summary_index,
                                             int index) const {
  auto time = manager->timestamps(summary_index);
  const auto &item_id = item_ids[index];
  switch (item_id.qualifier) {
  case ItemQualifier::Time:
    return {time, manager->time_item(summary_index, item_id.name)};
  case ItemQualifier::Performance:
    return {time, manager->performance_item(summary_index, item_id.name)};
  case ItemQualifier::Field:
    return {time, manager->field_item(summary_index, item_id.name)};
  case ItemQualifier::Aquifer:
    return {time,
            manager->aquifer_item(summary_index, item_id.name, item_id.index)};
  case ItemQualifier::Region:
    return {time,
            manager->region_item(summary_index, item_id.name, item_id.index)};
  case ItemQualifier::CrossRegionFlow:
    return {time, manager->cross_region_item(summary_index, item_id.name,
                                             item_id.index)};
  case ItemQualifier::Well:
    return {time,
            manager->well_item(summary_index, item_id.name, item_id.wg_name)};
  case ItemQualifier::Completion:
    return {time, manager->completion_item(summary_index, item_id.name,
                                           item_id.wg_name, item_id.index)};
  case ItemQualifier::Group:
    return {time,
            manager->group_item(summary_index, item_id.name, item_id.wg_name)};
  case ItemQualifier::Block:
    return {time,
            manager->block_item(summary_index, item_id.name, item_id.index)};
  case ItemQualifier::Unrecognized:
    throw std::runtime_error("Why would you wanna do this?");
  }
}

std::string_view DataManager::item_name(int index) const {
  return {item_ids[index].name.data(), item_ids[index].name.size()};
}

std::string DataManager::item_name_and_location(int index) const {
  const auto &item_id = item_ids[index];
  std::stringstream oss;

  std::string_view name = item_name(index);
  std::string_view wg_name(item_id.wg_name.data(), item_id.wg_name.size());

  switch (item_id.qualifier) {
  case ItemQualifier::Time:
  case ItemQualifier::Performance:
  case ItemQualifier::Field:
    oss << name;
    break;
  case ItemQualifier::Aquifer:
    oss << name << " @ " << item_id.index;
    break;
  case ItemQualifier::Region:
    oss << name << " @ ";
    if (wg_name.size() != 0) {
      oss << wg_name;
    } else {
      oss << item_id.index;
    }
    break;
  case ItemQualifier::CrossRegionFlow:
    oss << name << " @ " << item_id.index;
    break;
  case ItemQualifier::Well:
    oss << name << " @ " << wg_name;
    break;
  case ItemQualifier::Completion:
    oss << name << " @ " << wg_name << "[" << item_id.index << "]";
    break;
  case ItemQualifier::Group:
    oss << name << " @ " << wg_name;
    break;
  case ItemQualifier::Block:
    oss << name << " @ " << item_id.index;
    break;
  case ItemQualifier::Unrecognized:
    oss << "Unrecognized @ " << wg_name << "[" << item_id.index << "]";
    break;
  }
  return oss.str();
}

std::string DataManager::item_full_name(int summary_index, int index) const {
  const auto &item_id = item_ids[index];
  std::stringstream oss;

  const auto &sn = manager->summary_name(summary_index);
  std::string_view summary_name(sn.data(), sn.size());
  oss << summary_name << ": " << item_name_and_location(index);

  return oss.str();
}

bool DataManager::names_equal(int index1, int index2) const {
  return item_ids[index1].name == item_ids[index2].name;
}

// Refresh the time data.
bool DataManager::refresh() { return manager->refresh(); }

void DataManager::draw() {
  // Draw the "Sources" first. Sources can be removed, that's why we don't draw
  // the "Items" table in the same if statement.
  int to_be_removed = -1;
  if (!empty()) {
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
      item_ids = manager->all_item_ids();
    }
  }

  if (!empty()) {
    if (ImGui::CollapsingHeader("Items", ImGuiTreeNodeFlags_DefaultOpen)) {
      ImGuiTableFlags flags = ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                              ImGuiTableFlags_ScrollY;

      const int COLUMNS_COUNT = 4;

      if (ImGui::BeginTable("##items_table", COLUMNS_COUNT, flags)) {
        ImGui::TableSetupScrollFreeze(0, 1);
        ImGui::TableSetupColumn("#", ImGuiTableColumnFlags_WidthFixed, 30.0f);
        ImGui::TableSetupColumn("Name");
        ImGui::TableSetupColumn("Well/Group");
        ImGui::TableSetupColumn("Index");

        // header row
        ImGui::TableNextRow(ImGuiTableRowFlags_Headers);
        for (int column = 0; column < COLUMNS_COUNT; column++) {
          ImGui::TableSetColumnIndex(column);
          const char *column_name = ImGui::TableGetColumnName(
              column); // Retrieve name passed to TableSetupColumn()
          ImGui::PushID(column);
          ImGui::TableHeader(column_name);
          if (column > 0) {
            filters[column - 1]->Draw("##items_filter",
                                      ImGui::GetContentRegionAvail().x);
          }
          ImGui::PopID();
        }

        // data rows
        FilteredVector filtered_items(item_ids, [this](auto &&item) -> bool {
          return filter(std::forward<decltype(item)>(item));
        });

        int selection = -1;
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
}

bool DataManager::filter(const ItemId &item_id) const {
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