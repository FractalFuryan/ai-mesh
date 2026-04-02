# Contributing to ai-mesh

Thanks for contributing.

## Development workflow

1. Open an issue for substantial changes before implementation.
2. Fork and create a topic branch.
3. Add tests for behavior changes.
4. Run local checks before opening a PR.

## Local quality checks

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Pull request checklist

- Clear title and description.
- Linked issue when applicable.
- Passing CI.
- Backward-compatibility and migration notes when relevant.
