//! glitu: presentations written in markdown. The library serves the `glitu` binary and any tool
//! that wants to share its build and its embedded engine (a website that publishes the engine and
//! the themes for `glitu build --web` pages, for instance).

pub mod browser;
pub mod build;
pub mod chrono;
pub mod cli;
pub mod export;
pub mod online;
#[cfg(feature = "oracle")]
pub mod oracle;
pub mod pdf;
pub mod pptx;
pub mod raster;
pub mod serve;
pub mod source;
pub mod standalone;
pub mod themes;
pub mod zip;
