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
COPY lib lib/
COPY Makefile .
RUN chmod +x lib/gradle_proxy.sh \
    && lib/gradle_proxy.sh
RUN make lib
COPY . ./
RUN make clausy
ENTRYPOINT [ "bin/clausy" ]