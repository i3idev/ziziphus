#include "gxx/word_repository.hpp"

#include <sqlite3.h>
#include <iostream>
#include <string>
#include <utility>
#include <vector>

namespace gxx {
namespace {

std::string column_text(sqlite3_stmt* stmt, int column) {
    const auto* value = sqlite3_column_text(stmt, column);
    return value ? reinterpret_cast<const char*>(value) : "";
}


std::vector<std::string> json_strings(
    sqlite3* db, const std::string& data, const char* path) {
    std::vector<std::string> values;
    const char* sql =
        "SELECT value FROM json_each(?, ?) "
        "WHERE type IN ('text', 'integer', 'real');";

    sqlite3_stmt* stmt = nullptr;
    if (sqlite3_prepare_v2(db, sql, -1, &stmt, nullptr) != SQLITE_OK)
        return values;

    sqlite3_bind_text(stmt, 1, data.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 2, path, -1, SQLITE_TRANSIENT);

    while (sqlite3_step(stmt) == SQLITE_ROW) {
        auto value = column_text(stmt, 0);
        if (!value.empty())
            values.push_back(std::move(value));
    }

    sqlite3_finalize(stmt);
    return values;
}


std::string json_quote(const std::string& value) {
    std::string out = "\"";
    const char* hex = "0123456789abcdef";

    for (unsigned char c : value) {
        switch (c) {
        case '"':  out += "\\\""; break;
        case '\\': out += "\\\\"; break;
        case '\b': out += "\\b"; break;
        case '\f': out += "\\f"; break;
        case '\n': out += "\\n"; break;
        case '\r': out += "\\r"; break;
        case '\t': out += "\\t"; break;
        default:
            if (c < 0x20) {
                out += "\\u00";
                out += hex[c >> 4];
                out += hex[c & 0x0f];
            } else {
                out += static_cast<char>(c);
            }
        }
    }

    out += '"';
    return out;
}

std::string json_array(const std::vector<std::string>& values) {
    std::string out = "[";
    for (std::size_t i = 0; i < values.size(); ++i) {
        if (i) out += ',';
        out += json_quote(values[i]);
    }
    out += ']';
    return out;
}

std::string json_examples(const std::vector<Word::Example>& values) {
    std::string out = "[";
    for (std::size_t i = 0; i < values.size(); ++i) {
        if (i) out += ',';
        out += "{\"en\":" + json_quote(values[i].english);
        out += ",\"fa\":" + json_quote(values[i].translation) + "}";
    }
    out += ']';
    return out;
}

std::string json_family(const std::vector<Word::WordFamily>& values) {
    std::string out = "[";
    for (std::size_t i = 0; i < values.size(); ++i) {
        if (i) out += ',';
        out += "{\"word\":" + json_quote(values[i].word);
        out += ",\"pos\":" + json_quote(values[i].part_of_speech) + "}";
    }
    out += ']';
    return out;
}

bool load_words(sqlite3* db, const std::string& query,
                std::vector<Word>& results, bool filter) {
    results.clear();
    if (!db)
        return false;

    const char* sql = filter ? R"(
        SELECT id, title, COALESCE(level, 'A1'),
               COALESCE(json_extract(data, '$.pos'), ''),
               COALESCE(json_extract(data, '$.pronunciation'), ''),
               COALESCE(json_extract(data, '$.def'), text, ''),
               COALESCE(data, '{}'),
               COALESCE(created_at, ''),
               COALESCE(updated_at, '')
        FROM entries
        WHERE kind = 'word' AND lang = 'en'
          AND (
              title LIKE ? COLLATE NOCASE
              OR text LIKE ? COLLATE NOCASE
              OR json_extract(data, '$.def') LIKE ? COLLATE NOCASE
              OR json_extract(data, '$.translations.fa') LIKE ?
              OR EXISTS (
                  SELECT 1
                  FROM json_each(data, '$.translations.fa') AS tr
                  WHERE tr.value LIKE ?
              )
              OR EXISTS (
                  SELECT 1
                  FROM json_each(data, '$.synonyms') AS syn
                  WHERE syn.value LIKE ? COLLATE NOCASE
              )
          )
        ORDER BY
          CASE
            WHEN title = ? COLLATE NOCASE THEN 0
            WHEN EXISTS (
                SELECT 1
                FROM json_each(data, '$.translations.fa') AS tr
                WHERE tr.value = ?
            ) THEN 0
            WHEN title LIKE ? COLLATE NOCASE THEN 1
            WHEN EXISTS (
                SELECT 1
                FROM json_each(data, '$.translations.fa') AS tr
                WHERE tr.value LIKE ?
            ) THEN 1
            ELSE 2
          END,
          title COLLATE NOCASE;
    )" : R"(
        SELECT id, title, COALESCE(level, 'A1'),
               COALESCE(json_extract(data, '$.pos'), ''),
               COALESCE(json_extract(data, '$.pronunciation'), ''),
               COALESCE(json_extract(data, '$.def'), text, ''),
               COALESCE(data, '{}'),
               COALESCE(created_at, ''),
               COALESCE(updated_at, '')
        FROM entries
        WHERE kind = 'word' AND lang = 'en'
        ORDER BY title COLLATE NOCASE;
    )";

    sqlite3_stmt* stmt = nullptr;
    if (sqlite3_prepare_v2(db, sql, -1, &stmt, nullptr) != SQLITE_OK) {
        std::cerr << "[DB ERROR] " << sqlite3_errmsg(db) << '\n';
        return false;
    }

    if (filter) {
        const std::string pattern = "%" + query + "%";
        const std::string prefix = query + "%";

        sqlite3_bind_text(stmt, 1, pattern.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_bind_text(stmt, 2, pattern.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_bind_text(stmt, 3, pattern.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_bind_text(stmt, 4, pattern.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_bind_text(stmt, 5, pattern.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_bind_text(stmt, 6, pattern.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_bind_text(stmt, 7, query.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_bind_text(stmt, 8, query.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_bind_text(stmt, 9, prefix.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_bind_text(stmt, 10, prefix.c_str(), -1, SQLITE_TRANSIENT);
    }

    int rc = SQLITE_OK;
    while ((rc = sqlite3_step(stmt)) == SQLITE_ROW) {
        Word word;
        word.id = column_text(stmt, 0);
        word.title = column_text(stmt, 1);
        word.level = column_text(stmt, 2);
        word.part_of_speech = column_text(stmt, 3);
        word.pronunciation = column_text(stmt, 4);
        word.definition = column_text(stmt, 5);

        const std::string data = column_text(stmt, 6);
        word.created_at = column_text(stmt, 7);
        word.updated_at = column_text(stmt, 8);

        word.translations = json_strings(db, data, "$.translations.fa");
        word.synonyms = json_strings(db, data, "$.synonyms");
        word.antonyms = json_strings(db, data, "$.antonyms");
        word.collocations = json_strings(db, data, "$.collocations");

        sqlite3_stmt* detail = nullptr;
        const char* detail_sql =
            "SELECT COALESCE(json_extract(?, '$.notes'), ''), "
            "COALESCE(json_extract(?, '$.def'), '')";

        if (sqlite3_prepare_v2(db, detail_sql, -1, &detail, nullptr) == SQLITE_OK) {
            sqlite3_bind_text(detail, 1, data.c_str(), -1, SQLITE_TRANSIENT);
            sqlite3_bind_text(detail, 2, data.c_str(), -1, SQLITE_TRANSIENT);
            if (sqlite3_step(detail) == SQLITE_ROW) {
                word.notes = column_text(detail, 0);
                if (word.definition.empty())
                    word.definition = column_text(detail, 1);
            }
            sqlite3_finalize(detail);
        }

        sqlite3_stmt* examples = nullptr;
        if (sqlite3_prepare_v2(
                db,
                "SELECT COALESCE(json_extract(value, '$.en'), ''), "
                "COALESCE(json_extract(value, '$.fa'), '') "
                "FROM json_each(?, '$.examples');",
                -1, &examples, nullptr) == SQLITE_OK) {
            sqlite3_bind_text(examples, 1, data.c_str(), -1, SQLITE_TRANSIENT);
            while (sqlite3_step(examples) == SQLITE_ROW) {
                Word::Example example;
                example.english = column_text(examples, 0);
                example.translation = column_text(examples, 1);
                word.examples.push_back(std::move(example));
            }
            sqlite3_finalize(examples);
        }

        sqlite3_stmt* family = nullptr;
        if (sqlite3_prepare_v2(
                db,
                "SELECT COALESCE(json_extract(value, '$.word'), "
                "json_extract(value, '$.word_form'), ''), "
                "COALESCE(json_extract(value, '$.pos'), "
                "json_extract(value, '$.part_of_speech'), '') "
                "FROM json_each(?, '$.family');",
                -1, &family, nullptr) == SQLITE_OK) {
            sqlite3_bind_text(family, 1, data.c_str(), -1, SQLITE_TRANSIENT);
            while (sqlite3_step(family) == SQLITE_ROW) {
                Word::WordFamily item;
                item.word = column_text(family, 0);
                item.part_of_speech = column_text(family, 1);
                if (!item.word.empty())
                    word.word_family.push_back(std::move(item));
            }
            sqlite3_finalize(family);
        }

        results.push_back(std::move(word));
    }

    sqlite3_finalize(stmt);
    if (rc != SQLITE_DONE)
        std::cerr << "[DB ERROR] Reading entries: "
                  << sqlite3_errmsg(db) << '\n';

    return rc == SQLITE_DONE;
}
} // namespace

WordRepository::WordRepository(Database& database)
    : database_(database) {}

bool WordRepository::initialize() {
    if (!database_.is_open())
        return false;

    sqlite3* db = get_connection();
    if (!db)
        return false;

    sqlite3_stmt* stmt = nullptr;
    const char* sql =
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name='entries';";

    if (sqlite3_prepare_v2(db, sql, -1, &stmt, nullptr) != SQLITE_OK)
        return false;

    const bool exists = sqlite3_step(stmt) == SQLITE_ROW;
    sqlite3_finalize(stmt);

    if (!exists)
        std::cerr << "[DB ERROR] The GXX entries table was not found.\n";

    return exists;
}

bool WordRepository::add(const Word& word) {
    if (!database_.is_open() || word.title.empty())
        return false;

    sqlite3* db = get_connection();
    if (!db)
        return false;

    const char* sql = R"(
        INSERT INTO entries (id, kind, lang, level, title, text, data,
                             created_at, updated_at)
        VALUES (
            'word:' || lower(trim(?)), 'word', 'en', ?, trim(?), ?,
            json_object(
                'id', 'en-' || lower(trim(?)),
                'word', trim(?),
                'translations', json_object('fa', json(?)),
                'synonyms', json(?),
                'antonyms', json(?),
                'examples', json(?),
                'notes', ?,
                'pos', ?,
                'pronunciation', ?,
                'def', ?,
                'collocations', json(?),
                'family', json(?),
                'level', ?
            ),
            datetime('now'), datetime('now')
        );
    )";

    const std::string translations = json_array(word.translations);
    const std::string synonyms = json_array(word.synonyms);
    const std::string antonyms = json_array(word.antonyms);
    const std::string examples = json_examples(word.examples);
    const std::string collocations = json_array(word.collocations);
    const std::string family = json_family(word.word_family);

    sqlite3_stmt* stmt = nullptr;
    if (sqlite3_prepare_v2(db, sql, -1, &stmt, nullptr) != SQLITE_OK) {
        std::cerr << "[DB ERROR] Prepare insert: "
                  << sqlite3_errmsg(db) << '\n';
        return false;
    }

    sqlite3_bind_text(stmt, 1, word.title.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 2, word.level.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 3, word.title.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 4, word.definition.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 5, word.title.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 6, word.title.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 7, translations.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 8, synonyms.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 9, antonyms.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 10, examples.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 11, word.notes.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 12, word.part_of_speech.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 13, word.pronunciation.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 14, word.definition.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 15, collocations.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 16, family.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 17, word.level.c_str(), -1, SQLITE_TRANSIENT);

    const int rc = sqlite3_step(stmt);
    if (rc != SQLITE_DONE)
        std::cerr << "[DB ERROR] Insert: " << sqlite3_errmsg(db) << '\n';

    sqlite3_finalize(stmt);
    return rc == SQLITE_DONE;
}

bool WordRepository::update(const Word& word) {
    if (!database_.is_open() || word.id.empty() || word.title.empty())
        return false;

    sqlite3* db = get_connection();
    if (!db)
        return false;

    const char* sql = R"(
        UPDATE entries
        SET title = trim(?),
            level = ?,
            text = ?,
            data = json_set(
                COALESCE(data, '{}'),
                '$.word', trim(?),
                '$.translations.fa', json(?),
                '$.level', ?,
                '$.pos', ?,
                '$.pronunciation', ?,
                '$.def', ?,
                '$.notes', ?,
                '$.synonyms', json(?),
                '$.antonyms', json(?),
                '$.examples', json(?),
                '$.collocations', json(?),
                '$.family', json(?),
                '$.updated_at', datetime('now')
            ),
            updated_at = datetime('now')
        WHERE id = ? AND kind = 'word' AND lang = 'en';
    )";

    const std::string translations = json_array(word.translations);
    const std::string synonyms = json_array(word.synonyms);
    const std::string antonyms = json_array(word.antonyms);
    const std::string examples = json_examples(word.examples);
    const std::string collocations = json_array(word.collocations);
    const std::string family = json_family(word.word_family);

    sqlite3_stmt* stmt = nullptr;
    if (sqlite3_prepare_v2(db, sql, -1, &stmt, nullptr) != SQLITE_OK) {
        std::cerr << "[DB ERROR] Prepare update: "
                  << sqlite3_errmsg(db) << '\n';
        return false;
    }

    sqlite3_bind_text(stmt, 1, word.title.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 2, word.level.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 3, word.definition.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 4, word.title.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 5, translations.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 6, word.level.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 7, word.part_of_speech.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 8, word.pronunciation.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 9, word.definition.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 10, word.notes.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 11, synonyms.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 12, antonyms.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 13, examples.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 14, collocations.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 15, family.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 16, word.id.c_str(), -1, SQLITE_TRANSIENT);

    const int rc = sqlite3_step(stmt);
    const bool success = rc == SQLITE_DONE && sqlite3_changes(db) > 0;

    if (rc != SQLITE_DONE)
        std::cerr << "[DB ERROR] Update: " << sqlite3_errmsg(db) << '\n';

    sqlite3_finalize(stmt);
    return success;
}

bool WordRepository::remove(int id) {
    if (!database_.is_open())
        return false;

    sqlite3* db = get_connection();
    if (!db)
        return false;

    const char* sql = R"(
        DELETE FROM entries
        WHERE id = (
            SELECT id FROM (
                SELECT id, row_number() OVER (ORDER BY title COLLATE NOCASE) AS rn
                FROM entries
                WHERE kind = 'word' AND lang = 'en'
            ) WHERE rn = ?
        );
    )";

    sqlite3_stmt* stmt = nullptr;
    if (sqlite3_prepare_v2(db, sql, -1, &stmt, nullptr) != SQLITE_OK)
        return false;

    sqlite3_bind_int(stmt, 1, id);
    const int rc = sqlite3_step(stmt);
    const bool success = rc == SQLITE_DONE && sqlite3_changes(db) > 0;
    sqlite3_finalize(stmt);
    return success;
}

bool WordRepository::search(const std::string& query,
                            std::vector<Word>& results) {
    if (!database_.is_open())
        return false;

    return load_words(get_connection(), query, results, true);
}

bool WordRepository::list(std::vector<Word>& results) {
    if (!database_.is_open())
        return false;

    return load_words(get_connection(), "", results, false);
}

} // namespace gxx
