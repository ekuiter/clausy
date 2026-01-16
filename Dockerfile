FROM ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt install -y \
    build-essential \
    zlib1g-dev \
    default-jre \
    curl \
    libgmp-dev \
    libmpfr-dev \
    libboost-all-dev \
    ninja-build \
    python3 \
    libssl-dev \
    cmake \
    wget
# we could install cmake from source because the version in the Ubuntu repository (3.28) is outdated
# however for our dependencies it suffices to have old cmake
# ARG CMAKE_VERSION=4.2.1
# RUN wget -q https://github.com/Kitware/CMake/releases/download/v${CMAKE_VERSION}/cmake-${CMAKE_VERSION}.tar.gz \
#     && tar -xzf cmake-${CMAKE_VERSION}.tar.gz \
#     && cd cmake-${CMAKE_VERSION} \
#     && ./bootstrap --prefix=/opt/cmake \
#     && make -j$(nproc) \
#     && make install \
#     && cd .. \
#     && rm -rf cmake-${CMAKE_VERSION} cmake-${CMAKE_VERSION}.tar.gz
# ENV PATH="/opt/cmake/bin:${PATH}"
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
WORKDIR /home
COPY scripts scripts/
COPY src/external src/external/
RUN chmod +x scripts/gradle_proxy.sh \
    && scripts/gradle_proxy.sh
RUN make -C src/external -j$(nproc)
COPY . ./
# RUN make clausy
# ENTRYPOINT [ "bin/clausy" ]