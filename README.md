<div align="center">
  <br>
  <a href="https://crates.io/crates/fluxio">
      <img src="logo.svg" width="150">
  </a>
  <br><br>
  [<img alt="github" src="https://img.shields.io/badge/github-dr%20montasir%20/%20fluxio-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="22">](https://github.com/dr-montasir/fluxio)[<img alt="crates.io" src="https://img.shields.io/crates/v/fluxio.svg?style=for-the-badge&color=fc8d62&logo=rust" height="22">](https://crates.io/crates/fluxio)[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-fluxio-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="22">](https://docs.rs/fluxio)[<img alt="license" src="https://img.shields.io/badge/license-apache_2.0-4a98f7.svg?style=for-the-badge&labelColor=555555&logo=apache" height="22">](https://choosealicense.com/licenses/apache-2.0)

  <h1>FLUXIO</h1>
</div>

Fluxio is a wrapper around `hyper` version 0.14.19, designed specifically for use within the Fluxor project. This version (0.1.0) serves as a starting point and has been modified to fit the specific needs of Fluxor projects. Future versions may include further enhancements and modifications.

## About

Fluxio is built on top of `hyper` 0.14.19, a powerful HTTP implementation in Rust. By using Fluxio, developers can leverage the foundational capabilities provided by `hyper` while integrating them seamlessly into the Fluxor ecosystem.

## Modifications

Fluxio has been modified from the original `hyper` source code, specifically changing references from `hyper` to `fluxio` throughout the codebase. This helps in clearly distinguishing it as a unique component tailored for the Fluxor project.

## Future Developments

As a starter version (0.1.0), Fluxio is expected to evolve with subsequent updates. Future versions will likely include additional functionality and optimizations tailored to the needs of the Fluxor project.

## Contributions

Contributions to Fluxio are welcome! Please ensure that any contributions adhere to the original licensing terms of `hyper`.

## License

This project is based on `hyper` 0.14.19, which is licensed under the following terms:

```text
Copyright (c) 2014-2021 Sean McArthur

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
```

## Acknowledgments

Special thanks to Sean McArthur for the original `hyper` implementation, which serves as the foundation for Fluxio.

## Contact

If you have any questions or feedback, feel free to reach out:

- [dr-montasir](https://crates.io/users/dr-montasir)
- [GitHub Repository](https://github.com/dr-montasir/fluxio)
