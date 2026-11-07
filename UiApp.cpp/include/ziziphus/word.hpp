#pragma once

#include <string>
#include <vector>

namespace gxx {

struct Word {
    // Database identity.
    std::string id;

    // Core vocabulary fields.
    std::string title;
    std::string level = "A1";
    std::string part_of_speech;
    std::string pronunciation;
    std::string definition;

    // Translations and related vocabulary.
    std::vector<std::string> translations;
    std::vector<std::string> synonyms;
    std::vector<std::string> antonyms;

    // Usage examples.
    struct Example {
        std::string english;
        std::string translation;
    };

    std::vector<Example> examples;

    // Common word combinations.
    std::vector<std::string> collocations;

    // Related word forms.
    struct WordFamily {
        std::string word;
        std::string part_of_speech;
    };

    std::vector<WordFamily> word_family;

    // Additional notes and timestamps.
    std::string notes;
    std::string created_at;
    std::string updated_at;

    // Original JSON data from the database.
    std::string raw_data;
};

} // namespace gxx
