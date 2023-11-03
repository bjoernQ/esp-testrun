# esp-testrun (WIP)

A very simple test runner based on espflash.

It just runs ELF files found in a per-chip directory.

Each test can print these output which get interpreted by the runner:
- `[PASSED]` the test passed
- `[FAILED]` the rest failed
- `[HOST cmd arg1 arg2 ...]` run the given command on the host - the exit code won't influence the test result, the command needs to exit to make the testing progress
- `[RUN chip elf]` flash the given elf file to the given chip (e.g. `esp32c6`)

Tests need to be named `test*`. The `RUN` command can flash any elf - no need to be named like `test*`

## Usage

```
Usage: esp-testrun.exe [OPTIONS]

Options:
      --esp32 <ESP32>      Path to ESP32 elf files
      --esp32s2 <ESP32S2>  Path to ESP32-S2 elf files
      --esp32s3 <ESP32S3>  Path to ESP32-S3 elf files
      --esp32c2 <ESP32C2>  Path to ESP32-C2 elf files
      --esp32c3 <ESP32C3>  Path to ESP32-C3 elf files
      --esp32c6 <ESP32C6>  Path to ESP32-C6 elf files
      --esp32h2 <ESP32H2>  Path to ESP32-H2 elf files
  -h, --help               Print help
```

Example
```
cargo run -- --esp32=target\xtensa-esp32-none-elf\debug\examples\ --esp32c6=target\riscv32imac-unknown-none-elf\debug\examples\
```
