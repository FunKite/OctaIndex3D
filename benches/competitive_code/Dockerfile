# OctoMap Benchmark Container
# Ubuntu 22.04 + ROS2 Humble + OctoMap
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive
ENV ROS_DISTRO=humble

# Install basic tools
RUN apt-get update && apt-get install -y \
    curl \
    gnupg \
    lsb-release \
    software-properties-common \
    build-essential \
    cmake \
    git \
    && rm -rf /var/lib/apt/lists/*

# Add ROS2 repository
RUN curl -sSL https://raw.githubusercontent.com/ros/rosdistro/master/ros.key -o /usr/share/keyrings/ros-archive-keyring.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/ros-archive-keyring.gpg] http://packages.ros.org/ros2/ubuntu $(. /etc/os-release && echo $UBUNTU_CODENAME) main" | tee /etc/apt/sources.list.d/ros2.list > /dev/null

# Install OctoMap (without full ROS2 to reduce image size)
RUN apt-get update && apt-get install -y \
    liboctomap-dev \
    octomap-tools \
    && rm -rf /var/lib/apt/lists/*

# Set up workspace
WORKDIR /workspace

# Copy benchmark code
COPY benchmark_octomap.cpp /workspace/
COPY CMakeLists.txt /workspace/

# Build benchmark
RUN cmake . && make

CMD ["/workspace/benchmark_octomap"]
