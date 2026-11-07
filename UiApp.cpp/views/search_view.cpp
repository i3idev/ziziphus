#include "gxx/search_view.hpp"

#include <string>
#include <vector>
#include <utility>
#include <cctype>

#include "gxx/word.hpp"


namespace {

std::string escape_markup(const std::string& text) {
    gchar* escaped = g_markup_escape_text(text.c_str(), -1);
    std::string result = escaped ? escaped : "";
    g_free(escaped);
    return result;
}

std::string highlight_markup(
    const std::string& text,
    const std::string& query
) {
    if (text.empty() || query.empty())
        return escape_markup(text);

    auto lower = [](std::string value) {
        for (char& c : value)
            c = static_cast<char>(
                std::tolower(static_cast<unsigned char>(c))
            );
        return value;
    };

    const std::string source = lower(text);
    const std::string needle = lower(query);

    std::string result;
    std::size_t pos = 0;

    while (pos < text.size()) {
        const std::size_t match = source.find(needle, pos);

        if (match == std::string::npos) {
            result += escape_markup(text.substr(pos));
            break;
        }

        result += escape_markup(text.substr(pos, match - pos));
        result += "<span background=\"#ffe066\" foreground=\"#332700\">";
        result += escape_markup(text.substr(match, query.size()));
        result += "</span>";

        pos = match + query.size();
    }

    return result;
}

void set_highlighted(Gtk::Label& label,
                     const std::string& text,
                     const std::string& query) {
    label.set_use_markup(true);
    label.set_markup(highlight_markup(text, query));
}

} // namespace

namespace gxx {

SearchView::SearchView(
    WordRepository& repository,
    std::function<void(const Word&, const std::string&)> on_open,
    std::string initial_query
)
    : repository_(repository),
      on_open_(std::move(on_open)),
      initial_query_(std::move(initial_query)),
      title_label_("Search vocabulary"),
      search_button_("Search"),
      clear_button_("Clear"),
      result_label_("Search by word, translation, or definition.") {

    set_orientation(Gtk::Orientation::VERTICAL);
    set_spacing(16);
    set_margin(20);

    title_label_.set_xalign(0.0f);
    title_label_.add_css_class("title-2");

    search_entry_.set_placeholder_text(
        "Enter a word, translation, or definition"
    );
    search_entry_.set_text(initial_query_);
    search_entry_.set_hexpand(true);

    search_button_.add_css_class("suggested-action");
    search_button_.set_tooltip_text("Search the vocabulary database");

    clear_button_.set_tooltip_text("Clear the search and its results");

    search_box_.set_orientation(Gtk::Orientation::HORIZONTAL);
    search_box_.set_spacing(8);
    search_box_.append(search_entry_);
    search_box_.append(search_button_);
    search_box_.append(clear_button_);

    result_label_.set_xalign(0.0f);
    result_label_.add_css_class("dim-label");
    result_label_.set_wrap(true);

    results_box_.set_orientation(Gtk::Orientation::VERTICAL);
    results_box_.set_spacing(8);
    results_box_.set_hexpand(true);

    results_scroll_.set_child(results_box_);
    results_scroll_.set_policy(
        Gtk::PolicyType::NEVER,
        Gtk::PolicyType::AUTOMATIC
    );
    results_scroll_.set_vexpand(true);
    results_scroll_.set_has_frame(false);

    append(title_label_);
    append(search_box_);
    append(result_label_);
    append(results_scroll_);

    search_button_.signal_clicked().connect(
        sigc::mem_fun(*this, &SearchView::on_search_clicked)
    );

    search_entry_.signal_search_changed().connect(
        sigc::mem_fun(*this, &SearchView::on_search_clicked)
    );

    search_entry_.signal_activate().connect(
        sigc::mem_fun(*this, &SearchView::on_search_clicked)
    );

    clear_button_.signal_clicked().connect(
        sigc::mem_fun(*this, &SearchView::on_clear_clicked)
    );

    if (!initial_query_.empty()) {
        on_search_clicked();
    }
}

void SearchView::on_clear_clicked() {
    search_entry_.set_text("");

    while (auto* child = results_box_.get_first_child()) {
        results_box_.remove(*child);
    }

    result_label_.set_text(
        "Search by word, translation, or definition."
    );

    search_entry_.grab_focus();
}

void SearchView::on_search_clicked() {
    const std::string query = search_entry_.get_text();

    while (auto* child = results_box_.get_first_child()) {
        results_box_.remove(*child);
    }

    if (query.find_first_not_of(" \t\n\r") == std::string::npos) {
        result_label_.set_text("Enter a word to start searching.");
        return;
    }

    std::vector<Word> results;

    if (!repository_.search(query, results)) {
        result_label_.set_text(
            "Search failed. Check the database connection."
        );
        return;
    }

    if (results.empty()) {
        result_label_.set_text("No matching vocabulary found.");
        return;
    }

    result_label_.set_text(
        std::to_string(results.size()) + " matching word(s)"
    );

    for (const auto& word : results) {
        auto* open_button = Gtk::make_managed<Gtk::Button>();
        auto* row = Gtk::make_managed<Gtk::Box>(
            Gtk::Orientation::VERTICAL,
            6
        );

        row->set_margin(12);

        auto* word_title = Gtk::make_managed<Gtk::Label>(word.title);
        set_highlighted(*word_title, word.title, query);
        word_title->set_xalign(0.0f);
        word_title->set_selectable(true);
        word_title->add_css_class("heading");

        row->append(*word_title);

        if (!word.pronunciation.empty()) {
            auto* pronunciation =
                Gtk::make_managed<Gtk::Label>(word.pronunciation);
            set_highlighted(*pronunciation, word.pronunciation, query);
            pronunciation->set_xalign(0.0f);
            pronunciation->add_css_class("dim-label");
            pronunciation->set_selectable(true);
            row->append(*pronunciation);
        }

        if (!word.translations.empty() && !word.translations.front().empty()) {
            auto* translation =
                Gtk::make_managed<Gtk::Label>();
            set_highlighted(*translation, word.translations.front(), query);
            translation->set_xalign(0.0f);
            translation->set_wrap(true);
            translation->set_selectable(true);
            row->append(*translation);
        }

        if (!word.definition.empty()) {
            auto* definition =
                Gtk::make_managed<Gtk::Label>(word.definition);
            set_highlighted(*definition, word.definition, query);
            definition->set_xalign(0.0f);
            definition->set_wrap(true);
            definition->set_selectable(true);
            row->append(*definition);
        }

        const std::string metadata =
            (word.level.empty() ? "Unlevelled" : word.level) +
            (word.part_of_speech.empty()
                ? ""
                : "  |  " + word.part_of_speech);

        auto* details = Gtk::make_managed<Gtk::Label>(metadata);
        details->set_xalign(0.0f);
        details->add_css_class("dim-label");
        row->append(*details);

        open_button->set_child(*row);
        open_button->set_hexpand(true);
        open_button->set_halign(Gtk::Align::FILL);
        open_button->set_tooltip_text("Open full word details");
        open_button->signal_clicked().connect([this, word]() {
            if (on_open_) {
                on_open_(word, search_entry_.get_text());
            }
        });
        results_box_.append(*open_button);
    }
}

} // namespace gxx
