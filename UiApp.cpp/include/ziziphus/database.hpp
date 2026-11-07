
#pragma once

// Standard library headers.
#include <string>

// SQLite is intentionally not included here.
struct sqlite3;
// The database interface hides SQLite implementation details
// from the rest of the application.

namespace gxx {

/*
 * Database
 *
 * Provides the main interface for opening, closing, and managing
 * a GXX database connection.
 *
 * Responsibilities:
 * - Open an existing database or create a new database file.
 * - Close the active database connection safely.
 * - Check whether a database connection is active.
 * - Execute SQL statements that do not return query rows.
 * - Expose the active database path.
 * - Preserve the most recent database error message.
 *
 * This class manages the connection only.
 * Database schema creation and word operations belong in
 * separate components.
 */
class Database {
public:

    /*
     * Constructor
     *
     * Creates an empty Database object.
     *
     * No database file is opened automatically.
     * Call open() to establish a connection.
     */
    Database();

    /*
     * Destructor
     *
     * Releases the database connection when the object
     * is destroyed.
     *
     * This prevents the application from leaving an
     * active connection behind.
     */
    ~Database();

    /*
     * Copying is disabled because copying a database
     * connection owner could cause resource-management bugs.
     */
    Database(const Database&) = delete;
    Database& operator=(const Database&) = delete;

    /*
     * Move operations are disabled for now.
     *
     * They can be implemented later if transferring ownership
     * of a database connection becomes necessary.
     */
    Database(Database&&) = delete;
    Database& operator=(Database&&) = delete;

    /*
     * open()
     *
     * Opens a database using the provided filesystem path.
     *
     * Behavior:
     * - Closes any previously active connection.
     * - Creates missing parent directories when necessary.
     * - Opens an existing SQLite database, or creates a new one.
     * - Enables foreign-key constraints for this connection.
     *
     * Parameters:
     * path - Path to the database file, such as "words.gxx".
     *
     * Returns:
     * true  - The database was opened successfully.
     * false - The operation failed.
     *
     * On failure, last_error() provides the error message.
     */
    bool open(const std::string& path);

    /*
     * close()
     *
     * Closes the active database connection.
     *
     * Calling this function when no database is open
     * is safe.
     *
     * Note:
     * All outstanding statements should be finalized before
     * closing the database connection.
     */
    void close();

    /*
     * is_open()
     *
     * Checks whether this object currently holds an
     * active database connection.
     *
     * Returns:
     * true  - A database connection exists.
     * false - No database connection is active.
     */
    bool is_open() const;

    /*
     * execute()
     *
     * Executes an SQL statement that does not need to
     * return result rows.
     *
     * Typical uses:
     * - CREATE TABLE
     * - CREATE INDEX
     * - INSERT
     * - UPDATE
     * - DELETE
     * - PRAGMA configuration statements
     *
     * Do not build SQL statements by concatenating
     * untrusted user input.
     * Use prepared statements for parameterized queries.
     *
     * Parameters:
     * sql - SQL statement to execute.
     *
     * Returns:
     * true  - The SQL statement completed successfully.
     * false - The operation failed.
     *
     * On failure, last_error() provides the error message.
     */
    bool execute(const std::string& sql);

    // Return the active SQLite connection.
    sqlite3* get_connection() const { return db_; }

    /*
     * path()
     *
     * Returns the path of the currently open database.
     *
     * Returns an empty string when no database is open.
     *
     * The returned reference remains valid only while
     * the Database object's state is not modified or destroyed.
     */
    const std::string& path() const;

    /*
     * last_error()
     *
     * Returns the most recent error message recorded
     * by this Database object.
     *
     * A successful operation may clear the previous error.
     * An empty string means no error is currently recorded.
     *
     * This message is intended for diagnostics and
     * displaying useful error information to the user.
     */
    const std::string& last_error() const;

private:

    /*
     * db_
     *
     * Holds the underlying SQLite database connection.
     *
     * The void pointer keeps SQLite's type out of this
     * public header. database.cpp must cast it to sqlite3*
     * before passing it to SQLite functions.
     *
     * nullptr means no connection is active.
     */
    sqlite3* db_ = nullptr;

    /*
     * path_
     *
     * Stores the path of the currently open database file.
     */
    std::string path_;

    /*
     * last_error_
     *
     * Stores the latest database error message.
     */
    std::string last_error_;
};

} // namespace gxx
