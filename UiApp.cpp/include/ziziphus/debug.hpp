#pragma once

#include <chrono>
#include <iostream>
#include <iomanip>
#include <ctime>
#include <utility>
#include <string>
#include <cstdlib>

namespace gxx::debug {

inline bool enabled() {
    const char* value = std::getenv("GXX_DEBUG");
    return value && std::string(value) != "0";
}

inline void log(const char* level, const std::string& message) {
    if (!enabled()) return;
    const auto now = std::chrono::system_clock::now();
    const auto time = std::chrono::system_clock::to_time_t(now);
    const auto ms = std::chrono::duration_cast<std::chrono::milliseconds>(
        now.time_since_epoch()
    ).count() % 1000;

    std::tm local_time{};
    localtime_r(&time, &local_time);

    std::cerr << '['
              << std::put_time(&local_time, "%H:%M:%S")
              << '.' << ms << "] ["
              << level << "] "
              << message << '\n';
}

class Scope {
public:
    explicit Scope(std::string name)
        : name_(std::move(name)),
          start_(std::chrono::steady_clock::now()) {
        log("ENTER", name_);
    }

    ~Scope() {
        const auto elapsed = std::chrono::duration_cast<
            std::chrono::microseconds>(
                std::chrono::steady_clock::now() - start_
            ).count();

        log("EXIT", name_ + " (" + std::to_string(elapsed) + " us)");
    }

private:
    std::string name_;
    std::chrono::steady_clock::time_point start_;
};

} // namespace gxx::debug
