# Address Lookup Table Core BPF Implementation

This PR introduces the Address Lookup Table program reimplemented to Core BPF.

Some notable points of contention with this implementation are defined below.

### Feature `relax_authority_signer_check_for_lookup_table_creation`

Feature
[`FKAcEvNgSY79RpqsPNUV5gDyumopH4cEHqUxyfm8b8Ap`](https://github.com/solana-labs/solana/issues/27205)
should be activated on mainnet-beta before Address Lookup Table is ported to
Core BPF, since this will simplify the program's interface. It's next up in the
queue.

This feature ID decorates all call sites which can be removed from the Core BPF
implementation once activated. They include:

- Instruction for creating a lookup table "signed".
- Processor checks for authority signatures on create.
- Tests ensuring authority checks are conducted.
- The Rust feature flag I'm using to mock this feature ID:
  `relax-authority-checks-disabled`.

### Error Codes

Since built-in programs throw `InstructionError` and BPF programs throw
`ProgramError`, we simply need to use the corresponding variants from
`ProgramError`, which map to the same error code. See this in action:

<https://github.com/solana-labs/solana/blob/e4064023bf7936ced97b0d4de22137742324983d/sdk/program/src/program_error.rs#L289>

However, there are a few missing variants from `ProgramError` that are present
in `InstructionError`. I've added these variants in this PR.

<https://github.com/solana-labs/solana/pull/35113>

We simply have to wait until those errors are available with the next release.

Additionally, we should consider adding `bincode` support to
`Into<ProgramError>`, especially if we plan to continue to use `bincode` within
Core BPF programs.

See my notes in [`error.rs`](./src/error.rs);

### Program-Test

`solana-program-test` will not overwrite a built-in if the BPF program you've
provided shares the same address as an existing built-in.

Technically this is not an issue with `solana-program-test` but with the
program runtime.

At the time of this writing, Alexander has created a PR to allow
`LoadedPrograms::assign_program()` to overwrite builtins, and I've created a
PR on top of his to provide the necessary functionality to
`solana-program-test`.

<https://github.com/solana-labs/solana/pull/35233>

<https://github.com/solana-labs/solana/pull/35242>

Until we can use `solana-program-test` to test Core BPF programs in place of
their built-in counterparts, we have to use a different address to test.

See [`lib.rs`](./src/lib.rs).

### `SlotHashes` Sysvar

I've replaced the slot-based indexing of lookup tables with a manual
mathematical calculation of both recent slot and deactivation cooldown.
However, this method no longer uses hashes but instead uses slots, which means
that it no longer covers the case of a skipped slot.

If it's imperative to ensure we are only considering slots where blocks were
created, then we'll likely need to provide the `SlotHashes` account to the ALT
program on `create` and `close`, so we can reliably check slot hashes.

If it's _not_ important, I'll have to re-write the state tests to avoid using
`SlotHashes` directly, to avoid confusion and provide consistency.
