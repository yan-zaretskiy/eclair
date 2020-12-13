#ifndef ECLAIR_GUI_FILTEREDVECTOR_H
#define ECLAIR_GUI_FILTEREDVECTOR_H

#include <vector>

template <typename V, typename Predicate> class FilteredVector {
public:
  FilteredVector(const V &vec, Predicate p) : vec{vec} {
    for (size_t i = 0; i < vec.size(); ++i) {
      if (p(vec[i])) {
        indices.push_back(i);
      }
    }
  }

  using value_type = typename V::value_type;
  const value_type &operator[](size_t idx) const { return vec[indices[idx]]; }

  [[nodiscard]] size_t size() const { return indices.size(); }

  [[nodiscard]] size_t original_idx(size_t idx) const { return indices[idx]; }

private:
  const V &vec;
  std::vector<size_t> indices;
};

#endif // ECLAIR_GUI_FILTEREDVECTOR_H
