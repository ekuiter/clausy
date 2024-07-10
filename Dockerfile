FROM ubuntu:22.04
RUN apt-get update && apt install -y \
    build-essential \
    zlib1g-dev \
    default-jre \
    curl \
    cmake \
    libgmp-dev \
    libmpfr-dev \
    libboost-all-dev \
    ninja-build \
    python3 \
    && curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
WORKDIR /home
COPY . ./
RUN make
ENTRYPOINT [ "bin/clausy" ]