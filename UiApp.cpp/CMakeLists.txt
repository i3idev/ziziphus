
cmake_minimum_required(VERSION 3.20)

project(gxx_ui
    VERSION 0.1.0
    DESCRIPTION "GXX GTKmm 4 graphical application"
    LANGUAGES CXX
)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(PkgConfig REQUIRED)
pkg_check_modules(GTKMM REQUIRED IMPORTED_TARGET gtkmm-4.0)
find_package(SQLite3 REQUIRED)

add_executable(gxx-ui
    main.cpp
    app/application.cpp
    database/database.cpp
    database/word_repository.cpp
    services/settings_service.cpp
    views/main_window.cpp
    views/search_view.cpp
    views/dictionary_view.cpp
    views/word_editor.cpp
    views/settings_view.cpp
    views/about_view.cpp
)

target_include_directories(gxx-ui
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_link_libraries(gxx-ui
    PRIVATE
        PkgConfig::GTKMM
        SQLite::SQLite3
)

target_compile_options(gxx-ui
    PRIVATE
        -Wall
        -Wextra
        -Wpedantic
)

install(TARGETS gxx-ui
    RUNTIME DESTINATION bin
)
