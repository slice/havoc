# havoc

havoc is a [Discord] client instrumentation toolkit written in Rust that aims to
be robust, correct, and efficient.

[discord]: https://discord.com

## Usage

Binaries are currently unavailable. To get started, make sure you have a
functional [Rust] toolchain on your machine (try [rustup]). Clone the repository
and try out these commands in a terminal:

[rust]: https://www.rust-lang.org
[rustup]: https://rustup.rs

```sh
# Scrape the latest Canary build and output basic information (such as the version number) to stdout.
$ cargo run -- scrape fe:canary

# Scrape the latest Canary build and dump build information into a JSON file in the current directory.
$ cargo run -- scrape fe:canary --dump self
```

## License

havoc is distributed under the MIT License. See [LICENSE](LICENSE) for details.
