
#include "gxx/database.hpp"

#include <sqlite3.h>

#include <filesystem>
#include <string>

namespace gxx {

Database::Database() = default;

Database::~Database() {
    close();
}

bool Database::open(const std::string& path) {
    // Close the current database before opening another one.
    close();

    // Create the parent directory when it does not exist.
    const std::filesystem::path db_path(path);
    const auto parent = db_path.parent_path();

    if (!parent.empty()) {
        std::error_code error;
        std::filesystem::create_directories(parent, error);

        if (error) {
            last_error_ = error.message();
            return false;
        }
    }

    // Open the SQLite database and enable read/write access.
    const int result = sqlite3_open_v2(
        path.c_str(),
        &db_,
        SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
        nullptr
    );

    if (result != SQLITE_OK) {
        last_error_ = db_ ? sqlite3_errmsg(db_)
                          : "Failed to allocate SQLite connection.";

        close();
        return false;
    }

    // Enable foreign key constraints for relational data.
    if (!execute("PRAGMA foreign_keys = ON;")) {
        close();
        return false;
    }

    // Store the active database path after a successful connection.
    path_ = path;
    last_error_.clear();

    return true;
}

void Database::close() {
    // Release the SQLite connection and reset the current state.
    if (db_) {
        sqlite3_close_v2(db_);
        db_ = nullptr;
    }

    path_.clear();
}

bool Database::is_open() const {
    // Report whether a database connection is active.
    return db_ != nullptr;
}

bool Database::execute(const std::string& sql) {
    // Reject SQL execution when no database is open.
    if (!db_) {
        last_error_ = "No database is open.";
        return false;
    }

    // Execute an SQL statement without returning query rows.
    char* error_message = nullptr;

    const int result = sqlite3_exec(
        db_,
        sql.c_str(),
        nullptr,
        nullptr,
        &error_message
    );

    if (result != SQLITE_OK) {
        // Preserve the error message before releasing SQLite memory.
        last_error_ = error_message
            ? error_message
            : sqlite3_errmsg(db_);

        sqlite3_free(error_message);
        return false;
    }

    last_error_.clear();
    return true;
}

const std::string& Database::path() const {
    // Return the path of the currently open database.
    return path_;
}

const std::string& Database::last_error() const {
    // Return the most recent database error.
    return last_error_;
}

} // namespace gxx
