ARG DEBIAN_HASH
FROM debian@sha256:${DEBIAN_HASH}

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    TZ=UTC \
    HOME=/home/build \
    PATH=/scripts:/usr/sbin:/usr/bin:/sbin:/bin

ARG CONFIG_DIR
ADD ${CONFIG_DIR} /config

ARG SCRIPTS_DIR
ADD ${SCRIPTS_DIR} /scripts

RUN packages-install
