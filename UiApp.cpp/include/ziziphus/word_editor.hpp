
#pragma once

#include <gtkmm.h>
#include <string>
#include <vector>
#include <vector>

#include "gxx/database.hpp"
#include "gxx/word_repository.hpp"

namespace gxx {

/*
 * WordEditor
 *
 * Provides a dialog for creating vocabulary entries.
 *
 * Responsibilities:
 * - Collect vocabulary information from the user.
 * - Validate the required word title.
 * - Save vocabulary entries through WordRepository.
 * - Close the dialog when saving succeeds or is cancelled.
 *
 * The repository is borrowed by reference and is not owned
 * or destroyed by this dialog.
 */
class WordEditor : public Gtk::Window {
public:
    /*
     * Construct the word editor dialog.
     *
     * The repository must remain alive while this dialog is in use.
     */
    WordEditor(Database& database, WordRepository& repository,
              const Word* word = nullptr);

    // Prevent copying GTK widgets and their internal state.
    WordEditor(const WordEditor&) = delete;
    WordEditor& operator=(const WordEditor&) = delete;

private:
    // Save the values currently entered in the form.
    void on_save_clicked();

    // Close the dialog without saving.
    void on_cancel_clicked();
    void on_clear_clicked();
    void update_database_status();

    // Provides access to vocabulary database operations.
    Database& database_;
    WordRepository& repository_;
    std::string editing_id_;

    // Main container for the editor form.
    Gtk::Box form_box_{Gtk::Orientation::VERTICAL, 12};
    Gtk::Frame destination_frame_;
    Gtk::Box destination_box_{Gtk::Orientation::VERTICAL, 4};
    Gtk::Label database_status_label_;
    Gtk::Label database_path_label_;
    Gtk::Label form_status_label_;

    // Vocabulary title field.
    Gtk::Label title_label_;
    Gtk::Entry title_entry_;

    // Language proficiency level selector.
    Gtk::Label level_label_;
    Gtk::ComboBoxText level_combo_;

    // Grammatical category field.
    Gtk::Label part_of_speech_label_;
    Gtk::Entry part_of_speech_entry_;

    // Pronunciation field.
    Gtk::Label pronunciation_label_;
    Gtk::Entry pronunciation_entry_;

    // Definition field supporting multiple lines.
    Gtk::Label definition_label_;
    Gtk::TextView definition_entry_;
    Gtk::ScrolledWindow definition_scroll_;

    // Translation field.
    Gtk::Label translation_label_;
    Gtk::Entry translation_entry_;

    // Additional notes field supporting multiple lines.
    Gtk::Label notes_label_;
    Gtk::TextView notes_entry_;
    Gtk::ScrolledWindow notes_scroll_;

    struct DynamicField {
        Gtk::Box* row = nullptr;
        Gtk::Entry* first = nullptr;
        Gtk::Entry* second = nullptr;
        Gtk::Button* remove_button = nullptr;
    };

    Gtk::ScrolledWindow form_scroll_;
    Gtk::Box translations_box_{Gtk::Orientation::VERTICAL, 6};
    Gtk::Box synonyms_box_{Gtk::Orientation::VERTICAL, 6};
    Gtk::Box antonyms_box_{Gtk::Orientation::VERTICAL, 6};
    Gtk::Box examples_box_{Gtk::Orientation::VERTICAL, 6};
    Gtk::Box collocations_box_{Gtk::Orientation::VERTICAL, 6};
    Gtk::Box family_box_{Gtk::Orientation::VERTICAL, 6};

    Gtk::Button add_translation_button_{"Add translation"};
    Gtk::Button add_synonym_button_{"Add synonym"};
    Gtk::Button add_antonym_button_{"Add antonym"};
    Gtk::Button add_example_button_{"Add example"};
    Gtk::Button add_collocation_button_{"Add collocation"};
    Gtk::Button add_family_button_{"Add word-family item"};

    std::vector<DynamicField> translation_fields_;
    std::vector<DynamicField> synonym_fields_;
    std::vector<DynamicField> antonym_fields_;
    std::vector<DynamicField> example_fields_;
    std::vector<DynamicField> collocation_fields_;
    std::vector<DynamicField> family_fields_;

    DynamicField* add_dynamic_field(
        Gtk::Box& container,
        std::vector<DynamicField>& fields,
        const std::string& first_placeholder,
        const std::string& second_placeholder = {},
        const std::string& first_value = {},
        const std::string& second_value = {});

    void clear_dynamic_fields(
        Gtk::Box& container,
        std::vector<DynamicField>& fields);

    // Container for the form action buttons.
    Gtk::Box button_box_{Gtk::Orientation::HORIZONTAL, 8};

    // Save the vocabulary entry.
    Gtk::Button clear_button_{"Clear"};
    Gtk::Button save_button_;

    // Cancel editing without saving.
    Gtk::Button cancel_button_;

    void clear_form();
};

} // namespace gxx
