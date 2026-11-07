
#pragma once

#include <gtkmm.h>
#include <memory>

namespace ziziphus {

// Forward declaration avoids including the complete window header here.
class MainWindow;

/*
 * Application
 *
 * Controls the lifecycle of the GXX graphical application.
 *
 * Responsibilities:
 * - Hold the GTK application instance.
 * - Create and manage the main window.
 * - Start the GTK event loop.
 *
 * The implementation is defined in application.cpp.
 */
class Application {
public:
    // Create the GTK application instance.
    Application();

    // Release resources after the application finishes.
    ~Application();

    // Prevent copying because the application owns its main window.
    Application(const Application&) = delete;
    Application& operator=(const Application&) = delete;

    // Prevent moving to keep the ownership model straightforward.
    Application(Application&&) = delete;
    Application& operator=(Application&&) = delete;

    /*
     * Start the application event loop.
     *
     * argc and argv contain the command-line arguments.
     * Returns the application's exit status.
     */
    int run(int argc, char* argv[]);

private:
    // GTK application object responsible for application lifecycle.
    Glib::RefPtr<Gtk::Application> app_;

    // Owns the main window for the lifetime of this controller.
    std::unique_ptr<MainWindow> window_;
};

} // namespace ziziphus
