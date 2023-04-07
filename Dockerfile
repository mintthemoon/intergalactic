FROM rust:1.68-buster AS build
COPY . .
RUN cargo build --release
WORKDIR /dist
RUN mkdir lib \
    && mv $(ldd /target/release/intergalactic | grep libgcc_s.so.1 | awk '{print $3}') lib/


FROM gcr.io/distroless/base-debian11:latest
COPY --from=build /target/release/intergalactic /usr/local/bin/intergalactic
COPY --from=build /dist/lib/* /usr/lib/
ENTRYPOINT ["intergalactic"]
