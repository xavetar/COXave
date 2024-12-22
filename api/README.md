![Greenfield](https://img.shields.io/badge/Greenfield-0fc908.svg)
[![Deps](https://deps.rs/repo/github/xavetar/COXave/status.svg)](https://deps.rs/repo/github/xavetar/COXave)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

# COXave

![COXave Logo](res/coxave-header.png)

<div style="display: flex; justify-content: center; gap: 20px;">
    <a href="https://nowpayments.io/donation?api_key=NRH28QG-ABRM7CC-J7NVGXN-F8FTRS1&source=lk_donation&medium=referral" target="_blank">
        <img src="https://nowpayments.io/images/embeds/donation-button-black.svg" alt="Crypto donation button by NOWPayments" style="height: 60px !important; width: 217px !important;">
    </a>
</div>

## About

The library is hosted on [crates.io](https://crates.io/crates/COXave/).

## Add library

CLI:

```shell
cargo add COXave
```

Cargo.toml:

```toml
[dependencies]
COXave = { version = "*" }
```

## Usage with Python

```shell
maturin build -m api/Cargo.toml --release --features python && pip install --force-reinstall target/wheels/COXave-*.whl
```

## License

COXave is primarily distributed under the terms of three the Anti-Virus license and MIT license and the Apache License (Version 2.0)

See [LICENSE-ANTI-VIRUS](LICENSE) and [LICENSE-APACHE](LICENSE) and [LICENSE-MIT](LICENSE) for details.