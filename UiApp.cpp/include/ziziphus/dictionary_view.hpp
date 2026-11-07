
#pragma once

#include <gtkmm.h>
#include <functional>
#include "gxx/word.hpp"

#include "gxx/word_repository.hpp"

namespace gxx {

/*
 * DictionaryView
 *
 * Displays vocabulary entries stored in the GXX database.
 *
 * Responsibilities:
 * - Display the vocabulary list.
 * - Show each word's translation, level, and definition.
 * - Reload the list when requested.
 * - Display loading errors and empty-state messages.
 *
 * The repository is borrowed by reference and is not owned
 * or destroyed by this view.
 */
class DictionaryView : public Gtk::Box {
public:
    /*
     * Create the dictionary view.
     *
     * The supplied repository must remain alive while this
     * view is in use.
     */
    DictionaryView(
        WordRepository& repository,
        std::function<void(const Word&)> on_open
    );

    // Prevent copying GTK widgets and their associated state.
    DictionaryView(const DictionaryView&) = delete;
    DictionaryView& operator=(const DictionaryView&) = delete;

private:
    // Load vocabulary entries from the repository and update the UI.
    void load_words();

    // Provides access to vocabulary database operations.
    WordRepository& repository_;
    std::function<void(const Word&)> on_open_;

    // Main page heading.
    Gtk::Label title_label_;

    // Button used to reload the vocabulary list.
    Gtk::Button refresh_button_;

    // Arranges the heading and refresh button horizontally.
    Gtk::Box toolbar_;

    // Contains the individual vocabulary rows.
    Gtk::Box words_box_;

    // Provides scrolling when the vocabulary list is long.
    Gtk::ScrolledWindow scrolled_window_;

    // Displays loading status, errors, or the number of words.
    Gtk::Label status_label_;
};

} // namespace gxx
