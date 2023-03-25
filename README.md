# havoc

<img src="./docs/spectacles_67d10e6_uncollapsed.jpg" align="right" width=385>

havoc is a [Discord] client instrumentation toolkit written in [Rust] that aims
to be robust, correct, and efficient. [watchdog] is a persistent daemon-like
program that leverages havoc to monitor for new builds, exposes a fluent HTTP
API, and performs code and asset diffing between builds. [spectacles] is a
frontend that consumes this information, collating and presenting it in a
beautiful and easy-to-understand way.

havoc and friends are intended to be useful tools to client modders and curious
onlookers alike. User friendliness and experience is an overall priority.

[rust]: https://www.rust-lang.org
[watchdog]: /crates/watchdog
[spectacles]: /spectacles
[discord]: https://discord.com

<br clear="both">

## Usage

We currently don't provide pre-built binaries. To get started, make sure you
have a functional [Rust] toolchain on your machine (try [rustup]). Clone the
repository and try out these commands in a shell:

[rustup]: https://rustup.rs

```sh
# Scrape the latest Canary build and output basic information (such as the
# build number, ID, and assets) to stdout.
$ cargo run --bin havoc -- scrape fe:canary

# Scrape the latest Canary build, parsing and dumping all Webpack modules'
# source code into a JSON file in the current directory, keyed by module ID.
$ cargo run --bin havoc -- scrape fe:canary --dump modules
```

## License

havoc and related projects are distributed under the MIT License. See the
[LICENSE](LICENSE) file for more details.
