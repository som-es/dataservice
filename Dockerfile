# syntax=docker/dockerfile:1.6
FROM rust:1.93 AS builder

RUN apt-get update && apt-get install -y \
    libiconv-hook-dev \
    libtool \
    libxml2-dev \
    libxslt-dev \
    libclang-dev \
    libssl-dev \
    pkg-config \
    xmlsec1 \
    libxml2-dev libxmlsec1-dev libxmlsec1-openssl \
    postgresql-client \
    # postgresql \
    libpq-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY ./ ./

RUN mkdir -p /usr/share/tessdata && \
    curl -L -o /usr/share/tessdata/deu.traineddata \
    https://github.com/tesseract-ocr/tessdata_best/raw/main/deu.traineddata

RUN --mount=type=ssh \
    cargo fetch

ENV TESSDATA_DIR=/usr/share/tessdata
ENV DISABLE_SPEECH_TEXT=false
ENV MIND_UPDATE_TIME=true
ENV ENFORCE_AI_RUN=true

RUN --mount=type=ssh cargo update
RUN --mount=type=ssh cargo build --release
#CMD cargo build --release && ./target/release/dataservice/
CMD ["./target/release/dataservice"]
