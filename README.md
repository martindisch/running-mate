# RunningMate

A Telegram chatbot that helps you pick up running.

## Getting started
1. Install Docker & Docker Compose
2. Create and edit `.env`. For reference, see `.env.TEMPLATE`.

## Run
To get everything up and running, use `docker-compose up`. If you change some
source files, do `docker-compose up --build` to rebuild the image with your
changes. To remove all traces of it from your system, do `docker-compose down
-v --rmi all`.

## TLS
The whole application is TLS-agnostic to make containerized deployments easy.
TLS is handled by a reverse proxy like nginx, see the files in `nginx` for an
example.

## License
Licensed under either of

 * [Apache License, Version 2.0](LICENSE-APACHE)
 * [MIT license](LICENSE-MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
