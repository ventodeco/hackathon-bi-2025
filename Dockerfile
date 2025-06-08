FROM scratch

COPY /target/release/backend-project /

EXPOSE 8080

CMD ["backend-project"]