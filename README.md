<div align="center">
  <br>
  <a href="https://crates.io/crates/fluxio">
      <img src="logo.svg" width="150">
  </a>
  <br><br>

[<img alt="github" src="https://img.shields.io/badge/github-dr%20montasir%20/%20fluxio-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="22">](https://github.com/dr-montasir/fluxio)[<img alt="crates.io" src="https://img.shields.io/crates/v/fluxio.svg?style=for-the-badge&color=fc8d62&logo=rust" height="22">](https://crates.io/crates/fluxio)[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-fluxio-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="22">](https://docs.rs/fluxio)

  <h1>FLUXIO: Sunset (v0.5.3)</h1>
</div>

## Project Status: Sunset
Fluxio version 0.5.3 is the final release of this crate. 

The Fluxor ecosystem is transitioning away from internal hyper and tokio dependencies. Networking and runtime foundations are migrated to a specialized architecture.

## Future Development: Webio & Crator
Functionality previously handled by Fluxio is now integrated into the following crates:

*   **[Fluxor](https://crates.io)**: The main ecosystem engine.
*   **[Webio](https://crates.io)**: The specialized web utility layer.
*   **[Crator](https://crates.io)**: The core runner and hardware-abstraction interface.

## Rationale
Fluxio served as a modified wrapper around hyper 0.14. Higher performance and tighter integration within the Fluxor stack require a transition to Webio and Crator. This shift moves development beyond generic wrappers toward tools optimized for specific ecosystem goals.

---

## Legacy License
This project was based on hyper 0.14.19. Previous versions remain available on Crates.io under the original MIT license terms provided by Sean McArthur.

**Contact:** [dr-montasir](https://crates.io/users/dr-montasir)
