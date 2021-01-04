#include "cxx.h"
#include "eclair_ffi.rs.h"

#include <tuple>

namespace eclair {

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

} // namespace eclair