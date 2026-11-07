#pragma once

#include <string>
#include "gxx/database.hpp"

namespace gxx {

class SettingsService {
public:
    explicit SettingsService(Database& database);

    bool initialize();
    bool set(const std::string& key, const std::string& value);
    bool get(const std::string& key, std::string& value) const;
    bool remove(const std::string& key);

private:
    Database& database_;
};

} // namespace gxx
