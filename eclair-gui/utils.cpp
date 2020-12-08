#include "cxx.h"
#include "eclair_ffi.rs.h"

#include <sstream>
#include <tuple>

std::string to_string(const ItemQualifier &q) {
  switch (q) {
  case ItemQualifier::Time:
    return "Time";
  case ItemQualifier::Performance:
    return "Performance";
  case ItemQualifier::Field:
    return "Field";
  case ItemQualifier::Aquifer:
    return "Aquifer";
  case ItemQualifier::Region:
    return "Region";
  case ItemQualifier::CrossRegionFlow:
    return "CrossRegionFlow";
  case ItemQualifier::Well:
    return "Well";
  case ItemQualifier::Completion:
    return "Completion";
  case ItemQualifier::Group:
    return "Group";
  case ItemQualifier::Block:
    return "Block";
  case ItemQualifier::Unrecognized:
    return "Unrecognized";
  }
}

std::string item_name(const ItemId &item_id) {
  std::stringstream oss;

  switch (item_id.qualifier) {
  case ItemQualifier::Time:
  case ItemQualifier::Performance:
  case ItemQualifier::Field:
    oss << item_id.name;
    break;
  case ItemQualifier::Aquifer:
    oss << item_id.name << " @ " << item_id.index;
    break;
  case ItemQualifier::Region:
    oss << item_id.name << " @ ";
    if (item_id.wg_name.size() != 0) {
      oss << item_id.wg_name;
    } else {
      oss << item_id.index;
    }
    break;
  case ItemQualifier::CrossRegionFlow:
    oss << item_id.name << " @ " << item_id.index;
    break;
  case ItemQualifier::Well:
    oss << item_id.name << " @ " << item_id.wg_name;
    break;
  case ItemQualifier::Completion:
    oss << item_id.name << " @ " << item_id.wg_name << "[" << item_id.index
        << "]";
    break;
  case ItemQualifier::Group:
    oss << item_id.name << " @ " << item_id.wg_name;
    break;
  case ItemQualifier::Block:
    oss << item_id.name << " @ " << item_id.index;
    break;
  case ItemQualifier::Unrecognized:
    oss << "Unrecognized @ " << item_id.wg_name << "[" << item_id.index << "]";
    break;
  }
  return oss.str();
}

rust::Vec<TimeSeries>
get_item_values(const ItemId &item_id,
                const rust::Box<SummaryManager> &manager) {
  switch (item_id.qualifier) {
  case ItemQualifier::Time:
    return manager->time_item(item_id.name);
  case ItemQualifier::Performance:
    return manager->performance_item(item_id.name);
  case ItemQualifier::Field:
    return manager->field_item(item_id.name);
  case ItemQualifier::Aquifer:
    return manager->aquifer_item(item_id.name, item_id.index);
  case ItemQualifier::Region:
    return manager->region_item(item_id.name, item_id.index);
  case ItemQualifier::CrossRegionFlow:
    return manager->cross_region_item(item_id.name, item_id.index);
  case ItemQualifier::Well:
    return manager->well_item(item_id.name, item_id.wg_name);
  case ItemQualifier::Completion:
    return manager->completion_item(item_id.name, item_id.wg_name,
                                    item_id.index);
  case ItemQualifier::Group:
    return manager->group_item(item_id.name, item_id.wg_name);
  case ItemQualifier::Block:
    return manager->block_item(item_id.name, item_id.index);
  case ItemQualifier::Unrecognized:
    throw std::runtime_error("Why would you wanna do this?");
  }
}

std::tuple<double, double> time_range(const rust::Vec<TimeStamps> &times) {
  double min = std::numeric_limits<double>::max();
  double max = std::numeric_limits<double>::lowest();
  for (const auto &ts : times) {
    min = std::min(min, ts.values.front());
    max = std::max(max, ts.values.back());
  }
  return {min, max};
}

std::tuple<double, double> data_range(const rust::Vec<TimeSeries> &data) {
  double min = std::numeric_limits<double>::max();
  double max = std::numeric_limits<double>::lowest();
  for (const auto &d : data) {
    if (d.values.empty()) {
      continue;
    }
    const auto [cmin, cmax] =
        std::minmax_element(d.values.begin(), d.values.end());
    min = std::min(min, *cmin);
    max = std::max(max, *cmax);
  }
  return {min, max};
}