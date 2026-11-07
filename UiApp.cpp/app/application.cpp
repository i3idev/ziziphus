
#include <gtkmm.h>

#include <memory>

#include "ziziphus/application.hpp"
#include "ziziphus/main_window.hpp"



namespace ziziphus {

/*
 * Application
 *
 * Main entry point for the GXX graphical application.
 *
 * Responsibilities:
 * - Initialize the GTK application.
 * - Configure the application identifier.
 * - Create the main application window.
 * - Present the window when the application starts.
 * - Let GTK manage the application lifecycle.
 */
Application::Application()
    : app_(Gtk::Application::create(
          "dev.i3idev.ziziphus"
      )) {
}

/*
 * run()
 *
 * Starts the GTK application.
 *
 * The activate signal is emitted when the application
 * should display its primary interface.
 *
 * GTK manages the event loop and application lifecycle.
 *
 * Parameters:
 * argc - Number of command-line arguments.
 * argv - Command-line argument values.
 *
 * Returns:
 * The application's exit status.
 */
int Application::run(int argc, char* argv[]) {
    // Create the main window when the application activates.
    app_->signal_activate().connect([this]() {
        // Avoid creating duplicate windows on repeated activation.
        if (!window_) {
            window_ = std::make_unique<MainWindow>();
            app_->add_window(*window_);
        }

        // Display the main application window.
        window_->present();
    });

    // Start the GTK event loop.
    return app_->run(argc, argv);
}

} // namespace gxx

// Define the destructor after MainWindow is fully declared.
gxx::Application::~Application() = default;
