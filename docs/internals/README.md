# How ripandload works

These documents explain the code, for someone who knows programming but
little Rust. The user's guide is the [README](../../README.md) at the top of
the repository.

## Read in order

1. [Architecture](architecture.md): the whole app in ten short sections,
   each linking to the pipeline that goes deeper.
2. [Pipelines](pipelines/README.md): six traces through the code, from a
   click to what the user sees.
3. [Extending the app](extending.md): the files to touch, in order, to add
   a Setting, a Source or a kind of Run.

## Look things up

- [Rust primer](rust-primer.md): the Rust this codebase uses, each feature
  with a real excerpt. The other documents link to it where a feature
  first matters.
- [Crates](crates.md): every dependency, why the app needs it, and what it
  beat.

## The API docs

The doc comments on every item, private ones included, build into
searchable API docs, with every item path the documents above mention:

```sh
cargo doc --workspace --document-private-items --open
```

There's no published copy, so the documents here don't link into it.

## Words and decisions

The words these documents capitalise, such as Source, Run and Staging
folder, are defined in the [glossary](../../GLOSSARY.md), and the reasons
behind the big decisions are in [`docs/adr/`](../adr/).
