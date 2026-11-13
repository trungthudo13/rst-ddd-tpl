# Rust with DDD template

A general-purpose Rust service template organized with Domain-Driven Design
(DDD) and hexagonal boundaries. It combines **Axum** for HTTP APIs, **Tokio** for
the async runtime, **Tonic** for gRPC, and **HiQLite** for demo persistence.

The Item CRUD example shows how to separate domain rules, application use cases,
infrastructure adapters, and presentation contracts. Replace or extend Item with
your own business modules and services.

See [Architecture](ARCHITECTURE.md) for module responsibilities, separation
rationale, and instructions to run and adapt the template.

Tag pushes publish the runnable demo and a reusable runtime base to GitHub
Container Registry. See [image publishing and reuse](ARCHITECTURE.md#publishing-and-reusing-images)
for release commands and downstream Dockerfiles.
