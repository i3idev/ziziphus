
#pragma once

#include <string>
#include <vector>

#include "gxx/database.hpp"
#include "gxx/word.hpp"

namespace gxx {

/*
 * WordRepository
 *
 * Provides database operations for vocabulary records.
 *
 * Responsibilities:
 * - Initialize the vocabulary table.
 * - Insert new words into the database.
 * - Remove words by their unique identifiers.
 *
 * This class does not own the Database instance.
 * The Database object must remain alive while this repository is in use.
 */
class WordRepository {
public:
    /*
     * Construct a repository connected to an existing Database object.
     *
     * The database must be opened before repository operations are used.
     */
    explicit WordRepository(Database& database);

    // Initialize the database schema required by the repository.
    bool initialize();

    // Insert a new word into the database.
    bool add(const Word& word);
    bool update(const Word& word);

    // Search vocabulary by title, translation, or definition.
    bool search(const std::string& query, std::vector<Word>& results);
    bool list(std::vector<Word>& results);

    // Remove a word using its database identifier.
    bool remove(int id);

private:
    /*
     * Reference to the database connection.
     *
     * The repository does not open or close the database itself.
     */
    Database& database_;

    /*
     * Return the underlying SQLite connection.
     *
     * This helper requires Database to expose its SQLite handle.
     * Keep this method only if the matching implementation is provided.
     */
    sqlite3* get_connection() const { return database_.get_connection(); }
};

} // namespace gxx
