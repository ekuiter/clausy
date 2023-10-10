FROM ubuntu:22.04
RUN apt-get update && apt install -y \
    build-essential \
    zlib1g-dev \
    default-jre \
    curl \
    && curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
WORKDIR /home
COPY . ./
RUN ./build.sh
ENTRYPOINT [ "bin/clausy" ]