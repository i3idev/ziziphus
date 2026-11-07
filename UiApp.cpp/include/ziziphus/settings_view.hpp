#pragma once

#include <gtkmm.h>
#include <string>

#include "gxx/database.hpp"
#include "gxx/settings_service.hpp"
#include "gxx/word_repository.hpp"

namespace gxx {

class SettingsView : public Gtk::Box {
public:
    SettingsView(
        Database& database,
        WordRepository& repository,
        SettingsService& settings_service
    );

    SettingsView(const SettingsView&) = delete;
    SettingsView& operator=(const SettingsView&) = delete;

private:
    void refresh_database_info();
    void refresh_database_list();
    void select_database(const std::string& path);
    void show_database_schema();
    void choose_database();
    void create_database();
    void on_open_response(int response);
    void on_create_response(int response);
    void clear_schema();

    Database& database_;
    WordRepository& repository_;
    SettingsService& settings_service_;

    Gtk::ScrolledWindow page_scroll_;
    Gtk::Box page_content_{Gtk::Orientation::VERTICAL, 12};

    Gtk::Label title_label_;
    Gtk::Label database_path_label_;
    Gtk::Label database_info_label_;
    Gtk::Label status_label_;
    Gtk::Label databases_label_;
    Gtk::Box databases_box_{Gtk::Orientation::VERTICAL, 6};

    Gtk::ComboBoxText language_combo_;
    Gtk::ComboBoxText level_combo_;
    Gtk::ComboBoxText theme_combo_;

    Gtk::Button open_button_;
    Gtk::Button create_button_;
    Gtk::Button refresh_button_;

    Gtk::Box actions_box_;
    Gtk::Box schema_box_;
    Gtk::ScrolledWindow schema_scroll_;

    Glib::RefPtr<Gtk::FileChooserNative> open_dialog_;
    Glib::RefPtr<Gtk::FileChooserNative> create_dialog_;
};

} // namespace gxx
