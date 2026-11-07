#include "gxx/about_view.hpp"

#include <cstdlib>
#include <unistd.h>
#include <filesystem>
#include <string>
#include <vector>
#include <sstream>
#include <gtkmm.h>
#include <sqlite3.h>

namespace gxx {
namespace {

bool command_exists(const std::string& command) {
    const char* raw_path = std::getenv("PATH");
    if (!raw_path) return false;

    std::string path(raw_path);
    std::size_t start = 0;

    while (start <= path.size()) {
        const auto end = path.find(':', start);
        const auto dir = path.substr(
            start, end == std::string::npos ? end : end - start
        );

        const auto candidate =
            std::filesystem::path(dir.empty() ? "." : dir) / command;

        if (std::filesystem::exists(candidate) &&
            access(candidate.c_str(), X_OK) == 0) {
            return true;
        }

        if (end == std::string::npos) break;
        start = end + 1;
    }

    return false;
}

Gtk::Widget* dependency_card(const std::string& name,
                             const std::string& purpose,
                             bool installed) {
    auto* frame = Gtk::make_managed<Gtk::Frame>();
    frame->add_css_class("card");

    auto* row = Gtk::make_managed<Gtk::Box>(
        Gtk::Orientation::HORIZONTAL, 12
    );
    row->set_margin(12);

    auto* details = Gtk::make_managed<Gtk::Box>(
        Gtk::Orientation::VERTICAL, 4
    );

    auto* title = Gtk::make_managed<Gtk::Label>(name);
    title->set_xalign(0.0f);
    title->add_css_class("heading");

    auto* description = Gtk::make_managed<Gtk::Label>(purpose);
    description->set_xalign(0.0f);
    description->set_wrap(true);
    description->set_hexpand(true);

    auto* status_box = Gtk::make_managed<Gtk::Box>(
        Gtk::Orientation::HORIZONTAL, 6
    );

    auto* status_icon = Gtk::make_managed<Gtk::Label>(
        installed ? "[+]" : "[-]"
    );

    auto* status = Gtk::make_managed<Gtk::Label>(
        installed ? "FOUND" : "NOT FOUND"
    );
    status->add_css_class(installed ? "success" : "error");

    status_box->append(*status_icon);
    status_box->append(*status);
    status_box->set_valign(Gtk::Align::CENTER);

    details->append(*title);
    details->append(*description);
    details->set_hexpand(true);

    row->append(*details);
    row->append(*status_box);
    frame->set_child(*row);

    return frame;
}

} // namespace

AboutView::AboutView()
    : Gtk::Box(Gtk::Orientation::VERTICAL) {
    auto* repository_button =
        Gtk::make_managed<Gtk::Button>("GitHub Repository");

    repository_button->set_hexpand(true);
    repository_button->set_size_request(-1, 52);
    repository_button->signal_clicked().connect([] {
        try {
            Gio::AppInfo::launch_default_for_uri(
                "https://github.com/i3idev/gxx"
            );
        } catch (const Glib::Error& error) {
            g_warning("Could not open repository: %s",
                      error.what());
        }
    });

    content_.append(*repository_button);

    scroll_.set_policy(
        Gtk::PolicyType::NEVER, Gtk::PolicyType::AUTOMATIC
    );
    scroll_.set_child(content_);
    scroll_.set_hexpand(true);
    scroll_.set_vexpand(true);
    set_hexpand(true);
    set_vexpand(true);
    append(scroll_);

    content_.set_margin(24);
    content_.set_spacing(16);

    auto* title = Gtk::make_managed<Gtk::Label>("About gxx");
    title->set_xalign(0.0f);
    title->add_css_class("title-1");
    content_.append(*title);

    auto* subtitle = Gtk::make_managed<Gtk::Label>(
        "Application information, dependencies and development tools."
    );
    subtitle->set_xalign(0.0f);
    subtitle->set_wrap(true);
    content_.append(*subtitle);

    add_section(
        "Application",
        "gxx is a vocabulary and language-learning application. "
        "This page shows available tools; FOUND does not guarantee "
        "that every application feature is operational."
    );

    auto* runtime = Gtk::make_managed<Gtk::Frame>("Runtime & Development");
    runtime->add_css_class("card");

    auto* runtime_box = Gtk::make_managed<Gtk::Box>(
        Gtk::Orientation::VERTICAL, 8
    );
    runtime_box->set_margin(12);

    runtime_box->append(*dependency_card(
        "GTKmm 4", "C++ interface for the GTK graphical toolkit.", true
    ));
    runtime_box->append(*dependency_card(
        "SQLite", "Database engine used by gxx.", true
    ));

    runtime->set_child(*runtime_box);
    content_.append(*runtime);

    auto* tools = Gtk::make_managed<Gtk::Frame>("Developer Tools");
    tools->add_css_class("card");

    auto* tools_box = Gtk::make_managed<Gtk::Box>(
        Gtk::Orientation::VERTICAL, 8
    );
    tools_box->set_margin(12);

    tools_box->append(*dependency_card(
        "C++ compiler", "Required to compile the application.",
        command_exists("c++") || command_exists("g++") ||
        command_exists("clang++")
    ));
    tools_box->append(*dependency_card(
        "CMake", "Configures the project build system.", command_exists("cmake")
    ));
    tools_box->append(*dependency_card(
        "pkg-config", "Helps locate installed development libraries.",
        command_exists("pkg-config")
    ));

    tools->set_child(*tools_box);
    content_.append(*tools);

    add_section(
        "Build dependencies",
        "On Debian-based systems, the common development packages are "
        "build-essential, cmake, pkg-config, libgtkmm-4.0-dev and "
        "libsqlite3-dev. Install only the packages required by your "
        "project configuration."
    );


}

void AboutView::add_section(
    const Glib::ustring& title,
    const Glib::ustring& description
) {
    auto* section = Gtk::make_managed<Gtk::Box>(
        Gtk::Orientation::VERTICAL, 0
    );
    section->add_css_class("card");

    auto* button = Gtk::make_managed<Gtk::Button>();
    button->add_css_class("flat");
    button->set_hexpand(true);

    auto* header = Gtk::make_managed<Gtk::Box>(
        Gtk::Orientation::HORIZONTAL, 10
    );
    header->set_margin(12);

    auto* heading = Gtk::make_managed<Gtk::Label>(title);
    heading->set_xalign(0.0f);
    heading->add_css_class("heading");
    heading->set_hexpand(true);

    auto* arrow = Gtk::make_managed<Gtk::Label>("+");

    header->append(*heading);
    header->append(*arrow);
    button->set_child(*header);

    auto* body = Gtk::make_managed<Gtk::Revealer>();
    auto* text = Gtk::make_managed<Gtk::Label>(description);
    text->set_xalign(0.0f);
    text->set_wrap(true);
    text->set_selectable(true);
    text->set_margin(12);

    body->set_child(*text);
    body->set_reveal_child(false);

    button->signal_clicked().connect(
        [body, arrow, expanded = false]() mutable {
            expanded = !expanded;
            body->set_reveal_child(expanded);
            arrow->set_text(expanded ? "-" : "+");
        }
    );

    section->append(*button);
    section->append(*body);
    content_.append(*section);
}

} // namespace gxx
