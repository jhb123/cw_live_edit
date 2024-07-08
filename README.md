# Overview

This is a server for letting people concurrently solve a crossword based on WebSockets. It includes a JavaScript frontend; the repo for the Android app is [here](https://github.com/jhb123/crossword-scan-app). See the project's [about page](https://www.live-crossword.net/about) for an overview of this project.
## Development
These two commands let you develop the server with hot-reloading.
```
RUST_LOG=info cargo watch -x run -i static

tailwindcss -i ./static/input.css -o ./static/styles.css --watch
```
## Other binaries
When puzzles are deleted via the API, they are soft-deleted. To delete them forever or restore them, the prune program can be used. Build this binary with `cargo build --bin prune`. Note, this needs to be built into the docker image.

There is an Websocket echo server that can be built with `cargo build --bin echo`.