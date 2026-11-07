
#pragma once

#include <string>

namespace gxx {

/*
 * Settings
 *
 * Stores user-configurable preferences for the GXX application.
 *
 * This structure only represents settings in memory.
 * SettingsService is responsible for reading and writing
 * persistent settings in the database.
 */
struct Settings {
    // Interface language used by the application.
    std::string language = "en";

    // Default vocabulary proficiency level.
    std::string default_level = "A1";

    // Application appearance preference.
    std::string theme = "dark";
};

} // namespace gxx
