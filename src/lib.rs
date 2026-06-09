//! Generative Artifact Protocol (GAP): token-efficient artifact generation and
//! updates for LLMs. This crate is the reference implementation of the stateless
//! apply engine, plus a versioned artifact store and C FFI bindings.
//!
//! The core is a single pure function, [`apply::apply`], which resolves an
//! envelope against the current artifact state:
//!
//! ```
//! use gap::gap::{Envelope, Name};
//! use gap::apply;
//!
//! let envelope = Envelope::from_json(r#"{
//!     "protocol": "gap/0.1",
//!     "id": "greeting",
//!     "version": 1,
//!     "name": "synthesize",
//!     "meta": {"format": "text/html"},
//!     "content": [{"body": "<gap:target id=\"msg\">hello</gap:target>"}]
//! }"#)?;
//!
//! let (artifact, handle) = apply::apply(None, &envelope)?;
//! assert!(artifact.body.contains("hello"));
//! assert_eq!(handle.name, Name::Handle);
//! # anyhow::Ok(())
//! ```
//!
//! See the [protocol specification](https://github.com/urmzd/generative-artifact-protocol/blob/main/spec/gap.md)
//! for the wire format and conformance levels.

pub mod apply;
pub mod cffi;
pub mod gap;
pub mod markers;
pub mod store;
