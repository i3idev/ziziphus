#pragma once

#include <memory>
#include <string>
#include "gxx/word.hpp"

#include <gtkmm.h>

#include "ziziphus/database.hpp"
#include "gxx/word_repository.hpp"
#include "gxx/word_editor.hpp"
#include "gxx/settings_service.hpp"

namespace gxx {

class MainWindow : public Gtk::Window {
public:
    MainWindow();
    ~MainWindow() override = default;

    MainWindow(const MainWindow&) = delete;
    MainWindow& operator=(const MainWindow&) = delete;

private:
    void clear_content();
    void show_search(std::string initial_query = {});
    void show_dictionary();
    void show_word_editor(const Word* word = nullptr);
    void show_settings();
    void show_about();
    void show_word_details(const Word& word, bool from_search,
                          const std::string& query = {});

    Database database_;
    WordRepository word_repository_;
    std::unique_ptr<WordEditor> word_editor_;
    SettingsService settings_service_;

    Gtk::ScrolledWindow scroll_window_;
    Gtk::Box root_box_{Gtk::Orientation::VERTICAL, 0};
    Gtk::HeaderBar header_bar_;
    Gtk::Box nav_box_{Gtk::Orientation::HORIZONTAL, 8};
    Gtk::Box content_box_{Gtk::Orientation::VERTICAL, 12};
    Gtk::Label title_label_{"GXX"};
    Gtk::Button search_button_{"Search"};
    Gtk::Button dictionary_button_{"Dictionary"};
    Gtk::Button add_word_button_{"Add word"};
    Gtk::Button settings_button_{"Settings"};
    Gtk::Button about_button_{"About"};
    Gtk::Label status_label_;
};

} // namespace gxx
