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
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
WORKDIR /home
COPY scripts scripts/
COPY src/external src/external/
RUN chmod +x scripts/gradle_proxy.sh \
    && scripts/gradle_proxy.sh
RUN make -C src/external
COPY . ./
RUN make
ENTRYPOINT [ "build/clausy" ]