
#include "gxx/application.hpp"

/*
 * main()
 *
 * Entry point of the GXX GUI application.
 *
 * Responsibilities:
 * - Create the application controller.
 * - Pass command-line arguments to GTK.
 * - Start the application event loop.
 * - Return the application's exit status.
 */
int main(int argc, char* argv[]) {
    // Always prefer the dark GTK theme.
    auto settings = Gtk::Settings::get_default();
    if (settings) {
        settings->property_gtk_application_prefer_dark_theme() = true;
    }

    // gxx_force_dark_theme

    // Create the main application controller.
    gxx::Application application;

    // Start the GTK application and return its exit status.
    return application.run(argc, argv);
}
