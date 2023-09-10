FROM ubuntu:22.04
ENV DEBIAN_FRONTEND noninteractive
RUN apt-get update && apt-get install -y git curl build-essential python3 python3-venv imagemagick
RUN mv /etc/ImageMagick-6/policy.xml /etc/ImageMagick-6/policy.xml.off
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
WORKDIR /types3
COPY . .
CMD [ "util/test.sh" ]
