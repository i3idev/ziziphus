
#pragma once

#include <gtkmm.h>
#include <functional>
#include <string>
#include "gxx/word.hpp"

#include "gxx/word_repository.hpp"

namespace gxx {

/*
 * SearchView
 *
 * Provides the graphical interface for searching vocabulary.
 *
 * Responsibilities:
 * - Accept a search query from the user.
 * - Search for matching words through WordRepository.
 * - Display matching words and their translations.
 * - Show status messages when results are unavailable.
 *
 * The repository is borrowed by reference and is not owned
 * or destroyed by this view.
 */
class SearchView : public Gtk::Box {
public:
    /*
     * Construct the search interface.
     *
     * The repository must remain alive while this view is in use.
     */
    SearchView(
        WordRepository& repository,
        std::function<void(const Word&, const std::string&)> on_open,
        std::string initial_query = {}
    );

    // Prevent copying GTK widgets and their internal state.
    SearchView(const SearchView&) = delete;
    SearchView& operator=(const SearchView&) = delete;

private:
    // Execute a search using the current query.
    void on_search_clicked();
    void on_clear_clicked();

    // Provides access to vocabulary search operations.
    WordRepository& repository_;
    std::function<void(const Word&, const std::string&)> on_open_;
    std::string initial_query_;

    // Page heading.
    Gtk::Label title_label_;

    // Input field containing the user's search query.
    Gtk::SearchEntry search_entry_;

    // Button that starts the search operation.
    Gtk::Button search_button_;
    Gtk::Button clear_button_;

    // Displays the search result count or status message.
    Gtk::Label result_label_;

    // Contains the individual matching vocabulary entries.
    Gtk::Box results_box_;
    Gtk::ScrolledWindow results_scroll_;

    // Arranges the search field and button horizontally.
    Gtk::Box search_box_;
};

} // namespace gxx
