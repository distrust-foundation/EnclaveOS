ARG DEBIAN_HASH
FROM debian@sha256:${DEBIAN_HASH}

RUN apt update && \
    apt install -y \
        git \
        curl \
        build-essential \
        flex \
        bison \
        libncurses-dev \
        bc \
        libelf-dev \
        libarchive-tools \
        libssl-dev \
        fakeroot \
        cpio
