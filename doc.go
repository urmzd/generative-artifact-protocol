// Package gap implements the core Generative Artifact Protocol apply engine.
//
// The package is intentionally small: protocol structs, marker resolution,
// a stateless Apply function, and a versioned in-memory ArtifactStore. It does
// no network I/O and keeps protocol resolution deterministic.
package gap
