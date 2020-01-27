# RunningMate

A Telegram chatbot that helps you pick up running.

It's very easy to build, but setting up a usable instance is a slightly more
involved process. You need a Telegram bot, configure it to send webhooks to
your endpoint, which requires a domain and TLS certificate. You also need to
set up and train an application on Wit.ai for the natural language processing.
A working deployment of the bot for testing is available at
https://t.me/running_mate_bot until at least June 30th, 2020.

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
