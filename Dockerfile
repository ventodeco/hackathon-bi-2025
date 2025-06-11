FROM scratch

COPY /target/x86_64-unknown-linux-musl/release/backend-project /backend-project

RUN chmod +x /backend-project

EXPOSE 8080

CMD ["/backend-project"]
