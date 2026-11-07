
#include "gxx/settings_service.hpp"

#include <sqlite3.h>

#include <string>

namespace gxx {

SettingsService::SettingsService(Database& database)
    : database_(database) {
    // Keep a reference to the application's database connection.
}

bool SettingsService::initialize() {
    // Create the settings table if it does not already exist.
    const std::string sql = R"(
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
    )";

    // Execute the schema creation statement.
    if (!database_.execute(sql)) {
        return false;
    }

    // Insert default settings without overwriting existing values.
    const std::string defaults = R"(
        INSERT OR IGNORE INTO settings (key, value) VALUES
            ('language', 'en'),
            ('default_level', 'A1'),
            ('theme', 'dark');
    )";

    return database_.execute(defaults);
}

bool SettingsService::set(
    const std::string& key,
    const std::string& value
) {
    // Reject empty setting keys.
    if (key.empty() || !database_.is_open()) {
        return false;
    }

    // Obtain the active SQLite connection.
    sqlite3* db = database_.get_connection();
    if (!db) {
        return false;
    }

    // Insert a setting or update its value when the key already exists.
    const char* sql = R"(
        INSERT INTO settings (key, value, updated_at)
        VALUES (?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = CURRENT_TIMESTAMP;
    )";

    sqlite3_stmt* statement = nullptr;

    // Compile the parameterized statement.
    if (sqlite3_prepare_v2(db, sql, -1, &statement, nullptr) != SQLITE_OK) {
        return false;
    }

    // Bind the key and value safely.
    sqlite3_bind_text(
        statement, 1, key.c_str(), -1, SQLITE_TRANSIENT
    );
    sqlite3_bind_text(
        statement, 2, value.c_str(), -1, SQLITE_TRANSIENT
    );

    // Execute the INSERT or UPDATE operation.
    const int result = sqlite3_step(statement);
    const bool success = result == SQLITE_DONE;

    // Release SQLite resources.
    sqlite3_finalize(statement);

    return success;
}

bool SettingsService::get(
    const std::string& key,
    std::string& value
) const {
    // A database connection is required to read settings.
    if (key.empty() || !database_.is_open()) {
        return false;
    }

    // Obtain the active SQLite connection.
    sqlite3* db = database_.get_connection();
    if (!db) {
        return false;
    }

    // Prepare a query for the requested setting.
    const char* sql = R"(
        SELECT value
        FROM settings
        WHERE key = ?;
    )";

    sqlite3_stmt* statement = nullptr;

    if (sqlite3_prepare_v2(db, sql, -1, &statement, nullptr) != SQLITE_OK) {
        return false;
    }

    // Bind the requested setting key.
    sqlite3_bind_text(
        statement, 1, key.c_str(), -1, SQLITE_TRANSIENT
    );

    // Read the matching setting, if one exists.
    const int result = sqlite3_step(statement);
    bool success = false;

    if (result == SQLITE_ROW) {
        const unsigned char* text = sqlite3_column_text(statement, 0);

        // Copy the returned value before finalizing the statement.
        value = text
            ? reinterpret_cast<const char*>(text)
            : "";

        success = true;
    }

    // Release the prepared statement.
    sqlite3_finalize(statement);

    return success;
}

bool SettingsService::remove(const std::string& key) {
    // Reject invalid requests before accessing SQLite.
    if (key.empty() || !database_.is_open()) {
        return false;
    }

    sqlite3* db = database_.get_connection();
    if (!db) {
        return false;
    }

    // Delete the setting associated with the supplied key.
    const char* sql = "DELETE FROM settings WHERE key = ?;";
    sqlite3_stmt* statement = nullptr;

    if (sqlite3_prepare_v2(db, sql, -1, &statement, nullptr) != SQLITE_OK) {
        return false;
    }

    // Bind the setting key.
    sqlite3_bind_text(
        statement, 1, key.c_str(), -1, SQLITE_TRANSIENT
    );

    // Execute the DELETE statement.
    const int result = sqlite3_step(statement);
    const bool success = result == SQLITE_DONE;

    // Release SQLite resources.
    sqlite3_finalize(statement);

    return success;
}

} // namespace gxx
