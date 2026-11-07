
#include "gxx/main_window.hpp"
#include "gxx/debug.hpp"

#include <gtkmm.h>
#include <iostream>
#include <string>
#include <filesystem>
#include <functional>
#include <utility>
#include <vector>

#include "gxx/search_view.hpp"
#include "gxx/settings_view.hpp"
#include "gxx/about_view.hpp"
#include "gxx/word_editor.hpp"
#include "gxx/dictionary_view.hpp"

namespace gxx {

namespace {

class WordDetailsView : public Gtk::Box {
public:
    WordDetailsView(const gxx::Word& word, std::function<void()> on_back,
                    std::function<void(const gxx::Word&)> on_edit)
        : Gtk::Box(Gtk::Orientation::VERTICAL, 12),
          on_back_(std::move(on_back)),
          on_edit_(std::move(on_edit)),
          back_button_("Back to results"),
          edit_button_("Edit"),
          title_label_(word.title) {
        set_margin(20);
        set_spacing(14);

        back_button_.set_halign(Gtk::Align::START);
        title_label_.add_css_class("title-1");
        title_label_.set_xalign(0.0f);
        title_label_.set_selectable(true);
        title_label_.set_wrap(true);

        auto* actions = Gtk::make_managed<Gtk::Box>(
            Gtk::Orientation::HORIZONTAL, 8);
        actions->append(back_button_);
        actions->append(edit_button_);
        append(*actions);
        append(title_label_);

        std::string metadata;
        if (!word.pronunciation.empty())
            metadata += word.pronunciation;
        if (!word.part_of_speech.empty()) {
            if (!metadata.empty()) metadata += "   ";
            metadata += word.part_of_speech;
        }
        if (!word.level.empty()) {
            if (!metadata.empty()) metadata += "   ";
            metadata += word.level;
        }

        add_section("Word information", metadata);
        add_section("Definition", word.definition);
        add_section("Translations", join(word.translations));
        add_section("Synonyms", join(word.synonyms));
        add_section("Antonyms", join(word.antonyms));

        std::string examples;
        for (const auto& example : word.examples) {
            if (!examples.empty()) examples += "\n\n";
            examples += example.english;
            if (!example.translation.empty())
                examples += "\n  " + example.translation;
        }
        add_section("Examples", examples);
        add_section("Collocations", join(word.collocations));

        std::string family;
        for (const auto& item : word.word_family) {
            if (!family.empty()) family += "\n";
            family += item.word;
            if (!item.part_of_speech.empty())
                family += " (" + item.part_of_speech + ")";
        }
        add_section("Word family", family);
        add_section("Notes", word.notes);
        add_section("Created", word.created_at);
        add_section("Last updated", word.updated_at);

        back_button_.signal_clicked().connect([this]() {
            if (on_back_) on_back_();
        });

        edit_button_.signal_clicked().connect([this, word]() {
            if (on_edit_) on_edit_(word);
        });
    }

private:
    static std::string join(const std::vector<std::string>& items) {
        std::string result;
        for (const auto& item : items) {
            if (item.empty()) continue;
            if (!result.empty()) result += "\n";
            result += "- " + item;
        }
        return result;
    }

    void add_section(const std::string& heading,
                     const std::string& value) {
        auto* frame = Gtk::make_managed<Gtk::Frame>();
        auto* box = Gtk::make_managed<Gtk::Box>(
            Gtk::Orientation::VERTICAL, 6);
        box->set_margin(12);

        auto* title = Gtk::make_managed<Gtk::Label>(heading);
        title->set_xalign(0.0f);
        title->add_css_class("heading");
        box->append(*title);

        auto* content = Gtk::make_managed<Gtk::Label>(
           value.empty()
               ? "No information saved.\nEdit this word to add information."
               : value
       );

       if (value.empty()) {
           content->add_css_class("error");
           content->set_tooltip_text(
               "Select Edit to add information to this section.");
       }
        content->set_xalign(0.0f);
        content->set_wrap(true);
        content->set_selectable(true);
        content->set_valign(Gtk::Align::START);
        content->set_max_width_chars(80);
        box->append(*content);

        frame->set_child(*box);
        append(*frame);
    }

    std::function<void()> on_back_;
    std::function<void(const gxx::Word&)> on_edit_;
    Gtk::Button back_button_;
    Gtk::Button edit_button_;
    Gtk::Label title_label_;
};

} // namespace


MainWindow::MainWindow()
    : database_(),
      word_repository_(database_),
      settings_service_(database_), header_bar_(),
      root_box_(Gtk::Orientation::VERTICAL),
      content_box_(Gtk::Orientation::VERTICAL),
      nav_box_(Gtk::Orientation::HORIZONTAL),
      title_label_("GXX Vocabulary"),
      search_button_("Search"),
      dictionary_button_("Dictionary"),
      add_word_button_("Add Word"),
      settings_button_("Settings"),
      status_label_("Ready") {

    // Open the project-local database and initialize its tables.
    const auto db_path =
        std::filesystem::path("/home/mamad/.gxx/db/en2fa.db");

    if (!database_.open(db_path.string())) {
        std::cerr << "[DB ERROR] Open failed: "
                  << database_.last_error() << '\n';
        status_label_.set_text("Database open failed");
    } else if (!word_repository_.initialize()) {
        std::cerr << "[DB ERROR] Word table initialization failed: "
                  << database_.last_error() << '\n';
        status_label_.set_text("Word table initialization failed");
    } else if (!settings_service_.initialize()) {
        std::cerr << "[DB ERROR] Settings initialization failed: "
                  << database_.last_error() << '\n';
        status_label_.set_text("Settings initialization failed");
    } else {
        std::cerr << "[DB] Opened: " << database_.path() << '\n';
    }

    // Configure the main application window.
    set_title("GXX");
    set_resizable(true);
    set_default_size(1000, 700);
    set_child(root_box_);

    // Use native GNOME window decorations and controls.
    set_decorated(true);

    // Configure the root layout.
    root_box_.set_spacing(0);
    root_box_.append(nav_box_);
    scroll_window_.set_child(content_box_);
    scroll_window_.set_policy(Gtk::PolicyType::NEVER, Gtk::PolicyType::AUTOMATIC);
    scroll_window_.set_vexpand(true);
    root_box_.append(scroll_window_);
    root_box_.append(status_label_);

    // Configure the navigation bar.
    nav_box_.set_spacing(8);
    nav_box_.set_margin(12);
    nav_box_.append(search_button_);
    nav_box_.append(dictionary_button_);
    nav_box_.append(add_word_button_);
    nav_box_.append(settings_button_);
   nav_box_.append(about_button_);

    // Align the navigation controls to the left.
    nav_box_.set_halign(Gtk::Align::START);

    // Configure the content area.
    content_box_.set_spacing(12);
    content_box_.set_margin(16);
    content_box_.set_vexpand(true);

    // Configure the status bar.
    status_label_.set_xalign(0.0f);
    status_label_.set_margin(8);
    status_label_.add_css_class("dim-label");

    // Connect navigation buttons to their handlers.
    search_button_.signal_clicked().connect(
        [this]() { show_search(); }
    );

    dictionary_button_.signal_clicked().connect(
        sigc::mem_fun(*this, &MainWindow::show_dictionary)
    );

    add_word_button_.signal_clicked().connect(
        [this]() { show_word_editor(); }
    );

    settings_button_.signal_clicked().connect(
        sigc::mem_fun(*this, &MainWindow::show_settings)
    );

   about_button_.signal_clicked().connect(
       sigc::mem_fun(*this, &MainWindow::show_about)
   );

    // Display the search view when the application starts.
    show_search();
}

void MainWindow::clear_content() {
    // Remove all widgets from the content area.
    while (auto* child = content_box_.get_first_child()) {
        content_box_.remove(*child);
    }
}

void MainWindow::show_search(std::string initial_query) {
    debug::Scope trace("MainWindow::show_search");
    // Clear the previous view before displaying a new one.
    std::cerr << "[DEBUG] Search clicked\n";
    clear_content();

    // Create and display the search interface.
    auto* view = Gtk::make_managed<SearchView>(
        word_repository_,
        [this](const Word& word, const std::string& query) {
            show_word_details(word, true, query);
        },
        std::move(initial_query)
    );
    content_box_.append(*view);
    debug::log("VIEW", "SearchView attached to content_box");
    status_label_.set_text("Search vocabulary");
}

void MainWindow::show_dictionary() {
    debug::Scope trace("MainWindow::show_dictionary");
    // Clear the previous view before displaying a new one.
    std::cerr << "[DEBUG] Dictionary clicked\n";
    clear_content();

    // Create and display the dictionary interface.
    auto* view = Gtk::make_managed<DictionaryView>(
        word_repository_,
        [this](const Word& word) {
            show_word_details(word, false);
        }
    );
    content_box_.append(*view);
    debug::log("VIEW", "DictionaryView attached to content_box");
    status_label_.set_text("Browse vocabulary");
}

void MainWindow::show_word_details(
    const Word& word,
    bool from_search,
    const std::string& query
) {
    clear_content();

    auto* view = Gtk::make_managed<WordDetailsView>(
        word,
        [this, from_search, query]() {
            if (from_search) {
                show_search(query);
            } else {
                show_dictionary();
            }
        },
        [this](const Word& selected_word) {
            show_word_editor(&selected_word);
        }
    );

    content_box_.append(*view);
    status_label_.set_text("Word details: " + word.title);
}

void MainWindow::show_word_editor(const Word* word) {
    debug::Scope trace("MainWindow::show_word_editor");

    // Keep one editor instance alive until its close request is handled.
    if (word_editor_) {
        word_editor_->present();
        word_editor_->set_visible(true);
        return;
    }

    word_editor_ = std::make_unique<WordEditor>(database_, word_repository_, word);
    word_editor_->set_transient_for(*this);
    word_editor_->set_modal(true);

    word_editor_->signal_close_request().connect(
        [this]() -> bool {
            // Defer destruction until GTK finishes processing the close event.
            Glib::signal_idle().connect_once([this]() {
                word_editor_.reset();
            });
            return false;
        },
        false
    );

    word_editor_->present();
    status_label_.set_text(word ? "Edit vocabulary entry" : "Add a vocabulary entry");
}

void MainWindow::show_settings() {
    debug::Scope trace("MainWindow::show_settings");
    // Clear the previous view before displaying a new one.
    std::cerr << "[DEBUG] Settings clicked\n";
    clear_content();

    // Create and display the settings interface.
    auto* view = Gtk::make_managed<SettingsView>(database_, word_repository_, settings_service_);
    content_box_.append(*view);
    debug::log("VIEW", "SettingsView attached to content_box");
    status_label_.set_text("Application settings");
}

} // namespace gxx

void gxx::MainWindow::show_about() {
    clear_content();
    auto* view = Gtk::make_managed<gxx::AboutView>();
    content_box_.append(*view);
    status_label_.set_text("About GXX");
}
