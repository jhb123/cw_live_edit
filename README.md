## Build
```
docker build . --tag=cw-grid-server
```

## Run
If you have no grace and decorum
```
docker run -p 5051:5051 -v ./puzzles:/puzzles --init cw-grid-server
```
otherwise, omit `--init`.
## Development
```
cargo watch -i static
```
Install tailwindcss and
```
tailwindcss -i ./static/input.css -o ./static/styles.css --watch
```
## Routing a client to a puzzle
![Connection flow](https://github.com/jhb123/cw_live_edit/blob/puzzle-persistance/connection_flow.png?raw=true)
