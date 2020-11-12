#include "eclair_ffi.rs.h"

#include <iostream>

std::ostream &operator<<(std::ostream &out, const ItemQualifier &q) {
  switch (q) {
  case ItemQualifier::Time:
    return out << "Time";
  case ItemQualifier::Performance:
    return out << "Performance";
  case ItemQualifier::Field:
    return out << "Field";
  case ItemQualifier::Aquifer:
    return out << "Aquifer";
  case ItemQualifier::Region:
    return out << "Region";
  case ItemQualifier::CrossRegionFlow:
    return out << "CrossRegionFlow";
  case ItemQualifier::Well:
    return out << "Well";
  case ItemQualifier::Completion:
    return out << "Completion";
  case ItemQualifier::Group:
    return out << "Group";
  case ItemQualifier::Block:
    return out << "Block";
  case ItemQualifier::Unrecognized:
    return out << "Unrecognized";
  }
}