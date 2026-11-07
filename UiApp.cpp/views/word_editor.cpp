#include "gxx/word_editor.hpp"
#include "gxx/debug.hpp"
#include "gxx/word.hpp"

#include <algorithm>
#include <exception>
#include <string>

namespace gxx {

WordEditor::WordEditor(Database& database, WordRepository& repository,
                       const Word* word)
    : database_(database),
      repository_(repository),
      editing_id_(word ? word->id : ""),
      title_label_("Word *"),
      level_label_("English level"),
      part_of_speech_label_("Part of speech"),
      pronunciation_label_("Pronunciation"),
      definition_label_("Definition"),
      translation_label_("Translations"),
      notes_label_("Notes"),
      save_button_("Save word"),
      cancel_button_("Cancel") {
    set_title(word ? "GXX - Edit Word" : "GXX - Word Editor");
    set_default_size(640, 820);
    set_modal(true);

    form_box_.set_margin(16);
    form_box_.set_spacing(10);

    destination_frame_.set_label("Save destination");
    destination_box_.set_margin(10);
    database_status_label_.set_xalign(0.0f);
    database_path_label_.set_xalign(0.0f);
    database_path_label_.set_wrap(true);
    database_path_label_.set_selectable(true);
    database_status_label_.add_css_class("heading");
    database_path_label_.add_css_class("dim-label");
    destination_box_.append(database_status_label_);
    destination_box_.append(database_path_label_);
    destination_frame_.set_child(destination_box_);
    form_box_.append(destination_frame_);

    title_entry_.set_placeholder_text("e.g. abandon");
    part_of_speech_entry_.set_placeholder_text("noun, verb, adjective...");
    pronunciation_entry_.set_placeholder_text("IPA or pronunciation guide");

    for (const char* level : {"A1", "A2", "B1", "B2", "C1"})
        level_combo_.append(level);
    level_combo_.set_active_text("A1");

    definition_entry_.set_wrap_mode(Gtk::WrapMode::WORD_CHAR);
    notes_entry_.set_wrap_mode(Gtk::WrapMode::WORD_CHAR);

    definition_scroll_.set_child(definition_entry_);
    definition_scroll_.set_min_content_height(100);
    definition_scroll_.set_policy(
        Gtk::PolicyType::AUTOMATIC, Gtk::PolicyType::AUTOMATIC);

    notes_scroll_.set_child(notes_entry_);
    notes_scroll_.set_min_content_height(80);
    notes_scroll_.set_policy(
        Gtk::PolicyType::AUTOMATIC, Gtk::PolicyType::AUTOMATIC);

    form_box_.append(title_label_);
    form_box_.append(title_entry_);
    form_box_.append(level_label_);
    form_box_.append(level_combo_);
    form_box_.append(part_of_speech_label_);
    form_box_.append(part_of_speech_entry_);
    form_box_.append(pronunciation_label_);
    form_box_.append(pronunciation_entry_);
    form_box_.append(definition_label_);
    form_box_.append(definition_scroll_);

    auto add_section = [this](const char* title, Gtk::Box& fields,
                               Gtk::Button& add_button) {
        auto* heading = Gtk::make_managed<Gtk::Label>(title);
        heading->set_xalign(0.0f);
        heading->add_css_class("heading");
        form_box_.append(*heading);
        form_box_.append(fields);
        add_button.set_halign(Gtk::Align::START);
        form_box_.append(add_button);
    };

    add_section("Translations", translations_box_, add_translation_button_);
    add_section("Synonyms", synonyms_box_, add_synonym_button_);
    add_section("Antonyms", antonyms_box_, add_antonym_button_);
    add_section("Examples (English / translation)", examples_box_, add_example_button_);
    add_section("Collocations", collocations_box_, add_collocation_button_);
    add_section("Word family (word / part of speech)", family_box_, add_family_button_);

    form_box_.append(notes_label_);
    form_box_.append(notes_scroll_);

    form_status_label_.set_xalign(0.0f);
    form_status_label_.set_wrap(true);
    form_box_.append(form_status_label_);

    button_box_.set_spacing(8);
    button_box_.set_halign(Gtk::Align::END);
    button_box_.append(clear_button_);
    button_box_.append(cancel_button_);
    button_box_.append(save_button_);
    form_box_.append(button_box_);

    form_scroll_.set_child(form_box_);
    form_scroll_.set_policy(
        Gtk::PolicyType::AUTOMATIC, Gtk::PolicyType::AUTOMATIC);
    set_child(form_scroll_);

    add_translation_button_.signal_clicked().connect([this] {
        add_dynamic_field(translations_box_, translation_fields_,
                          "Translation");
    });
    add_synonym_button_.signal_clicked().connect([this] {
        add_dynamic_field(synonyms_box_, synonym_fields_, "Synonym");
    });
    add_antonym_button_.signal_clicked().connect([this] {
        add_dynamic_field(antonyms_box_, antonym_fields_, "Antonym");
    });
    add_example_button_.signal_clicked().connect([this] {
        add_dynamic_field(examples_box_, example_fields_,
                          "English example", "Translation");
    });
    add_collocation_button_.signal_clicked().connect([this] {
        add_dynamic_field(collocations_box_, collocation_fields_,
                          "Collocation");
    });
    add_family_button_.signal_clicked().connect([this] {
        add_dynamic_field(family_box_, family_fields_,
                          "Related word", "Part of speech");
    });

    if (word) {
        title_entry_.set_text(word->title);
        level_combo_.set_active_text(word->level.empty() ? "A1" : word->level);
        part_of_speech_entry_.set_text(word->part_of_speech);
        pronunciation_entry_.set_text(word->pronunciation);
        definition_entry_.get_buffer()->set_text(word->definition);
        notes_entry_.get_buffer()->set_text(word->notes);

        if (word->translations.empty())
            add_dynamic_field(translations_box_, translation_fields_, "Translation");
        else
            for (const auto& value : word->translations)
                add_dynamic_field(translations_box_, translation_fields_,
                                  "Translation", {}, value);

        if (word->synonyms.empty())
            add_dynamic_field(synonyms_box_, synonym_fields_, "Synonym");
        else
            for (const auto& value : word->synonyms)
                add_dynamic_field(synonyms_box_, synonym_fields_, "Synonym", {}, value);

        if (word->antonyms.empty())
            add_dynamic_field(antonyms_box_, antonym_fields_, "Antonym");
        else
            for (const auto& value : word->antonyms)
                add_dynamic_field(antonyms_box_, antonym_fields_, "Antonym", {}, value);

        if (word->examples.empty())
            add_dynamic_field(examples_box_, example_fields_,
                              "English example", "Translation");
        else
            for (const auto& value : word->examples)
                add_dynamic_field(examples_box_, example_fields_,
                                  "English example", "Translation",
                                  value.english, value.translation);

        if (word->collocations.empty())
            add_dynamic_field(collocations_box_, collocation_fields_, "Collocation");
        else
            for (const auto& value : word->collocations)
                add_dynamic_field(collocations_box_, collocation_fields_,
                                  "Collocation", {}, value);

        if (word->word_family.empty())
            add_dynamic_field(family_box_, family_fields_,
                              "Related word", "Part of speech");
        else
            for (const auto& value : word->word_family)
                add_dynamic_field(family_box_, family_fields_,
                                  "Related word", "Part of speech",
                                  value.word, value.part_of_speech);

        save_button_.set_label("Save changes");
    } else {
        add_dynamic_field(translations_box_, translation_fields_, "Translation");
        add_dynamic_field(synonyms_box_, synonym_fields_, "Synonym");
        add_dynamic_field(antonyms_box_, antonym_fields_, "Antonym");
        add_dynamic_field(examples_box_, example_fields_,
                          "English example", "Translation");
        add_dynamic_field(collocations_box_, collocation_fields_, "Collocation");
        add_dynamic_field(family_box_, family_fields_,
                          "Related word", "Part of speech");
    }

    save_button_.signal_clicked().connect(
        sigc::mem_fun(*this, &WordEditor::on_save_clicked));
    cancel_button_.signal_clicked().connect(
        sigc::mem_fun(*this, &WordEditor::on_cancel_clicked));
    clear_button_.signal_clicked().connect(
        sigc::mem_fun(*this, &WordEditor::on_clear_clicked));

    update_database_status();
    title_entry_.grab_focus();
}

WordEditor::DynamicField* WordEditor::add_dynamic_field(
    Gtk::Box& container, std::vector<DynamicField>& fields,
    const std::string& first_placeholder,
    const std::string& second_placeholder,
    const std::string& first_value,
    const std::string& second_value) {

    DynamicField field;
    field.row = Gtk::make_managed<Gtk::Box>(Gtk::Orientation::HORIZONTAL, 6);
    field.first = Gtk::make_managed<Gtk::Entry>();
    field.first->set_placeholder_text(first_placeholder);
    field.first->set_hexpand(true);
    field.first->set_text(first_value);
    field.row->append(*field.first);

    if (!second_placeholder.empty()) {
        field.second = Gtk::make_managed<Gtk::Entry>();
        field.second->set_placeholder_text(second_placeholder);
        field.second->set_hexpand(true);
        field.second->set_text(second_value);
        field.row->append(*field.second);
    }

    field.remove_button = Gtk::make_managed<Gtk::Button>("Remove");
    field.row->append(*field.remove_button);
    container.append(*field.row);
    fields.push_back(field);

    auto* row = field.row;
    field.remove_button->signal_clicked().connect([&container, &fields, row] {
        container.remove(*row);
        fields.erase(
            std::remove_if(fields.begin(), fields.end(),
                [row](const DynamicField& item) { return item.row == row; }),
            fields.end());
    });

    return &fields.back();
}

void WordEditor::clear_dynamic_fields(
    Gtk::Box& container, std::vector<DynamicField>& fields) {
    for (auto& field : fields)
        container.remove(*field.row);
    fields.clear();
}

void WordEditor::update_database_status() {
    if (database_.is_open()) {
        database_status_label_.set_text("Connected");
        database_path_label_.set_text(database_.path());
        save_button_.set_sensitive(true);
    } else {
        database_status_label_.set_text("Not connected");
        database_path_label_.set_text("No active database. Open one in Settings.");
        save_button_.set_sensitive(false);
    }
}

void WordEditor::clear_form() {
    title_entry_.set_text("");
    part_of_speech_entry_.set_text("");
    pronunciation_entry_.set_text("");
    definition_entry_.get_buffer()->set_text("");
    notes_entry_.get_buffer()->set_text("");
    level_combo_.set_active_text("A1");
    form_status_label_.set_text("");

    clear_dynamic_fields(translations_box_, translation_fields_);
    clear_dynamic_fields(synonyms_box_, synonym_fields_);
    clear_dynamic_fields(antonyms_box_, antonym_fields_);
    clear_dynamic_fields(examples_box_, example_fields_);
    clear_dynamic_fields(collocations_box_, collocation_fields_);
    clear_dynamic_fields(family_box_, family_fields_);

    add_dynamic_field(translations_box_, translation_fields_, "Translation");
    add_dynamic_field(synonyms_box_, synonym_fields_, "Synonym");
    add_dynamic_field(antonyms_box_, antonym_fields_, "Antonym");
    add_dynamic_field(examples_box_, example_fields_,
                      "English example", "Translation");
    add_dynamic_field(collocations_box_, collocation_fields_, "Collocation");
    add_dynamic_field(family_box_, family_fields_,
                      "Related word", "Part of speech");

    title_entry_.grab_focus();
}

void WordEditor::on_clear_clicked() {
    clear_form();
}

void WordEditor::on_save_clicked() {
    update_database_status();

    if (!database_.is_open()) {
        form_status_label_.set_text(
            "Cannot save: no database is currently connected.");
        return;
    }

    Word word;
    word.id = editing_id_;
    word.title = title_entry_.get_text();
    word.level = level_combo_.get_active_text();
    word.part_of_speech = part_of_speech_entry_.get_text();
    word.pronunciation = pronunciation_entry_.get_text();
    word.definition = definition_entry_.get_buffer()->get_text();
    word.notes = notes_entry_.get_buffer()->get_text();

    auto trim = [](std::string value) {
        const auto first = value.find_first_not_of(" \\t\\n\\r");
        if (first == std::string::npos)
            return std::string{};
        const auto last = value.find_last_not_of(" \\t\\n\\r");
        return value.substr(first, last - first + 1);
    };

    word.title = trim(word.title);
    if (word.title.empty()) {
        form_status_label_.set_text("Word is required.");
        title_entry_.grab_focus();
        return;
    }

    auto collect_values = [&trim](const std::vector<DynamicField>& fields,
                                  std::vector<std::string>& output) {
        for (const auto& field : fields) {
            auto value = trim(field.first->get_text());
            if (!value.empty())
                output.push_back(value);
        }
    };

    collect_values(translation_fields_, word.translations);
    collect_values(synonym_fields_, word.synonyms);
    collect_values(antonym_fields_, word.antonyms);
    collect_values(collocation_fields_, word.collocations);

    for (const auto& field : example_fields_) {
        auto english = trim(field.first->get_text());
        auto translation = field.second ? trim(field.second->get_text()) : "";
        if (!english.empty() || !translation.empty())
            word.examples.push_back({english, translation});
    }

    for (const auto& field : family_fields_) {
        auto related_word = trim(field.first->get_text());
        auto pos = field.second ? trim(field.second->get_text()) : "";
        if (!related_word.empty() || !pos.empty())
            word.word_family.push_back({related_word, pos});
    }

    try {
        save_button_.set_sensitive(false);
        form_status_label_.set_text("Saving...");

        const bool saved = editing_id_.empty()
            ? repository_.add(word)
            : repository_.update(word);

        if (saved) {
            debug::log("INFO", "Word saved successfully: " + word.title);
            close();
            return;
        }

        form_status_label_.set_text(
            "Could not save. The word may already exist in this database.");
        debug::log("ERROR", "WordRepository save returned false");
    } catch (const std::exception& e) {
        form_status_label_.set_text(std::string("Save failed: ") + e.what());
        debug::log("ERROR", std::string("Save failed: ") + e.what());
    }

    save_button_.set_sensitive(true);
}

void WordEditor::on_cancel_clicked() {
    close();
}

} // namespace gxx
