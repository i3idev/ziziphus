
#include "gxx/dictionary_view.hpp"

#include <gtkmm.h>

#include <string>
#include <vector>
#include <utility>

#include "gxx/word.hpp"
#include "gxx/word_repository.hpp"

namespace gxx {

DictionaryView::DictionaryView(
    WordRepository& repository,
    std::function<void(const Word&)> on_open
)
    : Gtk::Box(Gtk::Orientation::VERTICAL, 12),
      repository_(repository),
      on_open_(std::move(on_open)),
      title_label_("Vocabulary"),
      refresh_button_("Refresh"),
      toolbar_(Gtk::Orientation::HORIZONTAL, 8),
      words_box_(Gtk::Orientation::VERTICAL, 8),
      status_label_("Loading vocabulary...") {

    // Configure the main dictionary layout.
    set_orientation(Gtk::Orientation::VERTICAL);
    set_spacing(12);
    set_margin(20);

    // Configure the dictionary heading.
    title_label_.add_css_class("title-1");

    // Configure the toolbar.
    toolbar_.set_orientation(Gtk::Orientation::HORIZONTAL);
    toolbar_.set_spacing(8);
    toolbar_.append(title_label_);
    toolbar_.append(refresh_button_);

    // Keep the heading on the left and the refresh button on the right.
    title_label_.set_hexpand(true);
    title_label_.set_xalign(0.0f);

    // Configure the list that contains vocabulary entries.
    words_box_.set_orientation(Gtk::Orientation::VERTICAL);
    words_box_.set_spacing(8);

    // Allow the vocabulary list to scroll independently.
    scrolled_window_.set_child(words_box_);
    scrolled_window_.set_vexpand(true);
    scrolled_window_.set_hexpand(true);
    scrolled_window_.set_policy(
        Gtk::PolicyType::AUTOMATIC,
        Gtk::PolicyType::AUTOMATIC
    );

    // Configure the status message.
    status_label_.set_xalign(0.0f);
    status_label_.add_css_class("dim-label");

    // Assemble the dictionary interface.
    append(toolbar_);
    append(scrolled_window_);
    append(status_label_);

    // Reload vocabulary when the user clicks Refresh.
    refresh_button_.signal_clicked().connect(
        sigc::mem_fun(*this, &DictionaryView::load_words)
    );

    // Load the vocabulary when the view is created.
    load_words();
}

void DictionaryView::load_words() {
    // Remove entries from the previous loading operation.
    while (auto* child = words_box_.get_first_child()) {
        words_box_.remove(*child);
    }

    // Prepare a container for the retrieved vocabulary.
    std::vector<Word> words;

    // Retrieve all vocabulary entries from the repository.
    if (!repository_.list(words)) {
        status_label_.set_text(
            "Could not load vocabulary. Check the database."
        );
        return;
    }

    // Display a message when the database contains no words.
    if (words.empty()) {
        status_label_.set_text("No vocabulary entries yet.");
        return;
    }

    // Display the number of retrieved vocabulary entries.
    status_label_.set_text(
        std::to_string(words.size()) + " word(s)"
    );

    // Create a visual row for each vocabulary entry.
    for (const auto& word : words) {
        auto* open_button = Gtk::make_managed<Gtk::Button>();
        auto* row = Gtk::make_managed<Gtk::Box>(
            Gtk::Orientation::VERTICAL,
            4
        );

        // Display the vocabulary title.
        auto* title = Gtk::make_managed<Gtk::Label>(word.title);
        title->set_xalign(0.0f);
        title->set_wrap(true);
        title->add_css_class("heading");

        // Display the translation.
        auto* translation =
            Gtk::make_managed<Gtk::Label>((word.translations.empty() ? std::string{} : word.translations.front()));
        translation->set_xalign(0.0f);
        translation->set_wrap(true);

        // Display the proficiency level and grammatical category.
        auto* details = Gtk::make_managed<Gtk::Label>(
            word.level + " | " + word.part_of_speech
        );
        details->set_xalign(0.0f);
        details->add_css_class("dim-label");

        // Display the definition when it is available.
        row->append(*title);
        row->append(*translation);
        row->append(*details);

        if (!word.definition.empty()) {
            auto* definition =
                Gtk::make_managed<Gtk::Label>(word.definition);

            definition->set_xalign(0.0f);
            definition->set_wrap(true);

            row->append(*definition);
        }

        // Add spacing and a separator between entries.
        row->set_margin(8);
        open_button->set_child(*row);
        open_button->set_hexpand(true);
        open_button->set_halign(Gtk::Align::FILL);
        open_button->set_tooltip_text("Open full word details");
        open_button->signal_clicked().connect([this, word]() {
            if (on_open_) {
                on_open_(word);
            }
        });
        words_box_.append(*open_button);

        auto* separator =
            Gtk::make_managed<Gtk::Separator>(
                Gtk::Orientation::HORIZONTAL
            );

        words_box_.append(*separator);
    }
}

} // namespace gxx
