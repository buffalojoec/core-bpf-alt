# Address Lookup Table Core BPF Implementation

This PR introduces the Address Lookup Table program reimplemented to Core BPF.

All Core BPF implementation notes prefixed with:

```
[Core BPF]:
```

- `SlotHashes` sysvar replaced with `Clock`.
- `build.rs` and annotations in `lib.rs` are required for `solana-frozen-abi-macro`.
- `InstructionError::Immutable` has no `ProgramError` counterpart ([#35113](https://github.com/solana-labs/solana/pull/35113)).
- `InstructionError::IncorrectAuthority` has no `ProgramError` counterpart ([#35113](https://github.com/solana-labs/solana/pull/35113)).
- `solana-program-test` will not overwrite a built-in if the BPF program you've
  provided shares the same address as an existing built-in ([#35242](https://github.com/solana-labs/solana/pull/35242)).
