cmake_minimum_required(VERSION 3.10)
project(octomap_benchmark)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Platform-specific optimization flags
if(MSVC)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /O2 /arch:AVX2")
else()
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -O3 -march=native")
endif()

find_package(octomap REQUIRED)

include_directories(${OCTOMAP_INCLUDE_DIRS})

add_executable(benchmark_octomap benchmark_octomap.cpp)
target_link_libraries(benchmark_octomap ${OCTOMAP_LIBRARIES})
