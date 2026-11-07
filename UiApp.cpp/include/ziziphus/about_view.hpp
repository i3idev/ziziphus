#pragma once

#include <gtkmm.h>

namespace gxx {

class AboutView : public Gtk::Box {
public:
    AboutView();

private:
    Gtk::Box content_{Gtk::Orientation::VERTICAL, 12};
    Gtk::ScrolledWindow scroll_;

    void add_section(const Glib::ustring& title,
                     const Glib::ustring& description);
};

} // namespace gxx
