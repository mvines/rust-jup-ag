# jup-ag
[Jupiter Aggregator](https://jup.ag/) API bindings for Rust.

Basic usage examples can be found in the [examples](examples) directory.

## Usage
* Crates.io: https://crates.io/crates/jup-ag
* API Documentation: https://docs.rs/jup-ag/
* Jupiter Swap API Documentation: https://station.jup.ag/docs/v6/swap-api
* jup.ag Website: https://jup.ag/

## Examples

To run the examples:
```sh
$ cargo run --example <EXAMPLE_NAME>
```

### Using Self-hosted APIs

You can set custom API endpoints via environment variables to use any self-hosted Jupiter APIs. Like the [self-hosted V6 Swap API](https://station.jup.ag/docs/apis/self-hosted) or [paid hosted APIs](https://station.jup.ag/docs/apis/self-hosted#paid-hosted-apis). Here are the ENV vars:

```
QUOTE_API_URL=https://hosted.api
PRICE_API_URL=https://price.jup.ag/v1
```
