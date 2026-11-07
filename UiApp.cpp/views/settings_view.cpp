#include "gxx/settings_view.hpp"

#include <sqlite3.h>
#include <filesystem>
#include <cstdlib>

#include <string>
#include <vector>

namespace gxx {
namespace {

std::string quote_identifier(const std::string& value) {
    std::string quoted = "\"";
    for (char ch : value) {
        if (ch == '"')
            quoted += '"';
        quoted += ch;
    }
    return quoted + '"';
}

} // namespace

SettingsView::SettingsView(
    Database& database,
    WordRepository& repository,
    SettingsService& settings_service
)
    : database_(database),
      repository_(repository),
      settings_service_(settings_service),
      title_label_("Settings"),
      database_path_label_(""),
      database_info_label_(""),
      status_label_(""),
      databases_label_("Available databases"),
      open_button_("Open database"),
      refresh_button_("Refresh"),
      actions_box_(Gtk::Orientation::HORIZONTAL, 8),
      schema_box_(Gtk::Orientation::VERTICAL, 10) {

    set_orientation(Gtk::Orientation::VERTICAL);
    set_spacing(0);
    set_hexpand(true);
    set_vexpand(true);

    page_content_.set_orientation(Gtk::Orientation::VERTICAL);
    page_content_.set_spacing(18);
    page_content_.set_margin(24);
    page_content_.set_hexpand(true);

    page_scroll_.set_policy(
        Gtk::PolicyType::NEVER,
        Gtk::PolicyType::AUTOMATIC
    );
    page_scroll_.set_child(page_content_);
    page_scroll_.set_hexpand(true);
    page_scroll_.set_vexpand(true);
    append(page_scroll_);

    title_label_.set_xalign(0.0f);
    title_label_.add_css_class("title-1");
    page_content_.append(title_label_);

    auto* subtitle = Gtk::make_managed<Gtk::Label>(
        "Manage database connections and inspect the SQLite schema."
    );
    subtitle->set_xalign(0.0f);
    subtitle->set_wrap(true);
    subtitle->add_css_class("dim-label");
    page_content_.append(*subtitle);

    auto* databases_frame = Gtk::make_managed<Gtk::Frame>();
    databases_frame->add_css_class("card");

    auto* databases_content = Gtk::make_managed<Gtk::Box>(
        Gtk::Orientation::VERTICAL, 12
    );
    databases_content->set_margin(16);

    databases_label_.set_xalign(0.0f);
    databases_label_.add_css_class("title-2");
    databases_content->append(databases_label_);

    auto* database_hint = Gtk::make_managed<Gtk::Label>(
        "Select a database to make it active."
    );
    database_hint->set_xalign(0.0f);
    database_hint->add_css_class("dim-label");
    databases_content->append(*database_hint);

    databases_box_.set_spacing(8);
    databases_content->append(databases_box_);
    databases_frame->set_child(*databases_content);
    page_content_.append(*databases_frame);

    actions_box_.set_halign(Gtk::Align::START);
    open_button_.add_css_class("suggested-action");
    open_button_.set_tooltip_text("Open an existing SQLite database");

    refresh_button_.signal_clicked().connect([this]() {
        refresh_database_list();
        refresh_database_info();
    });

    open_button_.signal_clicked().connect(
        sigc::mem_fun(*this, &SettingsView::choose_database)
    );

    actions_box_.append(open_button_);
    actions_box_.append(refresh_button_);
    page_content_.append(actions_box_);

    auto* info_frame = Gtk::make_managed<Gtk::Frame>();
    info_frame->add_css_class("card");

    auto* info_content = Gtk::make_managed<Gtk::Box>(
        Gtk::Orientation::VERTICAL, 10
    );
    info_content->set_margin(16);

    auto* info_title = Gtk::make_managed<Gtk::Label>(
        "Connection information"
    );
    info_title->set_xalign(0.0f);
    info_title->add_css_class("title-2");
    info_content->append(*info_title);

    database_path_label_.set_xalign(0.0f);
    database_path_label_.set_selectable(true);
    database_path_label_.set_wrap(true);
    database_path_label_.add_css_class("dim-label");
    info_content->append(database_path_label_);

    database_info_label_.set_xalign(0.0f);
    database_info_label_.set_selectable(true);
    database_info_label_.set_wrap(true);
    info_content->append(database_info_label_);

    info_frame->set_child(*info_content);
    page_content_.append(*info_frame);

    auto* schema_frame = Gtk::make_managed<Gtk::Frame>();
    schema_frame->add_css_class("card");

    auto* schema_content = Gtk::make_managed<Gtk::Box>(
        Gtk::Orientation::VERTICAL, 10
    );
    schema_content->set_margin(16);

    auto* schema_title = Gtk::make_managed<Gtk::Label>(
        "Database schema"
    );
    schema_title->set_xalign(0.0f);
    schema_title->add_css_class("title-2");
    schema_content->append(*schema_title);

    auto* schema_hint = Gtk::make_managed<Gtk::Label>(
        "Expand a table or view to inspect its details."
    );
    schema_hint->set_xalign(0.0f);
    schema_hint->add_css_class("dim-label");
    schema_hint->set_wrap(true);
    schema_content->append(*schema_hint);

    schema_box_.set_spacing(8);
    schema_scroll_.set_policy(
        Gtk::PolicyType::NEVER,
        Gtk::PolicyType::NEVER
    );
    schema_scroll_.set_child(schema_box_);
    schema_scroll_.set_hexpand(true);
    schema_content->append(schema_scroll_);

    schema_frame->set_child(*schema_content);
    page_content_.append(*schema_frame);

    status_label_.set_xalign(0.0f);
    status_label_.set_wrap(true);
    status_label_.set_selectable(true);
    status_label_.add_css_class("dim-label");
    page_content_.append(status_label_);

    refresh_database_list();
    refresh_database_info();
}

void SettingsView::refresh_database_list() {
    while (auto* child = databases_box_.get_first_child())
        databases_box_.remove(*child);

    const char* home = std::getenv("HOME");
    if (!home) {
        auto* label = Gtk::make_managed<Gtk::Label>(
            "Could not determine the home directory."
        );
        label->set_xalign(0.0f);
        databases_box_.append(*label);
        return;
    }

    const std::filesystem::path directory =
        std::filesystem::path(home) / ".gxx" / "db";

    std::error_code ec;
    if (!std::filesystem::is_directory(directory, ec)) {
        auto* label = Gtk::make_managed<Gtk::Label>(
            "Database directory not found:\n" + directory.string()
        );
        label->set_xalign(0.0f);
        label->set_wrap(true);
        label->add_css_class("dim-label");
        databases_box_.append(*label);
        return;
    }

    int count = 0;

    for (std::filesystem::directory_iterator it(directory, ec), end;
         !ec && it != end; it.increment(ec)) {
        if (!it->is_regular_file(ec))
            continue;

        const auto extension = it->path().extension().string();
        if (extension != ".db" && extension != ".gxx" &&
            extension != ".sqlite" && extension != ".sqlite3")
            continue;

        const std::string path = it->path().string();
        const std::string name = it->path().filename().string();

        const bool is_active =
            database_.is_open() && database_.path() == path;

        auto* frame = Gtk::make_managed<Gtk::Frame>();
        frame->add_css_class("card");

        auto* content = Gtk::make_managed<Gtk::Box>(
            Gtk::Orientation::VERTICAL, 8
        );
        content->set_margin(10);

        auto* header = Gtk::make_managed<Gtk::Box>(
            Gtk::Orientation::HORIZONTAL, 10
        );
        header->set_hexpand(true);

        auto* name_label = Gtk::make_managed<Gtk::Label>(name);
        name_label->set_xalign(0.0f);
        name_label->set_hexpand(true);
        name_label->set_halign(Gtk::Align::START);
        name_label->set_ellipsize(Pango::EllipsizeMode::MIDDLE);
        name_label->add_css_class("heading");
        header->append(*name_label);

        if (is_active) {
            auto* badge = Gtk::make_managed<Gtk::Label>("ACTIVE");
            badge->add_css_class("success");
            badge->add_css_class("heading");
            badge->set_halign(Gtk::Align::END);
            badge->set_margin_start(8);
            header->append(*badge);
            frame->add_css_class("accent");
        }

        auto* path_label = Gtk::make_managed<Gtk::Label>(path);
        path_label->set_xalign(0.0f);
        path_label->set_wrap(true);
        path_label->set_selectable(true);
        path_label->add_css_class("dim-label");

        auto* select_button = Gtk::make_managed<Gtk::Button>(
            is_active ? "Currently selected" : "Use this database"
        );
        select_button->set_halign(Gtk::Align::START);
        select_button->set_sensitive(!is_active);

        if (!is_active)
            select_button->add_css_class("flat");

        select_button->signal_clicked().connect([this, path]() {
            select_database(path);
        });

        content->append(*header);
        content->append(*path_label);
        content->append(*select_button);
        frame->set_child(*content);
        databases_box_.append(*frame);
        ++count;
    }

    if (ec)
        status_label_.set_text("Could not read all database files.");

    if (count == 0) {
        auto* empty = Gtk::make_managed<Gtk::Label>(
            "No database files found in:\n" + directory.string()
        );
        empty->set_xalign(0.0f);
        empty->set_wrap(true);
        empty->add_css_class("dim-label");
        databases_box_.append(*empty);
    }
}

void SettingsView::select_database(const std::string& path) {
    if (!database_.is_open() || database_.path() != path) {
        if (!database_.open(path)) {
            status_label_.set_text(
                "Could not open database: " + database_.last_error()
            );
            return;
        }

        if (!repository_.initialize() || !settings_service_.initialize()) {
            status_label_.set_text(
                "Database opened, but initialization failed."
            );
            refresh_database_list();
            refresh_database_info();
            return;
        }
    }

    refresh_database_list();
    refresh_database_info();
    status_label_.set_text("Selected database: " + path);
}

void SettingsView::clear_schema() {
    while (auto* child = schema_box_.get_first_child())
        schema_box_.remove(*child);
}

void SettingsView::refresh_database_info() {
    database_path_label_.set_text(
        database_.is_open()
            ? "Path: " + database_.path()
            : "No database connected."
    );

    if (!database_.is_open()) {
        database_info_label_.set_text("Connection: closed");
        clear_schema();
        return;
    }

    sqlite3* db = database_.get_connection();
    if (!db) {
        database_info_label_.set_text("Unable to access SQLite connection.");
        return;
    }

    sqlite3_stmt* statement = nullptr;
    if (sqlite3_prepare_v2(
            db, "SELECT sqlite_version();", -1, &statement, nullptr
        ) == SQLITE_OK &&
        sqlite3_step(statement) == SQLITE_ROW) {
        const auto* version = sqlite3_column_text(statement, 0);
        database_info_label_.set_text(
            "Connection: open\nSQLite version: " +
            std::string(version
                ? reinterpret_cast<const char*>(version)
                : "unknown")
        );
    } else {
        database_info_label_.set_text("Connection: open");
    }

    if (statement)
        sqlite3_finalize(statement);

    show_database_schema();
}

void SettingsView::show_database_schema() {
    clear_schema();

    sqlite3* db = database_.get_connection();
    if (!db)
        return;

    const char* sql = R"(
        SELECT name, type
        FROM sqlite_master
        WHERE type IN ('table', 'view')
          AND name NOT LIKE 'sqlite_%'
        ORDER BY type, name;
    )";

    sqlite3_stmt* statement = nullptr;
    if (sqlite3_prepare_v2(db, sql, -1, &statement, nullptr) != SQLITE_OK) {
        status_label_.set_text("Could not read the database schema.");
        return;
    }

    int object_count = 0;

    while (sqlite3_step(statement) == SQLITE_ROW) {
        const auto* raw_name = sqlite3_column_text(statement, 0);
        const auto* raw_type = sqlite3_column_text(statement, 1);

        if (!raw_name || !raw_type)
            continue;

        const std::string name(
            reinterpret_cast<const char*>(raw_name)
        );
        const std::string type(
            reinterpret_cast<const char*>(raw_type)
        );

        auto* expander = Gtk::make_managed<Gtk::Expander>(name);
        expander->set_hexpand(true);

        auto* details = Gtk::make_managed<Gtk::Box>(
            Gtk::Orientation::VERTICAL, 8
        );
        details->set_margin_top(10);
        details->set_margin_bottom(10);
        details->set_margin_start(12);
        details->set_margin_end(12);

        auto* type_label = Gtk::make_managed<Gtk::Label>(
            "Object type: " + type
        );
        type_label->set_xalign(0.0f);
        type_label->add_css_class("dim-label");
        details->append(*type_label);

        if (type == "table") {
            const std::string count_sql =
                "SELECT COUNT(*) FROM " + quote_identifier(name) + ";";

            sqlite3_stmt* count_statement = nullptr;

            if (sqlite3_prepare_v2(
                    db, count_sql.c_str(), -1,
                    &count_statement, nullptr
                ) == SQLITE_OK &&
                sqlite3_step(count_statement) == SQLITE_ROW) {
                auto* row_count = Gtk::make_managed<Gtk::Label>(
                    "Rows: " + std::to_string(
                        sqlite3_column_int64(count_statement, 0)
                    )
                );
                row_count->set_xalign(0.0f);
                row_count->add_css_class("heading");
                details->append(*row_count);
            }

            if (count_statement)
                sqlite3_finalize(count_statement);

            const std::string columns_sql =
                "PRAGMA table_info(" + quote_identifier(name) + ");";

            sqlite3_stmt* columns_statement = nullptr;

            if (sqlite3_prepare_v2(
                    db, columns_sql.c_str(), -1,
                    &columns_statement, nullptr
                ) == SQLITE_OK) {

                auto* columns_title = Gtk::make_managed<Gtk::Label>(
                    "Columns"
                );
                columns_title->set_xalign(0.0f);
                columns_title->add_css_class("heading");
                details->append(*columns_title);

                auto* columns_frame = Gtk::make_managed<Gtk::Frame>();
                auto* grid = Gtk::make_managed<Gtk::Grid>();
                grid->set_row_spacing(6);
                grid->set_column_spacing(16);
                grid->set_margin(10);
                grid->set_hexpand(true);

                auto add_cell = [grid](
                    const std::string& text,
                    int column,
                    int row,
                    bool heading
                ) {
                    auto* label = Gtk::make_managed<Gtk::Label>(text);
                    label->set_xalign(0.0f);
                    label->set_selectable(true);
                    label->set_wrap(true);

                    if (heading)
                        label->add_css_class("heading");

                    grid->attach(*label, column, row, 1, 1);
                };

                add_cell("Column", 0, 0, true);
                add_cell("Type", 1, 0, true);
                add_cell("Constraint", 2, 0, true);

                int row = 1;

                while (sqlite3_step(columns_statement) == SQLITE_ROW) {
                    const auto* raw_column =
                        sqlite3_column_text(columns_statement, 1);
                    const auto* raw_data_type =
                        sqlite3_column_text(columns_statement, 2);

                    const std::string column = raw_column
                        ? reinterpret_cast<const char*>(raw_column) : "";
                    const std::string data_type = raw_data_type
                        ? reinterpret_cast<const char*>(raw_data_type) : "";

                    const bool primary_key =
                        sqlite3_column_int(columns_statement, 5) != 0;

                    add_cell(column, 0, row, false);
                    add_cell(
                        data_type.empty() ? "ANY" : data_type,
                        1, row, false
                    );
                    add_cell(
                        primary_key ? "PRIMARY KEY" : "",
                        2, row, false
                    );
                    ++row;
                }

                columns_frame->set_child(*grid);
                details->append(*columns_frame);
            }

            if (columns_statement)
                sqlite3_finalize(columns_statement);
        }

        expander->set_child(*details);

        auto* frame = Gtk::make_managed<Gtk::Frame>();
        frame->add_css_class("card");
        frame->set_child(*expander);
        schema_box_.append(*frame);
        ++object_count;
    }

    sqlite3_finalize(statement);

    if (object_count == 0) {
        auto* empty = Gtk::make_managed<Gtk::Label>(
            "No tables or views found."
        );
        empty->set_xalign(0.0f);
        empty->add_css_class("dim-label");
        schema_box_.append(*empty);
    }

    status_label_.set_text(
        std::to_string(object_count) + " database object(s) found."
    );
}

void SettingsView::choose_database() {
    auto* parent = dynamic_cast<Gtk::Window*>(get_root());
    if (!parent)
        return;

    open_dialog_ = Gtk::FileChooserNative::create(
        "Open SQLite database",
        *parent,
        Gtk::FileChooser::Action::OPEN,
        "_Open",
        "_Cancel"
    );

    open_dialog_->signal_response().connect(
        sigc::mem_fun(*this, &SettingsView::on_open_response)
    );
    open_dialog_->show();
}

void SettingsView::on_open_response(int response) {
    if (response != static_cast<int>(Gtk::ResponseType::ACCEPT)) {
        open_dialog_.reset();
        return;
    }

    const auto file = open_dialog_->get_file();
    const std::string path = file ? file->get_path() : "";
    open_dialog_.reset();

    if (path.empty()) {
        status_label_.set_text("No database file was selected.");
        return;
    }

    if (!database_.open(path)) {
        status_label_.set_text(
            "Could not open database: " + database_.last_error()
        );
        return;
    }

    if (!repository_.initialize() || !settings_service_.initialize()) {
        status_label_.set_text(
            "Database opened, but GXX initialization failed: " +
            database_.last_error()
        );
        refresh_database_info();
        return;
    }

    refresh_database_info();
    status_label_.set_text("Database opened successfully.");
}

} // namespace gxx
