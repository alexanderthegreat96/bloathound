# BloatHound
BloadHound is a CLI utility aimed to solve a specific problem that developers and users might encounter. When your hard-drive gets full, it can get rather hectic when it comes to figuring out what to actually delete. This tool solves that exact problem.

## How does it work?
You run the tool, select a drive, the scanner starts and it will attempt to scan everything recursively. By default it stores 100 topmost large files. Once the scan is complete, you can select which files to wipe.

## Building and Running
```bash
cargo build --release
./bloathound
```